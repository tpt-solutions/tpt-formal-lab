//! Record and replay: a hash-chained log of simulation steps.
//!
//! Each step of a logged run appends an entry holding the step count, the
//! [`state_hash`](DeterministicSim::state_hash) after the step, and a chain
//! value that folds in every earlier entry. Two runs that agree on every entry
//! have equal chains, and the chains of two logs diverge from the first
//! differing step on. That is what makes the first divergence findable by
//! bisection in `O(log n)` comparisons.

use core::hash::Hash;
use std::fmt;

use crate::sim::DeterministicSim;

/// FNV-1a offset basis, also the chain value before the first entry.
const GENESIS: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0100_0000_01b3;

/// One logged step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogEntry {
    /// The simulation's step count after the step ran.
    pub step: u64,
    /// The state hash after the step ran.
    pub state_hash: u64,
    /// A hash of this entry and every entry before it.
    pub chain: u64,
}

/// A hash-chained log of simulation steps.
///
/// # Example
///
/// ```rust
/// use tpt_fl_deterministic_sim::{DeterministicSim, StepLog};
///
/// let mut sim = DeterministicSim::<u64>::new();
/// sim.spawn(1);
/// let mut log = StepLog::new();
/// sim.step_n_logged(3, &mut log);
/// assert_eq!(log.len(), 3);
///
/// // A fresh run with the same setup reproduces the log exactly.
/// let mut again = DeterministicSim::<u64>::new();
/// again.spawn(1);
/// assert_eq!(again.verify_log(&log), Ok(()));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StepLog {
    entries: Vec<LogEntry>,
}

/// The first step at which a replay stopped matching its log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Divergence {
    /// Index of the mismatching entry in the log.
    pub index: usize,
    /// The state hash that the log recorded.
    pub expected: u64,
    /// The state hash the replay produced.
    pub actual: u64,
}

impl fmt::Display for Divergence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "replay diverged at entry {}: expected state hash {:#018x}, got {:#018x}",
            self.index, self.expected, self.actual
        )
    }
}

impl StepLog {
    /// Creates an empty log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the logged entries in step order.
    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }

    /// Returns the number of logged steps.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if no step has been logged.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the chain value of the last entry, or the genesis value for an
    /// empty log.
    pub fn head(&self) -> u64 {
        self.entries.last().map_or(GENESIS, |e| e.chain)
    }

    fn push(&mut self, step: u64, state_hash: u64) {
        let chain = fold(fold(self.head(), step), state_hash);
        self.entries.push(LogEntry {
            step,
            state_hash,
            chain,
        });
    }

    /// Returns the index of the first entry at which `self` and `other` differ,
    /// or `None` if they agree.
    ///
    /// If one log is a strict prefix of the other, the index is the length of
    /// the shorter log. The search bisects on the chain values, which is valid
    /// because equal chains imply equal prefixes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_deterministic_sim::{DeterministicSim, StepLog};
    ///
    /// let mut a = DeterministicSim::<u64>::new();
    /// a.spawn(0);
    /// let mut log_a = StepLog::new();
    /// a.step_n_logged(10, &mut log_a);
    ///
    /// // The second run differs from step 4 on, because the state is bumped.
    /// let mut b = DeterministicSim::<u64>::new();
    /// b.spawn(0);
    /// let mut log_b = StepLog::new();
    /// b.step_n_logged(4, &mut log_b);
    /// b.add_system("bump", |entities, _| {
    ///     for v in entities.values_mut() {
    ///         *v += 1;
    ///     }
    /// });
    /// b.step_n_logged(6, &mut log_b);
    /// assert_eq!(log_a.first_divergence(&log_b), Some(4));
    /// ```
    pub fn first_divergence(&self, other: &StepLog) -> Option<usize> {
        let common = self.len().min(other.len());
        let differs = |i: usize| self.entries[i].chain != other.entries[i].chain;

        // Find the smallest index with a different chain. Equal chains are a
        // prefix, so `differs` is monotone.
        let (mut lo, mut hi) = (0, common);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if differs(mid) {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        if lo < common {
            Some(lo)
        } else if self.len() != other.len() {
            Some(common)
        } else {
            None
        }
    }
}

/// Folds a 64-bit word into `state`, one byte at a time, little-endian.
fn fold(mut state: u64, word: u64) -> u64 {
    for b in word.to_le_bytes() {
        state ^= u64::from(b);
        state = state.wrapping_mul(PRIME);
    }
    state
}

impl<S: Clone + fmt::Debug + Hash> DeterministicSim<S> {
    /// Advances by one step and appends the result to `log`.
    pub fn step_logged(&mut self, log: &mut StepLog) {
        self.step();
        log.push(self.step_count(), self.state_hash());
    }

    /// Advances by `n` steps, appending each step to `log`.
    pub fn step_n_logged(&mut self, n: u64, log: &mut StepLog) {
        for _ in 0..n {
            self.step_logged(log);
        }
    }

    /// Replays `log` from the current state, checking each step's state hash.
    ///
    /// The simulation is advanced one step per entry. The state of the
    /// simulation is changed even when a mismatch is found.
    ///
    /// # Errors
    ///
    /// Returns the first [`Divergence`] between the replay and the log.
    pub fn verify_log(&mut self, log: &StepLog) -> Result<(), Divergence> {
        for (index, entry) in log.entries().iter().enumerate() {
            self.step();
            let actual = self.state_hash();
            if self.step_count() != entry.step || actual != entry.state_hash {
                return Err(Divergence {
                    index,
                    expected: entry.state_hash,
                    actual,
                });
            }
        }
        Ok(())
    }
}

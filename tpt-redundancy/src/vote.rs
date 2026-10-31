//! Voting logic for N-modular redundancy.

/// An N-modular-redundant collection of `N` values produced by independent channels.
///
/// Voting is performed over the stored copies to agree on a single value. A
/// strict majority vote tolerates up to `floor((N - 1) / 2)` faulty channels.
///
/// # Example
///
/// ```rust
/// use tpt_redundancy::{Replicated, VoteResult};
///
/// let r = Replicated::new([1u8, 1u8, 2u8]);
/// match r.majority_vote() {
///     VoteResult::Majority(v) => assert_eq!(v, 1u8),
///     _ => panic!("expected a majority of 1"),
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Replicated<T, const N: usize> {
    channels: [T; N],
}

/// The outcome of a value-based majority vote.
///
/// A vote in which `m` channels agree is a *majority* if `m > N / 2` and
/// *unanimous* if `m == N`. With no value exceeding that threshold the result
/// is [`VoteResult::NoMajority`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VoteResult<T> {
    /// All `N` channels reported exactly the same value.
    Unanimous(T),
    /// A strict majority (more than `N / 2` channels) reported `T`, but not all.
    Majority(T),
    /// No single value reached a strict majority.
    NoMajority,
}

impl<T> VoteResult<T> {
    /// Returns `true` if a value was agreed upon by a strict majority or more.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_redundancy::{Replicated, VoteResult};
    ///
    /// let tie = Replicated::new([1u8, 2u8, 3u8]);
    /// assert!(!tie.majority_vote().is_majority());
    /// let maj = Replicated::new([1u8, 1u8, 2u8]);
    /// assert!(maj.majority_vote().is_majority());
    /// ```
    pub fn is_majority(&self) -> bool {
        !matches!(self, VoteResult::NoMajority)
    }

    /// Returns the agreed value, or `None` if no majority was reached.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_redundancy::Replicated;
    ///
    /// let r = Replicated::new([7u8, 7u8, 1u8]);
    /// assert_eq!(r.majority_vote().value(), Some(&7u8));
    /// ```
    pub fn value(&self) -> Option<&T> {
        match self {
            VoteResult::Unanimous(v) | VoteResult::Majority(v) => Some(v),
            VoteResult::NoMajority => None,
        }
    }
}

impl<T, const N: usize> Replicated<T, N> {
    /// Creates a `Replicated` from an array of `N` channel outputs.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_redundancy::Replicated;
    ///
    /// let r = Replicated::new([1u32, 2u32, 3u32]);
    /// assert_eq!(r.len(), 3);
    /// ```
    pub fn new(channels: [T; N]) -> Self {
        Self { channels }
    }

    /// Returns the number of redundant channels `N`.
    pub fn len(&self) -> usize {
        N
    }

    /// Returns `true` if there are zero channels (i.e. `N == 0`).
    pub fn is_empty(&self) -> bool {
        N == 0
    }

    /// Returns a reference to the value produced by channel `i`.
    ///
    /// # Panics
    ///
    /// Panics if `i >= N`.
    pub fn channel(&self, i: usize) -> &T {
        &self.channels[i]
    }
}

impl<T: Clone + PartialEq, const N: usize> Replicated<T, N> {
    /// Performs a value-based majority vote over the channels.
    ///
    /// Counts how many channels report each distinct value and classifies the
    /// most frequent value:
    ///
    /// - [`VoteResult::Unanimous`] when every channel agrees,
    /// - [`VoteResult::Majority`] when a strict majority agrees but not all,
    /// - [`VoteResult::NoMajority`] otherwise (including exact ties on an even `N`).
    ///
    /// For `N == 0` the result is always [`VoteResult::NoMajority`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_redundancy::{Replicated, VoteResult};
    ///
    /// let unan = Replicated::new([5u8, 5u8, 5u8]).majority_vote();
    /// assert!(matches!(unan, VoteResult::Unanimous(5)));
    ///
    /// let maj = Replicated::new([5u8, 5u8, 9u8]).majority_vote();
    /// assert!(matches!(maj, VoteResult::Majority(5)));
    ///
    /// // Even N with an exact tie → NoMajority.
    /// let tie = Replicated::new([1u8, 2u8]).majority_vote();
    /// assert!(matches!(tie, VoteResult::NoMajority));
    /// ```
    pub fn majority_vote(&self) -> VoteResult<T> {
        if N == 0 {
            return VoteResult::NoMajority;
        }

        let mut best_count: usize = 0;
        let mut best_val: Option<&T> = None;

        for i in 0..N {
            let cand = &self.channels[i];
            let mut count = 1usize;
            for j in (i + 1)..N {
                if self.channels[j] == *cand {
                    count += 1;
                }
            }
            if count > best_count {
                best_count = count;
                best_val = Some(cand);
            }
        }

        let threshold = N / 2;
        match best_val {
            None => VoteResult::NoMajority,
            Some(v) if best_count == N => VoteResult::Unanimous(v.clone()),
            Some(v) if best_count > threshold => VoteResult::Majority(v.clone()),
            Some(_) => VoteResult::NoMajority,
        }
    }
}

impl<T: Ord + Clone, const N: usize> Replicated<T, N> {
    /// Returns the median of the channel values, or `None` when `N == 0`.
    ///
    /// The values are sorted by [`Ord`] and the middle element (at index `N / 2`)
    /// is returned. This is most meaningful for **odd `N`**, where the median is
    /// robust against `(N - 1) / 2` faulty (outlier) channels. For even `N` the
    /// upper-middle element is returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_redundancy::Replicated;
    ///
    /// let r = Replicated::new([3u32, 1u32, 2u32, 4u32, 9u32]);
    /// assert_eq!(r.median_vote(), Some(3u32)); // sorted: [1,2,3,4,9], middle = 3
    ///
    /// let empty: Replicated<u32, 0> = Replicated::new([]);
    /// assert_eq!(empty.median_vote(), None);
    /// ```
    pub fn median_vote(&self) -> Option<T> {
        self.order_statistic(N / 2)
    }

    /// Returns the value of the channel that is `k`-th in ascending order (0-based),
    /// or `None` if `k >= N`.
    ///
    /// A convenience wrapper over a sorted copy of the channels; `median_vote` is
    /// `order_statistic(N / 2)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_redundancy::Replicated;
    ///
    /// let r = Replicated::new([3u32, 1u32, 2u32]);
    /// assert_eq!(r.order_statistic(0), Some(1u32));
    /// assert_eq!(r.order_statistic(2), Some(3u32));
    /// assert_eq!(r.order_statistic(3), None);
    /// ```
    pub fn order_statistic(&self, k: usize) -> Option<T> {
        self.sorted_channels().get(k).cloned()
    }

    /// Returns a sorted copy of the channels (insertion sort: stable, no
    /// allocation, and fine for the small `N` this crate targets).
    fn sorted_channels(&self) -> [T; N] {
        let mut sorted: [T; N] = self.channels.clone();
        for i in 1..N {
            let mut j = i;
            while j > 0 && sorted[j - 1] > sorted[j] {
                sorted.swap(j - 1, j);
                j -= 1;
            }
        }
        sorted
    }
}

impl<T: Copy + Into<u64> + BitNarrow, const N: usize> Replicated<T, N> {
    /// Performs a bit-by-bit majority vote, producing an unsigned integer.
    ///
    /// Each channel value is widened to `u64`; for every bit position the most
    /// frequent bit among the `N` channels is selected. This is robust against
    /// stuck-at faults on unsigned data buses. The result is narrowed back to
    /// `T` by truncation.
    ///
    /// # Even `N` ties
    ///
    /// When `N` is even and exactly half of the channels have a bit set, the
    /// vote is a tie and that bit is **0**. The result is then biased toward
    /// zero, and no tie is reported. Use an odd `N` when the bitwise result
    /// must not depend on tie-breaking.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_redundancy::Replicated;
    ///
    /// // Two channels agree that bit 1 and bit 3 are set; one disagrees.
    /// let r = Replicated::new([0b1010u8, 0b1010u8, 0b0010u8]);
    /// assert_eq!(r.bitwise_vote(), 0b1010u8);
    ///
    /// // Even-N tie on bit 0 resolves to 0.
    /// let tie = Replicated::new([0b1u8, 0b0u8]);
    /// assert_eq!(tie.bitwise_vote(), 0u8);
    /// ```
    pub fn bitwise_vote(&self) -> T {
        let mut acc: u64 = 0;
        for bit in 0..64u32 {
            let mut zeros = 0usize;
            let mut ones = 0usize;
            for c in &self.channels {
                if ((*c).into() >> bit) & 1 == 1 {
                    ones += 1;
                } else {
                    zeros += 1;
                }
            }
            if ones > zeros {
                acc |= 1u64 << bit;
            }
        }
        T::bit_narrow(acc)
    }
}

mod private {
    /// Prevents `BitNarrow` from being implemented outside this crate.
    pub trait Sealed {}
}

/// Sealed helper trait for narrowing a `u64` accumulator back to an unsigned
/// integer type by truncation (only the low bits are meaningful for voting).
///
/// This trait is implemented for `u8`, `u16`, `u32` and `u64` only. It cannot be
/// implemented outside this crate.
pub trait BitNarrow: Copy + private::Sealed {
    /// Converts a `u64` to `Self` by truncation.
    fn bit_narrow(v: u64) -> Self;
}

macro_rules! impl_bit_narrow {
    ($($t:ty),*) => {
        $(
            impl private::Sealed for $t {}

            impl BitNarrow for $t {
                fn bit_narrow(v: u64) -> Self {
                    v as $t
                }
            }
        )*
    };
}
impl_bit_narrow!(u8, u16, u32, u64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unanimous_majority_and_tie() {
        assert!(matches!(
            Replicated::new([4u8, 4, 4]).majority_vote(),
            VoteResult::Unanimous(4)
        ));
        assert!(matches!(
            Replicated::new([4u8, 4, 9]).majority_vote(),
            VoteResult::Majority(4)
        ));
        assert!(matches!(
            Replicated::new([4u8, 9, 1]).majority_vote(),
            VoteResult::NoMajority
        ));
    }

    #[test]
    fn majority_tolerates_floor_of_n_minus_one_over_two_faults() {
        // N = 5: two faulty channels (floor(4/2) = 2) still leave a 3-of-5 majority.
        assert_eq!(
            Replicated::new([7u8, 7, 7, 1, 2]).majority_vote().value(),
            Some(&7)
        );
        // N = 4: one faulty channel leaves 3-of-4; two faults leave a 2-2 tie.
        assert_eq!(
            Replicated::new([7u8, 7, 7, 1]).majority_vote().value(),
            Some(&7)
        );
        assert_eq!(
            Replicated::new([7u8, 7, 1, 1]).majority_vote().value(),
            None
        );
    }

    #[test]
    fn empty_majority_is_no_majority() {
        let r: Replicated<u8, 0> = Replicated::new([]);
        assert!(matches!(r.majority_vote(), VoteResult::NoMajority));
        assert!(r.is_empty());
    }

    #[test]
    fn median_matches_known_arrays() {
        assert_eq!(Replicated::new([5u32, 1, 9, 3, 7]).median_vote(), Some(5));
        assert_eq!(Replicated::new([2u32, 2, 100]).median_vote(), Some(2));
        // Even N: upper-middle element.
        assert_eq!(Replicated::new([4u32, 1, 3, 2]).median_vote(), Some(3));
    }

    #[test]
    fn median_and_order_statistic_handle_zero_and_out_of_range() {
        let empty: Replicated<u32, 0> = Replicated::new([]);
        assert_eq!(empty.median_vote(), None);
        assert_eq!(empty.order_statistic(0), None);

        let r = Replicated::new([3u32, 1, 2]);
        assert_eq!(r.order_statistic(3), None);
        assert_eq!(r.order_statistic(usize::MAX), None);
    }

    #[test]
    fn bitwise_vote_hand_computed_patterns() {
        // bit0: 1,1,0 → 1; bit1: 0,0,0 → 0; bit2: 1,1,1 → 1; bit3: 0,0,0 → 0
        assert_eq!(
            Replicated::new([0b0101u8, 0b0101, 0b0100]).bitwise_vote(),
            0b0101
        );
        // Stuck-at-one fault on bit 7 is outvoted.
        assert_eq!(Replicated::new([0x10u8, 0x10, 0x90]).bitwise_vote(), 0x10);
        assert_eq!(Replicated::new([0xFFu16, 0xFF, 0x00]).bitwise_vote(), 0xFF);
    }

    #[test]
    fn bitwise_vote_even_n_tie_resolves_to_zero() {
        assert_eq!(
            Replicated::new([0b11u8, 0b00, 0b11, 0b00]).bitwise_vote(),
            0
        );
    }
}

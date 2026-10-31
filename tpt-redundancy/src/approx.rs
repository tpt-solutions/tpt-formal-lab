//! Voting with a caller-supplied agreement relation, plurality voting, and a
//! fault report.
//!
//! The crate does not decide what "close enough" means. A float voter passes
//! its tolerance as the agreement relation, so the rule is explicit at the call
//! site and nothing here depends on floating-point behaviour.

use crate::vote::{Replicated, VoteResult};

impl<T: Clone, const N: usize> Replicated<T, N> {
    /// Votes using `agree` to decide which channel values count as the same.
    ///
    /// Each channel's support is the number of channels that agree with it,
    /// counting itself. The channel with the largest support decides the result,
    /// with the same thresholds as [`Replicated::majority_vote`]. `agree` should
    /// be reflexive and symmetric. The result carries the value of the
    /// best-supported channel, and the first such channel wins ties.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_redundancy::{Replicated, VoteResult};
    ///
    /// // Three sensors read the same quantity with a tolerance of 0.5.
    /// let r = Replicated::new([10.0_f64, 10.2, 13.0]);
    /// let close = |a: &f64, b: &f64| (a - b).abs() <= 0.5;
    /// assert!(matches!(r.approx_vote(close), VoteResult::Majority(v) if v == 10.0));
    /// ```
    pub fn approx_vote(&self, agree: impl Fn(&T, &T) -> bool) -> VoteResult<T> {
        match self.best_candidate(&agree) {
            None => VoteResult::NoMajority,
            Some((idx, support)) => {
                let value = self.channel(idx).clone();
                if support == N {
                    VoteResult::Unanimous(value)
                } else if support > N / 2 {
                    VoteResult::Majority(value)
                } else {
                    VoteResult::NoMajority
                }
            }
        }
    }

    /// Reports which channels disagree with the majority value, judged by `agree`.
    ///
    /// Returns `None` when no strict majority exists, because then no channel can
    /// be blamed. Otherwise entry `i` is `true` if channel `i` does not agree with
    /// the majority value. Only faults within the tolerance of the majority are
    /// distinguished from the rest.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_redundancy::Replicated;
    ///
    /// let r = Replicated::new([7u8, 7, 1]);
    /// assert_eq!(r.faulty_channels_by(|a, b| a == b), Some([false, false, true]));
    /// assert_eq!(Replicated::new([1u8, 2]).faulty_channels_by(|a, b| a == b), None);
    /// ```
    pub fn faulty_channels_by(&self, agree: impl Fn(&T, &T) -> bool) -> Option<[bool; N]> {
        let (idx, support) = self.best_candidate(&agree)?;
        if support <= N / 2 {
            return None;
        }
        let value = self.channel(idx);
        Some(core::array::from_fn(|j| !agree(value, self.channel(j))))
    }

    /// Returns the best-supported channel as `(index, support)`, with ties going
    /// to the earliest channel. `None` only when `N == 0`.
    fn best_candidate(&self, agree: &impl Fn(&T, &T) -> bool) -> Option<(usize, usize)> {
        let mut best: Option<(usize, usize)> = None;
        for i in 0..N {
            let support = (0..N)
                .filter(|&j| j == i || agree(self.channel(i), self.channel(j)))
                .count();
            match best {
                Some((_, best_support)) if best_support >= support => {}
                _ => best = Some((i, support)),
            }
        }
        best
    }
}

impl<T: Clone + PartialEq, const N: usize> Replicated<T, N> {
    /// Returns the most frequent value, even when no value has a strict
    /// majority. Ties go to the value that appears first.
    ///
    /// Returns `None` only when `N == 0`. Use [`Replicated::majority_vote`] when
    /// the caller needs to know whether a majority exists.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_redundancy::Replicated;
    ///
    /// assert_eq!(Replicated::new([1u8, 2, 2, 3]).plurality_vote(), Some(2));
    /// assert_eq!(Replicated::new([1u8, 2]).plurality_vote(), Some(1));
    /// ```
    pub fn plurality_vote(&self) -> Option<T> {
        let (idx, _) = self.best_candidate(&|a: &T, b: &T| a == b)?;
        Some(self.channel(idx).clone())
    }

    /// Reports which channels disagree with the majority value, using `==`.
    /// See [`Replicated::faulty_channels_by`].
    pub fn faulty_channels(&self) -> Option<[bool; N]> {
        self.faulty_channels_by(|a, b| a == b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approx_vote_groups_values_within_tolerance() {
        let close = |a: &i32, b: &i32| (a - b).abs() <= 2;
        assert!(matches!(
            Replicated::new([10i32, 11, 12]).approx_vote(close),
            VoteResult::Unanimous(10)
        ));
        assert!(matches!(
            Replicated::new([10i32, 11, 50]).approx_vote(close),
            VoteResult::Majority(10)
        ));
        assert!(matches!(
            Replicated::new([10i32, 50, 90]).approx_vote(close),
            VoteResult::NoMajority
        ));
    }

    #[test]
    fn approx_vote_with_equality_matches_majority_vote() {
        for channels in [[1u8, 1, 2], [3, 3, 3], [1, 2, 3], [4, 4, 5]] {
            let r = Replicated::new(channels);
            assert_eq!(r.approx_vote(|a, b| a == b), r.majority_vote());
        }
    }

    #[test]
    fn approx_vote_on_empty_is_no_majority() {
        let r: Replicated<u8, 0> = Replicated::new([]);
        assert!(matches!(
            r.approx_vote(|a, b| a == b),
            VoteResult::NoMajority
        ));
    }

    #[test]
    fn plurality_picks_most_frequent_then_earliest() {
        assert_eq!(Replicated::new([5u8, 9, 9, 5, 9]).plurality_vote(), Some(9));
        assert_eq!(Replicated::new([5u8, 9, 9, 5]).plurality_vote(), Some(5));
        assert_eq!(Replicated::new([7u8]).plurality_vote(), Some(7));
        let empty: Replicated<u8, 0> = Replicated::new([]);
        assert_eq!(empty.plurality_vote(), None);
    }

    #[test]
    fn faulty_channels_flags_disagreeing_channels() {
        assert_eq!(
            Replicated::new([4u8, 4, 4, 9]).faulty_channels(),
            Some([false, false, false, true])
        );
        assert_eq!(
            Replicated::new([4u8, 4, 4]).faulty_channels(),
            Some([false, false, false])
        );
    }

    #[test]
    fn faulty_channels_needs_a_strict_majority() {
        assert_eq!(Replicated::new([1u8, 1, 2, 2]).faulty_channels(), None);
        assert_eq!(Replicated::new([1u8, 2, 3]).faulty_channels(), None);
    }

    #[test]
    fn faulty_channels_by_tolerance() {
        let close = |a: &f32, b: &f32| (a - b).abs() <= 0.5;
        assert_eq!(
            Replicated::new([1.0f32, 1.4, 9.0]).faulty_channels_by(close),
            Some([false, false, true])
        );
    }
}

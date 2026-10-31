//! Strategies for compound values, and combinators that build new strategies
//! from existing ones.
//!
//! Every strategy here is deterministic: it draws from the RNG in a fixed order,
//! so a seed still fixes the whole sequence of generated values.
//!
//! Compound strategies have no single index space, so shrinking does not
//! apply to them. A failing compound value is reported unshrunk.

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::rng::Xorshift64;
use crate::strategy::{IntRange, Strategy};

/// The most rejected draws [`Filter`] allows for one value before it gives up.
pub const MAX_FILTER_REJECTIONS: usize = 1_000;

impl<A: Strategy, B: Strategy> Strategy for (A, B) {
    type Value = (A::Value, B::Value);

    fn generate(&self, rng: &mut Xorshift64) -> Self::Value {
        let a = self.0.generate(rng);
        let b = self.1.generate(rng);
        (a, b)
    }
}

impl<A: Strategy, B: Strategy, C: Strategy> Strategy for (A, B, C) {
    type Value = (A::Value, B::Value, C::Value);

    fn generate(&self, rng: &mut Xorshift64) -> Self::Value {
        let a = self.0.generate(rng);
        let b = self.1.generate(rng);
        let c = self.2.generate(rng);
        (a, b, c)
    }
}

/// Generates `Some` or `None` with equal probability. Built by [`optional`].
#[derive(Clone, Copy, Debug)]
pub struct Optional<S>(S);

/// Generates `Option<S::Value>`, `None` and `Some` each half the time.
///
/// # Example
///
/// ```rust
/// use tpt_det_proptest::{optional, IntRange, Strategy, Xorshift64};
///
/// let strat = optional(IntRange::<u8>::new(0, 10));
/// let mut rng = Xorshift64::new(1);
/// let v = strat.generate(&mut rng);
/// assert!(v.map_or(true, |x| x < 10));
/// ```
pub fn optional<S: Strategy>(inner: S) -> Optional<S> {
    Optional(inner)
}

impl<S: Strategy> Strategy for Optional<S> {
    type Value = Option<S::Value>;

    fn generate(&self, rng: &mut Xorshift64) -> Self::Value {
        if rng.next_u64() & 1 == 1 {
            Some(self.0.generate(rng))
        } else {
            None
        }
    }
}

/// Generates a `Vec` whose length is drawn from `len`. Built by [`vec_of`].
#[derive(Clone, Debug)]
pub struct VecOf<S> {
    element: S,
    len: IntRange<usize>,
}

/// Generates vectors of `element` values, with a length in `[min_len, max_len)`.
///
/// # Panics
///
/// Panics if `min_len >= max_len`.
///
/// # Example
///
/// ```rust
/// use tpt_det_proptest::{vec_of, IntRange, Strategy, Xorshift64};
///
/// let strat = vec_of(IntRange::<i32>::new(0, 5), 2, 4);
/// let mut rng = Xorshift64::new(9);
/// let v = strat.generate(&mut rng);
/// assert!((2..4).contains(&v.len()));
/// ```
pub fn vec_of<S: Strategy>(element: S, min_len: usize, max_len: usize) -> VecOf<S> {
    VecOf {
        element,
        len: IntRange::new(min_len, max_len),
    }
}

impl<S: Strategy> Strategy for VecOf<S> {
    type Value = Vec<S::Value>;

    fn generate(&self, rng: &mut Xorshift64) -> Self::Value {
        let len = self.len.generate(rng);
        (0..len).map(|_| self.element.generate(rng)).collect()
    }
}

/// Transforms each generated value. Built by [`StrategyExt::map`].
#[derive(Clone, Copy, Debug)]
pub struct Map<S, F> {
    inner: S,
    f: F,
}

impl<S: Strategy, U, F: Fn(S::Value) -> U> Strategy for Map<S, F> {
    type Value = U;

    fn generate(&self, rng: &mut Xorshift64) -> U {
        (self.f)(self.inner.generate(rng))
    }
}

/// Keeps only the values that satisfy a predicate. Built by [`StrategyExt::filter`].
#[derive(Clone, Copy, Debug)]
pub struct Filter<S, P> {
    inner: S,
    predicate: P,
}

impl<S: Strategy, P: Fn(&S::Value) -> bool> Strategy for Filter<S, P> {
    type Value = S::Value;

    /// # Panics
    ///
    /// Panics if [`MAX_FILTER_REJECTIONS`] consecutive values are rejected,
    /// which usually means the predicate is unsatisfiable for this strategy.
    fn generate(&self, rng: &mut Xorshift64) -> S::Value {
        for _ in 0..MAX_FILTER_REJECTIONS {
            let v = self.inner.generate(rng);
            if (self.predicate)(&v) {
                return v;
            }
        }
        panic!("filter rejected {MAX_FILTER_REJECTIONS} values in a row");
    }
}

/// Chooses one of several strategies, uniformly. Built by [`one_of`].
pub struct OneOf<T> {
    choices: Vec<Box<dyn Strategy<Value = T>>>,
}

/// Chooses uniformly among `choices` for each value, then draws from that strategy.
///
/// # Panics
///
/// Panics if `choices` is empty.
///
/// # Example
///
/// ```rust
/// use tpt_det_proptest::{one_of, IntRange, Strategy, Xorshift64};
///
/// let strat = one_of(vec![
///     Box::new(IntRange::<i32>::new(0, 10)) as Box<dyn Strategy<Value = i32>>,
///     Box::new(IntRange::<i32>::new(100, 110)),
/// ]);
/// let mut rng = Xorshift64::new(3);
/// let v = strat.generate(&mut rng);
/// assert!((0..10).contains(&v) || (100..110).contains(&v));
/// ```
pub fn one_of<T>(choices: Vec<Box<dyn Strategy<Value = T>>>) -> OneOf<T> {
    assert!(!choices.is_empty(), "one_of requires at least one strategy");
    OneOf { choices }
}

impl<T> Strategy for OneOf<T> {
    type Value = T;

    fn generate(&self, rng: &mut Xorshift64) -> T {
        let idx = rng.next_below(self.choices.len() as u128) as usize;
        self.choices[idx].generate(rng)
    }
}

/// Combinators available on every [`Strategy`].
pub trait StrategyExt: Strategy + Sized {
    /// Transforms each generated value with `f`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_det_proptest::{IntRange, Strategy, StrategyExt, Xorshift64};
    ///
    /// let doubled = IntRange::<i32>::new(0, 5).map(|x| x * 2);
    /// let mut rng = Xorshift64::new(4);
    /// assert!(doubled.generate(&mut rng) % 2 == 0);
    /// ```
    fn map<U, F: Fn(Self::Value) -> U>(self, f: F) -> Map<Self, F> {
        Map { inner: self, f }
    }

    /// Keeps only the values for which `predicate` returns `true`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_det_proptest::{IntRange, Strategy, StrategyExt, Xorshift64};
    ///
    /// let evens = IntRange::<i32>::new(0, 100).filter(|x| x % 2 == 0);
    /// let mut rng = Xorshift64::new(5);
    /// assert!(evens.generate(&mut rng) % 2 == 0);
    /// ```
    fn filter<P: Fn(&Self::Value) -> bool>(self, predicate: P) -> Filter<Self, P> {
        Filter {
            inner: self,
            predicate,
        }
    }
}

impl<S: Strategy> StrategyExt for S {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::AnyBool;

    #[test]
    fn tuples_generate_each_component() {
        let strat = (
            IntRange::<u8>::new(0, 4),
            AnyBool,
            IntRange::<i8>::new(-3, 0),
        );
        let mut rng = Xorshift64::new(11);
        for _ in 0..100 {
            let (a, _b, c) = strat.generate(&mut rng);
            assert!(a < 4);
            assert!((-3..0).contains(&c));
        }
    }

    #[test]
    fn optional_produces_both_variants() {
        let strat = optional(IntRange::<u8>::new(0, 10));
        let mut rng = Xorshift64::new(12);
        let (mut some, mut none) = (0, 0);
        for _ in 0..200 {
            match strat.generate(&mut rng) {
                Some(v) => {
                    assert!(v < 10);
                    some += 1;
                }
                None => none += 1,
            }
        }
        assert!(some > 0 && none > 0, "some={some} none={none}");
    }

    #[test]
    fn vec_lengths_stay_in_range() {
        let strat = vec_of(IntRange::<i32>::new(0, 100), 1, 4);
        let mut rng = Xorshift64::new(13);
        let mut lengths = [0usize; 4];
        for _ in 0..300 {
            let v = strat.generate(&mut rng);
            assert!((1..4).contains(&v.len()));
            assert!(v.iter().all(|x| (0..100).contains(x)));
            lengths[v.len()] += 1;
        }
        assert!(lengths[1] > 0 && lengths[2] > 0 && lengths[3] > 0);
    }

    #[test]
    fn map_transforms_values() {
        let strat = IntRange::<u32>::new(1, 5).map(|x| x * 10);
        let mut rng = Xorshift64::new(14);
        for _ in 0..50 {
            let v = strat.generate(&mut rng);
            assert!(v % 10 == 0 && (10..50).contains(&v));
        }
    }

    #[test]
    fn filter_only_yields_accepted_values() {
        let strat = IntRange::<u32>::new(0, 1000).filter(|x| *x > 900);
        let mut rng = Xorshift64::new(15);
        for _ in 0..100 {
            assert!(strat.generate(&mut rng) > 900);
        }
    }

    #[test]
    #[should_panic(expected = "filter rejected")]
    fn unsatisfiable_filter_panics_instead_of_looping() {
        let strat = IntRange::<u32>::new(0, 10).filter(|_| false);
        let mut rng = Xorshift64::new(16);
        strat.generate(&mut rng);
    }

    #[test]
    fn one_of_draws_from_every_choice() {
        let strat = one_of(alloc::vec![
            Box::new(IntRange::<i32>::new(0, 10)) as Box<dyn Strategy<Value = i32>>,
            Box::new(IntRange::<i32>::new(100, 110)),
            Box::new(IntRange::<i32>::new(1000, 1010)),
        ]);
        let mut rng = Xorshift64::new(17);
        let mut hits = [0usize; 3];
        for _ in 0..300 {
            let v = strat.generate(&mut rng);
            let bucket = if v < 10 {
                0
            } else if v < 110 {
                1
            } else {
                2
            };
            hits[bucket] += 1;
        }
        assert!(hits.iter().all(|&h| h > 0), "hits={hits:?}");
    }

    #[test]
    fn compound_generation_is_deterministic() {
        let make = || vec_of((IntRange::<i32>::new(0, 50), AnyBool), 0, 8);
        let a = make().generate(&mut Xorshift64::new(99));
        let b = make().generate(&mut Xorshift64::new(99));
        assert_eq!(a, b);
    }
}

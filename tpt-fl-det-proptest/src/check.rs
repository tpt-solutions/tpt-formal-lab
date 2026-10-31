//! The property-test runner with bisection shrinking.

use crate::rng::Xorshift64;
use crate::strategy::{Seed, Strategy};

/// The result of running a property test.
///
/// `Ok(())` means the property held for every generated value. `Err` carries
/// the counterexample found by shrinking.
pub type TestResult<T> = Result<(), CounterExample<T>>;

/// A counterexample that falsifies a property.
///
/// `original` is the first value that failed. `minimal` is the value that
/// bisection reached, which always falsifies the property.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CounterExample<T>
where
    T: Clone,
{
    /// The first generated value that falsified the property.
    pub original: T,
    /// The smallest falsifying value found by bisection.
    pub minimal: T,
}

/// Runs `property` for `iterations` generated values from `strategy` using `seed`.
///
/// For each generated value the `property` closure is called; it must return
/// `true` for "holds". If it returns `false`, the runner records a
/// [`CounterExample`] and shrinks it by **bisection** toward a smaller
/// falsifying value in the strategy's index order, then returns immediately.
///
/// Generation and shrinking are fully deterministic: the same `seed` produces
/// the same sequence and the same counterexample on every run.
///
/// # Shrinking
///
/// Shrinking is a halving search over the strategy's index space. It only
/// probes values between the strategy's lower bound and the failing value, so
/// its cost is logarithmic in the range width, whatever that width is.
///
/// For a monotone property, such as `x < 50`, the result is the smallest
/// failing value. For a non-monotone property, bisection finds a failing value
/// with no failing neighbour below it in the probed path. That value is not
/// necessarily the global minimum.
///
/// # Panics
///
/// A property that panics is not caught, so the panic propagates and shrinking
/// does not run. Catching panics needs `std`, and this crate is `no_std`. Write
/// properties that return `false` instead of panicking.
///
/// # Example
///
/// ```rust
/// use tpt_fl_det_proptest::{check, IntRange, Seed};
///
/// // Property holds for all values in range → Ok.
/// let ok = check(100, Seed(1), IntRange::<i32>::new(0, 1000), |x| *x < 1000);
/// assert!(ok.is_ok());
/// ```
///
/// ```rust
/// use tpt_fl_det_proptest::{check, IntRange, Seed};
///
/// // Property fails for x >= 50; shrinking finds the minimal such value, 50.
/// let result = check(100, Seed(1), IntRange::<i32>::new(0, 100), |x| *x < 50);
/// assert!(result.is_err());
/// let ce = result.unwrap_err();
/// assert_eq!(ce.minimal, 50);
/// ```
pub fn check<S, P>(iterations: usize, seed: Seed, strategy: S, property: P) -> TestResult<S::Value>
where
    S: Strategy,
    S::Value: Clone,
    P: Fn(&S::Value) -> bool,
{
    let mut rng = Xorshift64::new(seed.0);

    for _ in 0..iterations {
        let v = strategy.generate(&mut rng);
        if !property(&v) {
            return Err(shrink(strategy, property, v));
        }
    }
    Ok(())
}

/// Bisects toward the smallest value that still falsifies `property`, starting
/// from `first_failure`.
fn shrink<S, P>(strategy: S, property: P, first_failure: S::Value) -> CounterExample<S::Value>
where
    S: Strategy,
    S::Value: Clone,
    P: Fn(&S::Value) -> bool,
{
    let unshrunk = || CounterExample {
        original: first_failure.clone(),
        minimal: first_failure.clone(),
    };

    // Without an ordered index space, or if the failing value has no index,
    // there is nothing to bisect over.
    let (first, last) = match strategy.shrink_bounds() {
        Some(bounds) => bounds,
        None => return unshrunk(),
    };
    let fail_idx = match strategy.index_of(&first_failure) {
        Some(idx) => idx,
        None => return unshrunk(),
    };

    // Invariant: the value at `right` falsifies the property, and every probed
    // value below `left` holds it. Initially `right` is the failing value itself.
    let mut left = first;
    let mut right = fail_idx;
    while left < right {
        let mid = left + (right - left) / 2;
        let candidate = match strategy.at_index(mid) {
            Some(v) => v,
            None => break,
        };
        if property(&candidate) {
            left = mid + 1;
        } else {
            right = mid;
        }
    }

    // `right` was only ever set to an index whose value failed, so this check
    // holds by construction. Re-check anyway, and fall back to the original
    // failure rather than report a value that passes.
    let minimal = match strategy.at_index(right) {
        Some(v) if !property(&v) => v,
        _ => return unshrunk(),
    };

    debug_assert!(last >= right);
    CounterExample {
        original: first_failure,
        minimal,
    }
}

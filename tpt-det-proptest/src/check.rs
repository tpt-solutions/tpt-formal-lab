//! The property-test runner with bisection shrinking.

use crate::rng::Xorshift64;
use crate::strategy::{Seed, Strategy};

/// The result of running a property test.
///
/// `Ok(())` means the property held for every generated value. `Err` carries
/// the minimal counterexample found by shrinking.
pub type TestResult<T> = Result<(), CounterExample<T>>;

/// A minimal counterexample that falsifies a property.
///
/// `original` is the first value that failed; `minimal` is the smallest value
/// (in the strategy's index order) that still falsifies the property after
/// bisection shrinking.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CounterExample<T>
where
    T: Clone,
{
    /// The first generated value that falsified the property.
    pub original: T,
    /// The minimal falsifying value after shrinking.
    pub minimal: T,
}

/// Runs `property` for `iterations` generated values from `strategy` using `seed`.
///
/// For each generated value the `property` closure is called; it must return
/// `true` for "holds". If it returns `false`, the runner records a
/// [`CounterExample`] and attempts to shrink it by **bisection** toward the
/// smallest value (in the strategy's index order) that still falsifies the
/// property, then returns immediately.
///
/// Generation and shrinking are fully deterministic: the same `seed` produces
/// the same sequence and the same minimal counterexample on every run.
///
/// # Example
///
/// ```rust
/// use tpt_det_proptest::{check, IntRange, Seed};
///
/// // Property holds for all values in range → Ok.
/// let ok = check(100, Seed(1), IntRange::<i32>::new(0, 1000), |x| *x < 1000);
/// assert!(ok.is_ok());
/// ```
///
/// ```rust
/// use tpt_det_proptest::{check, IntRange, Seed};
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
    // Without an ordered index space we cannot shrink; return the original.
    let bounds = match strategy.shrink_bounds() {
        Some(b) => b,
        None => {
            return CounterExample {
                original: first_failure.clone(),
                minimal: first_failure,
            }
        }
    };

    // Convert the first failing value to its index via the strategy. We recover
    // the index by scanning? Instead, regenerate the failing value's index is
    // not directly available, so we bisect in index space between the lower
    // bound and the failing value's own index. To get that index we re-derive it
    // by comparing generated values is infeasible; we instead treat `first_value`
    // as the upper end and bisect from the strategy low bound upward.
    //
    // We locate the failing index by linear search from the low bound, which is
    // simple and correct given the small ranges used in practice.
    let (lo, hi) = bounds;
    let mut fail_idx = lo;
    for idx in lo..hi {
        let candidate = strategy.at_index(idx).expect("index in bounds");
        if !property(&candidate) {
            fail_idx = idx;
            break;
        }
    }

    // Bisection: find the smallest index in [lo, fail_idx] that still fails.
    let mut left = lo;
    let mut right = fail_idx;
    let mut minimal_idx = fail_idx;
    while left < right {
        let mid = left + (right - left) / 2;
        let candidate = strategy.at_index(mid).expect("index in bounds");
        if !property(&candidate) {
            minimal_idx = mid;
            right = mid;
        } else {
            left = mid + 1;
        }
    }

    CounterExample {
        original: first_failure,
        minimal: strategy.at_index(minimal_idx).expect("index in bounds"),
    }
}

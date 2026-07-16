//! Integration tests: contract violations panic in debug builds and are
//! zero-cost (no-op) in release builds.

use tpt_verify_macros::{ensures, invariant, pure, requires};

#[requires(x > 0)]
fn positive_only(x: i32) -> i32 {
    x * 2
}

#[ensures(result >= 0)]
fn abs_value(x: i32) -> i32 {
    x.abs()
}

#[ensures(result >= 0)]
fn broken_abs(x: i32) -> i32 {
    // Deliberately wrong: does not actually take the absolute value.
    x
}

struct Counter {
    value: u32,
    max: u32,
}

impl Counter {
    #[invariant(self.value <= self.max)]
    fn increment_unchecked(&mut self) {
        self.value += 1;
    }
}

#[pure]
fn square(x: i32) -> i32 {
    x * x
}

#[test]
fn satisfied_contracts_never_panic() {
    assert_eq!(positive_only(5), 10);
    assert_eq!(abs_value(-3), 3);
    assert_eq!(square(4), 16);

    let mut c = Counter { value: 0, max: 5 };
    c.increment_unchecked();
    assert_eq!(c.value, 1);
}

#[cfg(debug_assertions)]
mod debug_behavior {
    use super::*;

    #[test]
    #[should_panic(expected = "Precondition violated")]
    fn requires_panics_on_violation() {
        positive_only(-1);
    }

    #[test]
    #[should_panic(expected = "Postcondition violated")]
    fn ensures_panics_on_violation() {
        broken_abs(-3);
    }

    #[test]
    #[should_panic(expected = "Invariant violated")]
    fn invariant_panics_on_violation() {
        let mut c = Counter { value: 5, max: 5 };
        c.increment_unchecked();
    }
}

#[cfg(not(debug_assertions))]
mod release_behavior {
    use super::*;

    #[test]
    fn contracts_are_zero_cost_in_release() {
        // Violated contracts do not panic when debug_assertions is off —
        // the checks compile away entirely.
        assert_eq!(positive_only(-1), -2);
        assert_eq!(broken_abs(-3), -3);

        let mut c = Counter { value: 5, max: 5 };
        c.increment_unchecked();
        assert_eq!(c.value, 6);
    }
}

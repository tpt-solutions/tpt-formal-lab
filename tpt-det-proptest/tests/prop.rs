//! Tests for `tpt-det-proptest`.

extern crate alloc;

use tpt_det_proptest::{check, AnyBool, IntRange, Seed, Strategy, Xorshift64};

#[test]
fn same_seed_same_sequence() {
    let a: alloc::vec::Vec<i32> = {
        let mut rng = Xorshift64::new(123);
        let strat = IntRange::<i32>::new(0, 1000);
        (0..10).map(|_| strat.generate(&mut rng)).collect()
    };
    let b: alloc::vec::Vec<i32> = {
        let mut rng = Xorshift64::new(123);
        let strat = IntRange::<i32>::new(0, 1000);
        (0..10).map(|_| strat.generate(&mut rng)).collect()
    };
    assert_eq!(a, b);
}

#[test]
fn different_seed_different_sequence() {
    let a: alloc::vec::Vec<i32> = {
        let mut rng = Xorshift64::new(1);
        let strat = IntRange::<i32>::new(0, 1_000_000);
        (0..10).map(|_| strat.generate(&mut rng)).collect()
    };
    let b: alloc::vec::Vec<i32> = {
        let mut rng = Xorshift64::new(2);
        let strat = IntRange::<i32>::new(0, 1_000_000);
        (0..10).map(|_| strat.generate(&mut rng)).collect()
    };
    assert_ne!(a, b);
}

#[test]
fn passing_property_runs_all_iterations() {
    let result = check(200, Seed(42), IntRange::<i32>::new(-50, 50), |x| {
        *x >= -50 && *x < 50
    });
    assert!(result.is_ok());
}

#[test]
fn anybool_generates_both_values_over_many_runs() {
    let mut saw_true = false;
    let mut saw_false = false;
    let mut rng = Xorshift64::new(7);
    let strat = AnyBool;
    for _ in 0..100 {
        if strat.generate(&mut rng) {
            saw_true = true;
        } else {
            saw_false = true;
        }
    }
    assert!(saw_true && saw_false);
}

#[test]
fn known_failure_shrinks_to_minimal() {
    // Property: x < 50. Fails for x in [50, 100). Minimal counterexample is 50.
    let result = check(500, Seed(3), IntRange::<i32>::new(0, 100), |x| *x < 50);
    assert!(result.is_err());
    let ce = result.unwrap_err();
    assert_eq!(ce.minimal, 50);
    assert!((ce.original) >= 50);
}

#[test]
fn known_failure_shrinks_to_minimal_signed() {
    // Property: x != 4. Minimal falsifying value is exactly 4.
    let result = check(500, Seed(11), IntRange::<i8>::new(-10, 10), |x| *x != 4);
    assert!(result.is_err());
    let ce = result.unwrap_err();
    assert_eq!(ce.minimal, 4);
}

#[test]
fn shrinking_is_deterministic() {
    let a = check(500, Seed(99), IntRange::<i32>::new(0, 1000), |x| *x < 777)
        .unwrap_err()
        .minimal;
    let b = check(500, Seed(99), IntRange::<i32>::new(0, 1000), |x| *x < 777)
        .unwrap_err()
        .minimal;
    assert_eq!(a, b);
    assert_eq!(a, 777);
}

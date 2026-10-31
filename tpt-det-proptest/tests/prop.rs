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

#[test]
fn zero_seed_runs_and_differs_from_seed_one() {
    let a: alloc::vec::Vec<u64> = {
        let mut rng = Xorshift64::new(0);
        (0..8).map(|_| rng.next_u64()).collect()
    };
    let b: alloc::vec::Vec<u64> = {
        let mut rng = Xorshift64::new(1);
        (0..8).map(|_| rng.next_u64()).collect()
    };
    assert!(a.iter().all(|&v| v != 0));
    assert_ne!(a, b);
    assert!(check(100, Seed(0), IntRange::<i32>::new(0, 10), |x| *x < 10).is_ok());
}

#[test]
fn shrinks_wide_range_quickly_to_exact_boundary() {
    // A linear scan over 2^64 indices would never finish. Bisection needs about
    // 64 probes and lands exactly on the boundary 2^40.
    let boundary: u64 = 1 << 40;
    let result = check(200, Seed(17), IntRange::<u64>::new(0, u64::MAX), |x| {
        *x < boundary
    });
    let ce = result.unwrap_err();
    assert_eq!(ce.minimal, boundary);
    assert!(ce.original >= boundary);
}

#[test]
fn shrinks_non_monotone_failure_set_to_a_failing_value() {
    // Fails on 7 and on the band 30..=40. The minimal value must itself fail,
    // and it can be no larger than the original failure.
    let property = |x: &i32| !(*x == 7 || (30..=40).contains(x));
    let ce = check(500, Seed(9), IntRange::<i32>::new(0, 100), property).unwrap_err();
    assert!(!property(&ce.minimal));
    assert!(ce.minimal <= ce.original);
}

#[test]
fn inclusive_range_reaches_maximum() {
    let strat = IntRange::<u8>::inclusive(250, u8::MAX);
    assert_eq!(strat.at_index(255), Some(u8::MAX));
    assert_eq!(strat.at_index(249), None);
    let mut rng = Xorshift64::new(3);
    let mut saw_max = false;
    for _ in 0..2_000 {
        let v = strat.generate(&mut rng);
        assert!(v >= 250);
        saw_max |= v == u8::MAX;
    }
    assert!(saw_max, "inclusive upper bound was never generated");
}

#[test]
fn full_width_u64_range_is_generatable() {
    let strat = IntRange::<u64>::inclusive(0, u64::MAX);
    let mut rng = Xorshift64::new(8);
    let _ = strat.generate(&mut rng);
    assert_eq!(strat.at_index(u64::MAX as u128), Some(u64::MAX));
}

#[test]
fn full_width_i128_range_is_generatable_and_ordered() {
    let strat = IntRange::<i128>::inclusive(i128::MIN, i128::MAX);
    let mut rng = Xorshift64::new(21);
    let values: alloc::vec::Vec<i128> = (0..64).map(|_| strat.generate(&mut rng)).collect();
    assert!(values.iter().any(|v| *v < 0));
    assert!(values.iter().any(|v| *v > 0));
    assert_eq!(strat.at_index(0), Some(i128::MIN));
}

#[test]
fn usize_and_isize_ranges_work() {
    let u = IntRange::<usize>::new(10, 20);
    let s = IntRange::<isize>::new(-5, 5);
    let mut rng = Xorshift64::new(4);
    for _ in 0..200 {
        assert!((10..20).contains(&u.generate(&mut rng)));
        assert!((-5..5).contains(&s.generate(&mut rng)));
    }
    assert!(check(200, Seed(4), IntRange::<usize>::new(0, 100), |x| *x < 100).is_ok());
}

#[test]
fn at_index_respects_lower_bound() {
    let strat = IntRange::<i32>::new(-10, 10);
    assert_eq!(strat.at_index(0), None);
    let (first, last) = strat.shrink_bounds().unwrap();
    assert_eq!(strat.index_of(&-10), Some(first));
    assert_eq!(strat.index_of(&9), Some(last));
    assert_eq!(strat.index_of(&10), None);
}

#[test]
#[should_panic(expected = "low < high")]
fn empty_exclusive_range_panics() {
    let _ = IntRange::<u32>::new(5, 5);
}

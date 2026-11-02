//! Tests for the Taylor-model solver in `tpt-fl-verified-ode`.

use tpt_fl_exact_math::{Interval, Rational};
use tpt_fl_verified_ode::{OdeError, TaylorArith, TaylorFn, TaylorOdeSolver, VectorOdeSolver};

fn r(n: i64, d: i64) -> Rational {
    Rational::from_frac(n, d)
}

struct Oscillator;

impl TaylorFn for Oscillator {
    fn eval<A: TaylorArith>(&self, _t: &A, y: &[A]) -> Vec<A> {
        vec![y[1].clone(), -y[0].clone()]
    }
}

struct Exp;

impl TaylorFn for Exp {
    fn eval<A: TaylorArith>(&self, _t: &A, y: &[A]) -> Vec<A> {
        vec![y[0].clone()]
    }
}

struct Const;

impl TaylorFn for Const {
    fn eval<A: TaylorArith>(&self, t: &A, _y: &[A]) -> Vec<A> {
        // y' = 2 + 3 t, so y(t) = 2t + 3t²/2 from y(0) = 0.
        let two = t.constant(Rational::from(2));
        let three = t.constant(Rational::from(3));
        vec![two + three * t.clone()]
    }
}

struct Logistic;

impl TaylorFn for Logistic {
    fn eval<A: TaylorArith>(&self, _t: &A, y: &[A]) -> Vec<A> {
        vec![y[0].clone() - y[0].clone() * y[0].clone()]
    }
}

// ── Enclosure containment ─────────────────────────────────────────────────────

#[test]
fn exp_growth_contains_true_value() {
    // y' = y, y(0) = 1 → y(1/4) = e^{1/4} ≈ 1.2840254.
    let mut solver = TaylorOdeSolver::new(Exp, 4, Rational::from(0), vec![Rational::from(1)]);
    let end = solver.step(&r(1, 4)).unwrap();
    assert!(end[0].contains(&r(1284, 1000)));
}

#[test]
fn polynomial_rhs_is_exact_enough_to_contain_truth() {
    // y' = 2 + 3t, y(0) = 0 → y(1) = 2 + 3/2 = 3.5, exactly.
    let mut solver = TaylorOdeSolver::new(Const, 3, Rational::from(0), vec![Rational::from(0)]);
    let end = solver.step(&Rational::from(1)).unwrap();
    assert!(end[0].contains(&r(35, 10)));
    assert!(end[0].width() < r(1, 100));
}

#[test]
fn harmonic_oscillator_contains_cos_and_sin() {
    // y0' = y1, y1' = -y0 from (1, 0): y0(1) = cos 1, y1(1) = -sin 1.
    let mut solver = TaylorOdeSolver::new(
        Oscillator,
        4,
        Rational::from(0),
        vec![Rational::from(1), Rational::from(0)],
    );
    let traj = solver.solve(4, &r(1, 4)).unwrap();
    let (t, end) = traj.last().unwrap();
    assert_eq!(t, &Rational::from(1));
    assert!(end[0].contains(&r(54, 100)));
    assert!(end[1].contains(&r(-84, 100)));
}

#[test]
fn logistic_contains_true_solution() {
    // y' = y - y², y(0) = 1/2 → y(t) = 1 / (1 + e^{-t}); y(1) ≈ 0.7310586.
    let mut solver = TaylorOdeSolver::new(Logistic, 4, Rational::from(0), vec![r(1, 2)]);
    let traj = solver.solve(4, &r(1, 4)).unwrap();
    let (_, end) = traj.last().unwrap();
    assert!(end[0].contains(&r(731, 1000)));
}

// ── Tightness relative to the Picard solver ───────────────────────────────────

#[test]
fn taylor_is_tighter_than_picard_on_oscillator() {
    let h = r(1, 4);
    let mut taylor = TaylorOdeSolver::new(
        Oscillator,
        4,
        Rational::from(0),
        vec![Rational::from(1), Rational::from(0)],
    );
    let mut picard = VectorOdeSolver::new(
        |_t: &Interval, y: &[Interval]| vec![y[1].clone(), -y[0].clone()],
        Rational::from(0),
        vec![Rational::from(1), Rational::from(0)],
    );
    let taylor_end = taylor.step(&h).unwrap();
    let picard_end = picard.step(&h).unwrap();
    assert!(taylor_end[0].width() < picard_end[0].width());
    assert!(taylor_end[1].width() < picard_end[1].width());
}

#[test]
fn higher_order_is_not_looser_than_lower_order() {
    let h = r(1, 2);
    let width_at = |order: usize| {
        let mut s = TaylorOdeSolver::new(Exp, order, Rational::from(0), vec![Rational::from(1)]);
        s.step(&h).unwrap()[0].width()
    };
    assert!(width_at(6) <= width_at(2));
}

// ── Step control and errors ───────────────────────────────────────────────────

#[test]
fn solve_until_lands_on_end_time() {
    let mut solver = TaylorOdeSolver::new(
        Oscillator,
        3,
        Rational::from(0),
        vec![Rational::from(1), Rational::from(0)],
    );
    let traj = solver.solve_until(&r(1, 3), &r(1, 4)).unwrap();
    assert_eq!(traj.len(), 2);
    assert_eq!(solver.t(), &r(1, 3));
}

#[test]
fn non_positive_step_is_rejected() {
    let mut solver = TaylorOdeSolver::new(Exp, 2, Rational::from(0), vec![Rational::from(1)]);
    assert_eq!(
        solver.step(&Rational::from(0)),
        Err(OdeError::NonPositiveStep)
    );
    assert_eq!(solver.t(), &Rational::from(0));
}

#[test]
fn blow_up_returns_error_and_leaves_state_unchanged() {
    // y' = y² from y(0) = 1 blows up at t = 1.
    struct Square;
    impl TaylorFn for Square {
        fn eval<A: TaylorArith>(&self, _t: &A, y: &[A]) -> Vec<A> {
            vec![y[0].clone() * y[0].clone()]
        }
    }
    let mut solver = TaylorOdeSolver::new(Square, 3, Rational::from(0), vec![Rational::from(1)]);
    assert!(solver.solve_until(&Rational::from(2), &r(1, 2)).is_err());
    assert!(solver.t() < &Rational::from(2));
}

#[test]
#[should_panic(expected = "Taylor order must be at least 1")]
fn zero_order_panics() {
    let _ = TaylorOdeSolver::new(Exp, 0, Rational::from(0), vec![Rational::from(1)]);
}

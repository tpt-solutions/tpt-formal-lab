//! Tests for the vector system solver in `tpt-fl-verified-ode`.

use tpt_fl_exact_math::{Interval, Rational};
use tpt_fl_verified_ode::{OdeError, OdeSolver, VectorOdeSolver};

fn r(n: i64, d: i64) -> Rational {
    Rational::from_frac(n, d)
}

fn point(x: i64) -> Interval {
    Interval::point(Rational::from(x))
}

// ── Exact cases ───────────────────────────────────────────────────────────────

#[test]
fn constant_slopes_are_exact_per_component() {
    // y0' = 2, y1' = -3 from (0, 0): after t = 1 the enclosure is exactly (2, -3).
    let mut solver = VectorOdeSolver::new(
        |_t: &Interval, _y: &[Interval]| vec![point(2), point(-3)],
        Rational::from(0),
        vec![Rational::from(0), Rational::from(0)],
    );
    let end = solver.step(&Rational::from(1)).unwrap();
    assert_eq!(end.len(), 2);
    assert_eq!(end[0].lo(), &Rational::from(2));
    assert_eq!(end[0].hi(), &Rational::from(2));
    assert_eq!(end[1].lo(), &Rational::from(-3));
    assert_eq!(end[1].hi(), &Rational::from(-3));
}

#[test]
fn decoupled_exponentials_contain_true_values() {
    // y0' = -y0, y1' = 2 y1 from (1, 1): y0(t) = e^{-t}, y1(t) = e^{2t}.
    let mut solver = VectorOdeSolver::new(
        |_t: &Interval, y: &[Interval]| vec![-y[0].clone(), y[1].clone() + y[1].clone()],
        Rational::from(0),
        vec![Rational::from(1), Rational::from(1)],
    );
    let traj = solver.solve(4, &r(1, 4)).unwrap();
    let (_, end) = traj.last().unwrap();
    // At t = 1: e^{-1} ≈ 0.3678794 and e^{2} ≈ 7.3890561.
    assert!(end[0].contains(&r(3679, 10_000)));
    assert!(end[1].contains(&r(7389, 1_000)));
}

// ── Coupled systems ───────────────────────────────────────────────────────────

#[test]
fn harmonic_oscillator_tracks_cos_and_sin() {
    // y0' = y1, y1' = -y0 from (1, 0): y0(t) = cos t, y1(t) = -sin t.
    let mut solver = VectorOdeSolver::new(
        |_t: &Interval, y: &[Interval]| vec![y[1].clone(), -y[0].clone()],
        Rational::from(0),
        vec![Rational::from(1), Rational::from(0)],
    );
    let traj = solver.solve(16, &r(1, 16)).unwrap();
    let (t, end) = traj.last().unwrap();
    assert_eq!(t, &Rational::from(1));
    // cos(1) ≈ 0.5403023 and -sin(1) ≈ -0.8414710. The enclosure must contain
    // the true values; it is allowed to be wider than the true value.
    assert!(end[0].contains(&r(54, 100)));
    assert!(end[1].contains(&r(-84, 100)));
    assert!(end[0].width() < r(1, 10));
    assert!(end[1].width() < r(1, 10));
}

#[test]
fn oscillator_conserves_energy_bounds() {
    // y0² + y1² is conserved, so its enclosure should stay near 1.
    let mut solver = VectorOdeSolver::new(
        |_t: &Interval, y: &[Interval]| vec![y[1].clone(), -y[0].clone()],
        Rational::from(0),
        vec![Rational::from(1), Rational::from(0)],
    );
    for (_, end) in solver.solve(8, &r(1, 8)).unwrap() {
        let energy = end[0].clone() * end[0].clone() + end[1].clone() * end[1].clone();
        assert!(energy.contains(&Rational::from(1)));
    }
}

#[test]
fn vector_state_matches_scalar_solver_for_one_component() {
    // A one-component vector system is the scalar solver: same enclosure.
    let mut scalar = OdeSolver::new(
        |_t: &Interval, y: &Interval| y.clone(),
        Rational::from(0),
        Rational::from(1),
    );
    let mut vector = VectorOdeSolver::new(
        |_t: &Interval, y: &[Interval]| vec![y[0].clone()],
        Rational::from(0),
        vec![Rational::from(1)],
    );
    let (lo, hi) = scalar.step(&r(1, 4)).unwrap();
    let end = vector.step(&r(1, 4)).unwrap();
    assert_eq!(end[0].lo(), &lo);
    assert_eq!(end[0].hi(), &hi);
}

// ── Step control and errors ───────────────────────────────────────────────────

#[test]
fn solve_until_lands_on_end_time() {
    let mut solver = VectorOdeSolver::new(
        |_t: &Interval, y: &[Interval]| vec![y[1].clone(), -y[0].clone()],
        Rational::from(0),
        vec![Rational::from(1), Rational::from(0)],
    );
    let traj = solver.solve_until(&r(1, 3), &r(1, 4)).unwrap();
    assert_eq!(traj.len(), 2);
    assert_eq!(solver.t(), &r(1, 3));
    assert_eq!(solver.y_box().len(), 2);
}

#[test]
fn non_positive_step_is_rejected_and_state_unchanged() {
    let mut solver = VectorOdeSolver::new(
        |_t: &Interval, _y: &[Interval]| vec![point(1)],
        Rational::from(0),
        vec![Rational::from(0)],
    );
    assert_eq!(
        solver.step(&Rational::from(0)),
        Err(OdeError::NonPositiveStep)
    );
    assert_eq!(solver.t(), &Rational::from(0));
    assert_eq!(solver.y(), vec![Rational::from(0)]);
}

#[test]
fn blow_up_returns_error_and_leaves_state_unchanged() {
    // y0' = y0², y0(0) = 1 blows up at t = 1 for every component.
    let mut solver = VectorOdeSolver::new(
        |_t: &Interval, y: &[Interval]| vec![y[0].clone() * y[0].clone(), point(0)],
        Rational::from(0),
        vec![Rational::from(1), Rational::from(0)],
    );
    let result = solver.solve_until(&Rational::from(2), &r(1, 2));
    assert!(result.is_err());
    assert!(solver.t() < &Rational::from(2));
}

#[test]
#[should_panic(expected = "one component per state variable")]
fn wrong_component_count_panics() {
    let solver = VectorOdeSolver::new(
        |_t: &Interval, _y: &[Interval]| vec![point(1)],
        Rational::from(0),
        vec![Rational::from(0), Rational::from(0)],
    );
    let _ = solver.a_priori(&Rational::from(1));
}

#[test]
fn empty_state_is_a_no_op_system() {
    let mut solver = VectorOdeSolver::new(
        |_t: &Interval, _y: &[Interval]| Vec::new(),
        Rational::from(0),
        Vec::new(),
    );
    let end = solver.step(&Rational::from(1)).unwrap();
    assert!(end.is_empty());
    assert_eq!(solver.t(), &Rational::from(1));
}

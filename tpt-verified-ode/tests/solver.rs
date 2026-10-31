//! Tests for `tpt-verified-ode`.

use tpt_exact_math::{Interval, Rational};
use tpt_verified_ode::{OdeError, OdeSolver};

fn r(n: i64, d: i64) -> Rational {
    Rational::from_frac(n, d)
}

fn exp_solver() -> OdeSolver<impl Fn(&Interval, &Interval) -> Interval> {
    OdeSolver::new(
        |_t: &Interval, y: &Interval| y.clone(),
        Rational::from(0),
        Rational::from(1),
    )
}

// ── Enclosure containment ─────────────────────────────────────────────────────

#[test]
fn exp_growth_contains_e_h() {
    // y' = y, y(0) = 1 → y(1/4) = e^{1/4} ≈ 1.2840254.
    let mut solver = exp_solver();
    let (lo, hi) = solver.step(&r(1, 4)).unwrap();
    assert!(lo <= r(1284, 1000), "lo={lo} should be <= 1.284");
    assert!(hi >= r(1284, 1000), "hi={hi} should be >= 1.284");
}

#[test]
fn exp_growth_multistep_contains_e_kh() {
    // Three steps of 1/4 → e^{3/4} ≈ 2.1170000.
    let mut solver = exp_solver();
    let traj = solver.solve(3, &r(1, 4)).unwrap();
    let (lo, hi) = (traj[2].1.clone(), traj[2].2.clone());
    assert!(lo <= r(2117, 1000), "lo={lo} should be <= 2.117");
    assert!(hi >= r(2117, 1000), "hi={hi} should be >= 2.117");
}

#[test]
fn logistic_non_monotone_contains_true_solution() {
    // y' = y (1 - y), y(0) = 1/2 → y(t) = 1/(1 + e^{-t}).
    // y(1/4) ≈ 0.5621765, which lies strictly between 0.56217 and 0.56218.
    let mut solver = OdeSolver::new(
        |_t: &Interval, y: &Interval| {
            let one = Interval::point(Rational::from(1));
            y.clone() * (one - y.clone())
        },
        Rational::from(0),
        r(1, 2),
    );
    let (lo, hi) = solver.step(&r(1, 4)).unwrap();
    assert!(lo <= r(56218, 100000), "lo={lo}");
    assert!(hi >= r(56217, 100000), "hi={hi}");
}

#[test]
fn non_autonomous_rhs_uses_time_interval() {
    // y' = t, y(0) = 0 → y(t) = t²/2. Exact: y(1/2) = 1/8, y(1) = 1/2.
    let mut solver = OdeSolver::new(
        |t: &Interval, _y: &Interval| t.clone(),
        Rational::from(0),
        Rational::from(0),
    );
    let traj = solver.solve(2, &r(1, 2)).unwrap();
    assert!(traj[0].1 <= r(1, 8) && r(1, 8) <= traj[0].2);
    assert!(traj[1].1 <= r(1, 2) && r(1, 2) <= traj[1].2);
}

#[test]
fn stiff_problem_is_handled_by_step_halving() {
    // y' = -50 y, y(0) = 1. With h = 1/10 the Picard map is not a contraction
    // (h·L = 5), so the solver must halve the step. Exact: y(1/10) = e^{-5}.
    // e^{-5} ≈ 0.0067379, which lies strictly between 0.00673 and 0.00674.
    let mut solver = OdeSolver::new(
        |_t: &Interval, y: &Interval| -(Interval::point(Rational::from(50)) * y.clone()),
        Rational::from(0),
        Rational::from(1),
    );
    let (lo, hi) = solver.step(&r(1, 10)).unwrap();
    assert!(lo <= r(674, 100000), "lo={lo}");
    assert!(hi >= r(673, 100000), "hi={hi}");
    assert_eq!(solver.t(), &r(1, 10));
}

// ── Width and failure behaviour ───────────────────────────────────────────────

#[test]
fn single_step_enclosure_is_tight() {
    // Tightening should give a width far below the step-size-scaled box
    // (about 2·h without it), so a single step of 1/4 stays well under 0.1.
    let mut solver = exp_solver();
    let (lo, hi) = solver.step(&r(1, 4)).unwrap();
    assert!(hi.clone() - lo.clone() < r(1, 10), "width={}", hi - lo);
}

#[test]
fn enclosure_width_is_nonnegative_and_bounded() {
    let mut solver = exp_solver();
    let traj = solver.solve(5, &r(1, 8)).unwrap();
    for (_, lo, hi) in &traj {
        assert!(lo <= hi);
        assert!(hi.clone() - lo.clone() < Rational::from(2));
    }
}

#[test]
fn blow_up_returns_error_and_leaves_state_unchanged() {
    // y' = y², y(0) = 1 blows up at t = 1. No enclosure exists over a step
    // that reaches t = 1, so the step must fail rather than return a box.
    let mut solver = OdeSolver::new(
        |_t: &Interval, y: &Interval| y.clone() * y.clone(),
        Rational::from(0),
        Rational::from(1),
    );
    assert_eq!(solver.step(&Rational::from(1)), Err(OdeError::NoEnclosure));
    assert_eq!(solver.t(), &Rational::from(0));
    assert_eq!(solver.y_box().lo(), &Rational::from(1));
    assert_eq!(solver.y_box().hi(), &Rational::from(1));
}

#[test]
fn non_positive_step_is_rejected() {
    let mut solver = exp_solver();
    assert_eq!(
        solver.step(&Rational::zero()),
        Err(OdeError::NonPositiveStep)
    );
    assert_eq!(solver.step(&r(-1, 4)), Err(OdeError::NonPositiveStep));
    assert_eq!(solver.a_priori(&r(-1, 4)), Err(OdeError::NonPositiveStep));
    assert_eq!(solver.t(), &Rational::from(0));
}

#[test]
fn a_priori_does_not_advance_the_solver() {
    let solver = exp_solver();
    let end = solver.a_priori(&r(1, 4)).unwrap();
    assert!(end.contains(&r(1284, 1000)));
    assert_eq!(solver.t(), &Rational::from(0));
    assert_eq!(solver.y_box().lo(), &Rational::from(1));
}

#[test]
fn solve_until_lands_exactly_on_the_end_time() {
    // y' = y: integrate to t = 1 in steps of at most 1/4. The enclosure at the
    // end must contain e ≈ 2.7182818, and the last time must be exactly 1.
    let mut solver = OdeSolver::new(
        |_t: &Interval, y: &Interval| y.clone(),
        Rational::from(0),
        Rational::from(1),
    );
    let traj = solver.solve_until(&r(1, 1), &r(1, 4)).unwrap();
    assert_eq!(traj.len(), 4);
    assert_eq!(solver.t(), &r(1, 1));
    let (_, lo, hi) = traj.last().unwrap();
    assert!(
        lo <= &r(271_828, 100_000) && hi >= &r(271_828, 100_000),
        "[{lo}, {hi}]"
    );
}

#[test]
fn solve_until_shortens_the_last_step() {
    let mut solver = OdeSolver::new(
        |_t: &Interval, y: &Interval| y.clone(),
        Rational::from(0),
        Rational::from(1),
    );
    let traj = solver.solve_until(&r(1, 3), &r(1, 4)).unwrap();
    let times: Vec<Rational> = traj.iter().map(|(t, _, _)| t.clone()).collect();
    assert_eq!(times, vec![r(1, 4), r(1, 3)]);
}

#[test]
fn solve_until_with_a_past_end_time_does_nothing() {
    let mut solver = OdeSolver::new(
        |_t: &Interval, y: &Interval| y.clone(),
        Rational::from(0),
        Rational::from(1),
    );
    assert!(solver
        .solve_until(&Rational::zero(), &r(1, 4))
        .unwrap()
        .is_empty());
    assert_eq!(solver.t(), &Rational::zero());
    assert_eq!(
        solver.solve_until(&r(1, 1), &Rational::zero()),
        Err(OdeError::NonPositiveStep)
    );
}

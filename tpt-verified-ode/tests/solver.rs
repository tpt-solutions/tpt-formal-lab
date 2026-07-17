//! Tests for `tpt-verified-ode`.

use tpt_exact_math::Rational;
use tpt_verified_ode::OdeSolver;

#[test]
fn exp_growth_contains_e_h() {
    // y' = y, y(0)=1 → y(h) = e^h. Use h = 1/4 = 0.25.
    // e^0.25 ≈ 1.284025; enclose with rationals 1.28 and 1.29.
    let mut solver = OdeSolver::new(
        |_t: &Rational, y: &Rational| y.clone(),
        Rational::from(0),
        Rational::from(1),
    );
    let h = Rational::from_frac(1, 4);
    let (lo, hi) = solver.step(&h);
    let lower = Rational::from_frac(128, 100);
    let upper = Rational::from_frac(129, 100);
    assert!(lo <= lower, "lo={} should be <= 1.28", lo);
    assert!(upper <= hi, "hi={} should be >= 1.29", hi);
}

#[test]
fn exp_growth_multistep_contains_e_kh() {
    // After k steps of h, the solution is e^{k h}. Use h = 1/4, three steps → e^{3/4}.
    // e^{0.75} ≈ 2.117; enclose with 2.11 and 2.13.
    let mut solver = OdeSolver::new(
        |_t: &Rational, y: &Rational| y.clone(),
        Rational::from(0),
        Rational::from(1),
    );
    let h = Rational::from_frac(1, 4);
    let traj = solver.solve(3, &h);
    let (lo, hi) = (traj[2].1.clone(), traj[2].2.clone());
    let lower = Rational::from_frac(211, 100);
    let upper = Rational::from_frac(213, 100);
    assert!(lo <= lower, "lo={} should be <= 2.11", lo);
    assert!(upper <= hi, "hi={} should be >= 2.13", hi);
}

#[test]
fn enclosure_width_is_nonnegative() {
    let mut solver = OdeSolver::new(
        |_t: &Rational, y: &Rational| y.clone(),
        Rational::from(0),
        Rational::from(1),
    );
    let traj = solver.solve(5, &Rational::from_frac(1, 4));
    for (_, lo, hi) in &traj {
        assert!(lo <= hi);
    }
}
#[test]
fn debug_single() {
    use tpt_exact_math::Rational;
    let mut solver = OdeSolver::new(
        |_t: &Rational, y: &Rational| y.clone(),
        Rational::from(0),
        Rational::from(1),
    );
    let (lo, hi) = solver.step(&Rational::from_frac(1, 4));
    eprintln!("DEBUG step h=1/4: lo={} hi={}", lo, hi);
    let h = Rational::from_frac(1, 4);
    let ap = solver.a_priori(&h);
    eprintln!("DEBUG a_priori: lo={} hi={}", ap.lo(), ap.hi());
}

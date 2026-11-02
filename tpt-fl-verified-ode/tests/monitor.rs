//! Tests for the numerical monitor in `tpt-fl-verified-ode`.

use tpt_fl_exact_math::{Interval, Rational};
use tpt_fl_verified_ode::{audit_float_trajectory, float_rk4, VectorOdeSolver};

fn oscillator_verified(steps: usize, h: &Rational) -> Vec<(Rational, Vec<Interval>)> {
    let mut solver = VectorOdeSolver::new(
        |_t: &Interval, y: &[Interval]| vec![y[1].clone(), -y[0].clone()],
        Rational::from(0),
        vec![Rational::from(1), Rational::from(0)],
    );
    solver.solve(steps, h).unwrap()
}

fn oscillator_float(steps: usize, h: f64) -> Vec<Vec<f64>> {
    float_rk4(
        |_t, y: &[f64]| vec![y[1], -y[0]],
        0.0,
        &[1.0, 0.0],
        h,
        steps,
    )
}

#[test]
fn accurate_float_run_has_no_escapes() {
    let h = Rational::from_frac(1, 16);
    let verified = oscillator_verified(16, &h);
    let float = oscillator_float(16, 1.0 / 16.0);
    assert!(audit_float_trajectory(&verified, &float).is_empty());
}

#[test]
fn injected_error_is_flagged_at_its_step_and_component() {
    let h = Rational::from_frac(1, 16);
    let verified = oscillator_verified(16, &h);
    let mut float = oscillator_float(16, 1.0 / 16.0);
    float[3][1] += 1.0;

    let escapes = audit_float_trajectory(&verified, &float);
    assert_eq!(escapes.len(), 1);
    assert_eq!(escapes[0].step, 3);
    assert_eq!(escapes[0].component, 1);
    assert!(!escapes[0]
        .enclosure
        .contains(&Rational::from_f64(escapes[0].value).unwrap()));
}

#[test]
fn wrong_float_model_escapes() {
    // The float code integrates y' = +y while the verified run uses y' = -y.
    let h = Rational::from_frac(1, 8);
    let mut verified = VectorOdeSolver::new(
        |_t: &Interval, y: &[Interval]| vec![-y[0].clone()],
        Rational::from(0),
        vec![Rational::from(1)],
    );
    let traj = verified.solve(8, &h).unwrap();
    let float = float_rk4(|_t, y: &[f64]| vec![y[0]], 0.0, &[1.0], 0.125, 8);
    assert!(!audit_float_trajectory(&traj, &float).is_empty());
}

#[test]
fn nan_always_escapes() {
    let h = Rational::from_frac(1, 4);
    let verified = oscillator_verified(4, &h);
    let mut float = oscillator_float(4, 0.25);
    float[0][0] = f64::NAN;
    let escapes = audit_float_trajectory(&verified, &float);
    assert_eq!(escapes.len(), 1);
    assert!(escapes[0].value.is_nan());
}

#[test]
fn rk4_is_accurate_for_exponential() {
    let states = float_rk4(|_t, y: &[f64]| vec![y[0]], 0.0, &[1.0], 0.01, 100);
    assert!((states[99][0] - core::f64::consts::E).abs() < 1e-8);
}

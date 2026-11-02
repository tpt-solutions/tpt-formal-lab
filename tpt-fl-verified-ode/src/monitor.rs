//! Numerical monitor: flags float trajectories that leave a verified enclosure.
//!
//! A verified run gives an enclosure that is guaranteed to contain the true
//! solution at each step. A float integrator approximates the same solution.
//! Any float value outside its enclosure is therefore a numerical error, or a
//! bug in the float code. The monitor reports each such value.

use alloc::vec::Vec;

use tpt_fl_exact_math::{Interval, Rational};

/// A float state value that lies outside its verified enclosure.
#[derive(Clone, Debug, PartialEq)]
pub struct Escape {
    /// Index of the step, counting from zero for the first step.
    pub step: usize,
    /// Index of the state component.
    pub component: usize,
    /// The float value that escaped.
    pub value: f64,
    /// The verified enclosure that does not contain `value`.
    pub enclosure: Interval,
}

/// Returns every float state value that is outside the verified enclosure at the
/// same step.
///
/// `verified` holds the output of [`crate::VectorOdeSolver::solve`] or
/// [`crate::TaylorOdeSolver::solve`]. `float` holds one state per step, in the
/// same order, such as the output of [`float_rk4`]. The two are compared index by
/// index, and the shorter one sets the number of steps checked. A NaN or
/// infinite value always counts as an escape.
///
/// # Example
///
/// ```rust
/// use tpt_fl_exact_math::{Interval, Rational};
/// use tpt_fl_verified_ode::{audit_float_trajectory, float_rk4, VectorOdeSolver};
///
/// // y' = -y from y(0) = 1, integrated exactly and with RK4.
/// let mut verified = VectorOdeSolver::new(
///     |_t: &Interval, y: &[Interval]| vec![-y[0].clone()],
///     Rational::from(0),
///     vec![Rational::from(1)],
/// );
/// let traj = verified.solve(8, &Rational::from_frac(1, 8)).unwrap();
/// let float = float_rk4(|_t, y: &[f64]| vec![-y[0]], 0.0, &[1.0], 0.125, 8);
///
/// assert!(audit_float_trajectory(&traj, &float).is_empty());
/// ```
pub fn audit_float_trajectory(
    verified: &[(Rational, Vec<Interval>)],
    float: &[Vec<f64>],
) -> Vec<Escape> {
    let mut escapes = Vec::new();
    for (step, ((_, enclosures), values)) in verified.iter().zip(float).enumerate() {
        for (component, (enclosure, &value)) in enclosures.iter().zip(values).enumerate() {
            let inside = Rational::from_f64(value).is_some_and(|x| enclosure.contains(&x));
            if !inside {
                escapes.push(Escape {
                    step,
                    component,
                    value,
                    enclosure: enclosure.clone(),
                });
            }
        }
    }
    escapes
}

/// Integrates `y' = f(t, y)` in floating point with the classical fourth-order
/// Runge–Kutta method, using a fixed step `h`.
///
/// Returns the state after each of the `steps` steps. The initial state is not
/// included, so element `i` corresponds to time `t0 + (i + 1)·h`.
///
/// # Panics
///
/// Panics if `f` returns a different number of components than `y0`.
///
/// # Example
///
/// ```rust
/// use tpt_fl_verified_ode::float_rk4;
///
/// // y' = y from y(0) = 1, so y(1) = e.
/// let states = float_rk4(|_t, y: &[f64]| vec![y[0]], 0.0, &[1.0], 0.01, 100);
/// assert!((states[99][0] - core::f64::consts::E).abs() < 1e-8);
/// ```
pub fn float_rk4<F>(f: F, t0: f64, y0: &[f64], h: f64, steps: usize) -> Vec<Vec<f64>>
where
    F: Fn(f64, &[f64]) -> Vec<f64>,
{
    let mut y = y0.to_vec();
    let mut t = t0;
    let mut out = Vec::with_capacity(steps);
    for _ in 0..steps {
        let k1 = slope(&f, t, &y);
        let y2 = offset(&y, &k1, h / 2.0);
        let k2 = slope(&f, t + h / 2.0, &y2);
        let y3 = offset(&y, &k2, h / 2.0);
        let k3 = slope(&f, t + h / 2.0, &y3);
        let y4 = offset(&y, &k3, h);
        let k4 = slope(&f, t + h, &y4);
        for (i, yi) in y.iter_mut().enumerate() {
            *yi += h / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
        }
        t += h;
        out.push(y.clone());
    }
    out
}

/// Evaluates `f` and checks that the result has one entry per state component.
fn slope<F>(f: &F, t: f64, y: &[f64]) -> Vec<f64>
where
    F: Fn(f64, &[f64]) -> Vec<f64>,
{
    let k = f(t, y);
    assert_eq!(
        k.len(),
        y.len(),
        "vector field must return one component per state variable"
    );
    k
}

/// Returns `y + a·k`, componentwise.
fn offset(y: &[f64], k: &[f64], a: f64) -> Vec<f64> {
    y.iter().zip(k).map(|(yi, ki)| yi + a * ki).collect()
}

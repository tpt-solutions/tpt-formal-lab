//! Interval-arithmetic ODE solvers.
//!
//! The scalar [`OdeSolver`], the vector [`VectorOdeSolver`] and the Taylor-model
//! [`TaylorOdeSolver`] share one core that works on a box of intervals, one
//! interval per state component. A scalar system is the one-component case.
//! The methods differ only in how one step is attempted.

use alloc::vec::Vec;
use core::fmt;

use tpt_fl_exact_math::{Interval, Rational};

use crate::taylor::{TaylorFn, TaylorModel};

/// Number of times a step is halved while searching for an enclosure.
const MAX_HALVINGS: u32 = 8;

/// Number of times the starting box is doubled before a step is halved.
const MAX_WIDEN: usize = 8;

/// Number of Picard rounds allowed for each starting box.
const MAX_ROUNDS: usize = 32;

/// Endpoints are rounded outward to multiples of `2^-DYADIC_BITS`, which keeps
/// denominators bounded over long integrations.
pub(crate) const DYADIC_BITS: usize = 64;

/// The right-hand side `f(t, y)` of a scalar ODE, evaluated on interval arguments.
///
/// Implementors return an enclosure of `f` over the box `t × y`: the result
/// must contain `f(s, v)` for every `s ∈ t` and `v ∈ y`. Build it from the
/// interval operators (`+`, `-`, `*`, `/`, unary `-`) and [`Interval::point`]
/// for constants. This is the natural interval extension of `f`. Evaluating
/// `f` only at the corners of the box is unsound for functions that are not
/// monotone in their arguments.
///
/// # Example
///
/// ```rust
/// use tpt_fl_exact_math::{Interval, Rational};
/// use tpt_fl_verified_ode::IntervalFn;
///
/// // y' = y (1 - y)
/// struct Logistic;
/// impl IntervalFn for Logistic {
///     fn eval(&self, _t: &Interval, y: &Interval) -> Interval {
///         let one = Interval::point(Rational::from(1));
///         y.clone() * (one - y.clone())
///     }
/// }
/// ```
pub trait IntervalFn {
    /// Returns an enclosure of `f(t, y)` over the box `t × y`.
    fn eval(&self, t: &Interval, y: &Interval) -> Interval;
}

impl<F: Fn(&Interval, &Interval) -> Interval> IntervalFn for F {
    fn eval(&self, t: &Interval, y: &Interval) -> Interval {
        self(t, y)
    }
}

/// The right-hand side `f(t, y)` of a vector ODE system on interval arguments.
///
/// The state `y` has one interval per component, and the result must have the
/// same length. Each component of the result must enclose the corresponding
/// component of `f(s, v)` for every `s ∈ t` and every `v ∈ y`, using the same
/// rules as [`IntervalFn`]: build it from the interval operators, and do not
/// evaluate only at corners.
///
/// # Example
///
/// ```rust
/// use tpt_fl_exact_math::{Interval, Rational};
/// use tpt_fl_verified_ode::VectorIntervalFn;
///
/// // Harmonic oscillator: y0' = y1, y1' = -y0
/// struct Oscillator;
/// impl VectorIntervalFn for Oscillator {
///     fn eval(&self, _t: &Interval, y: &[Interval]) -> Vec<Interval> {
///         vec![y[1].clone(), -y[0].clone()]
///     }
/// }
/// ```
pub trait VectorIntervalFn {
    /// Returns an enclosure of `f(t, y)` over the box `t × y`, one interval per component.
    fn eval(&self, t: &Interval, y: &[Interval]) -> Vec<Interval>;
}

impl<F: Fn(&Interval, &[Interval]) -> Vec<Interval>> VectorIntervalFn for F {
    fn eval(&self, t: &Interval, y: &[Interval]) -> Vec<Interval> {
        self(t, y)
    }
}

/// Adapts a scalar right-hand side to the vector interface.
struct Scalar<F>(F);

impl<F: IntervalFn> VectorIntervalFn for Scalar<F> {
    fn eval(&self, t: &Interval, y: &[Interval]) -> Vec<Interval> {
        alloc::vec![self.0.eval(t, &y[0])]
    }
}

/// Errors reported by the ODE solvers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OdeError {
    /// The step size was zero or negative.
    NonPositiveStep,
    /// No verified enclosure was found, even after repeatedly halving the step.
    /// The solver's state is unchanged.
    NoEnclosure,
}

impl fmt::Display for OdeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OdeError::NonPositiveStep => f.write_str("step size must be positive"),
            OdeError::NoEnclosure => f.write_str("no verified enclosure found for the step"),
        }
    }
}

/// A verified ODE integrator for `y' = f(t, y)`.
///
/// The solver maintains the current time `t` and an **enclosure** `Y`, a closed
/// interval guaranteed to contain the true state `y(t)`. Each call to
/// [`OdeSolver::step`] advances time by `h` and returns a new enclosure that
/// contains the true solution at `t + h`. If the Picard iteration cannot verify
/// a step of size `h`, the step is split in half and each half is verified in
/// turn. An error is returned only if no verified enclosure is found after the
/// maximum number of halvings.
///
/// # Example
///
/// ```rust
/// use tpt_fl_exact_math::{Interval, Rational};
/// use tpt_fl_verified_ode::OdeSolver;
///
/// let mut solver = OdeSolver::new(
///     |_t: &Interval, y: &Interval| y.clone(),
///     Rational::from(0),
///     Rational::from(1),
/// );
/// let (lo, hi) = solver.step(&Rational::from_frac(1, 10)).unwrap();
/// assert!(lo <= hi);
/// ```
pub struct OdeSolver<F> {
    core: Core<Scalar<F>>,
}

impl<F: IntervalFn> OdeSolver<F> {
    /// Creates a new solver for `y' = f(t, y)` with initial condition `y(t0) = y0`.
    ///
    /// The initial state is the degenerate enclosure `[y0, y0]`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_exact_math::{Interval, Rational};
    /// use tpt_fl_verified_ode::OdeSolver;
    ///
    /// let solver = OdeSolver::new(
    ///     |_t: &Interval, y: &Interval| y.clone(),
    ///     Rational::from(0),
    ///     Rational::from(1),
    /// );
    /// assert_eq!(solver.t(), &Rational::from(0));
    /// ```
    pub fn new(f: F, t0: Rational, y0: Rational) -> Self {
        Self {
            core: Core::new(Scalar(f), t0, alloc::vec![y0], picard_attempt::<Scalar<F>>),
        }
    }

    /// Returns the current time `t`.
    pub fn t(&self) -> &Rational {
        self.core.t()
    }

    /// Returns the current enclosure `[lo, hi]` of the state `y(t)`.
    pub fn y_box(&self) -> &Interval {
        &self.core.y_box()[0]
    }

    /// Returns the midpoint of the current enclosure (a point estimate).
    pub fn y(&self) -> Rational {
        self.y_box().midpoint()
    }

    /// Verifies a single step of size `h` from the current state without
    /// halving and without advancing the solver.
    ///
    /// Returns an enclosure of `y(t + h)`, or [`OdeError::NoEnclosure`] if the
    /// Picard iteration cannot verify the whole step.
    ///
    /// # Errors
    ///
    /// Returns [`OdeError::NonPositiveStep`] if `h <= 0`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_exact_math::{Interval, Rational};
    /// use tpt_fl_verified_ode::OdeSolver;
    ///
    /// let solver = OdeSolver::new(
    ///     |_t: &Interval, y: &Interval| y.clone(),
    ///     Rational::from(0),
    ///     Rational::from(1),
    /// );
    /// let end = solver.a_priori(&Rational::from_frac(1, 4)).unwrap();
    /// assert!(end.contains(&Rational::from_frac(128, 100)));
    /// assert_eq!(solver.t(), &Rational::from(0));
    /// ```
    pub fn a_priori(&self, h: &Rational) -> Result<Interval, OdeError> {
        self.core.a_priori(h).map(first)
    }

    /// Performs a single verified step of size `h`, returning the bounds
    /// `(lo, hi)` of the enclosure at `t + h`.
    ///
    /// If the whole step cannot be verified, it is split in half, up to a fixed
    /// depth. On success the solver advances to `t + h`. On error the solver is
    /// left unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`OdeError::NonPositiveStep`] if `h <= 0`, and
    /// [`OdeError::NoEnclosure`] if no enclosure can be verified.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_exact_math::{Interval, Rational};
    /// use tpt_fl_verified_ode::OdeSolver;
    ///
    /// let mut solver = OdeSolver::new(
    ///     |_t: &Interval, y: &Interval| y.clone(),
    ///     Rational::from(0),
    ///     Rational::from(1),
    /// );
    /// let (lo, hi) = solver.step(&Rational::from_frac(1, 8)).unwrap();
    /// assert!(lo <= hi);
    /// assert_eq!(solver.t(), &Rational::from_frac(1, 8));
    /// ```
    pub fn step(&mut self, h: &Rational) -> Result<(Rational, Rational), OdeError> {
        self.core.step(h).map(|end| bounds(&first(end)))
    }

    /// Integrates forward for `steps` steps of size `h`, returning the sequence
    /// of enclosures `(t_i, lo_i, hi_i)`.
    ///
    /// The first element corresponds to the first step (time `t0 + h`). If a
    /// step fails, the error is returned and the solver keeps the state reached
    /// by the last successful step.
    ///
    /// # Errors
    ///
    /// Returns the first error produced by [`OdeSolver::step`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_exact_math::{Interval, Rational};
    /// use tpt_fl_verified_ode::OdeSolver;
    ///
    /// let mut solver = OdeSolver::new(
    ///     |_t: &Interval, y: &Interval| y.clone(),
    ///     Rational::from(0),
    ///     Rational::from(1),
    /// );
    /// let traj = solver.solve(3, &Rational::from_frac(1, 4)).unwrap();
    /// assert_eq!(traj.len(), 3);
    /// ```
    pub fn solve(
        &mut self,
        steps: usize,
        h: &Rational,
    ) -> Result<Vec<(Rational, Rational, Rational)>, OdeError> {
        Ok(scalar_trajectory(self.core.solve(steps, h)?))
    }

    /// Integrates forward until time `t_end`, using steps of at most `h_max`.
    ///
    /// The last step is shortened so that the solver lands exactly on `t_end`.
    /// Each step is still halved when it cannot be verified, so the step size
    /// adapts to the problem: a stiff stretch uses short steps, and the solver
    /// goes back to `h_max` afterwards. If `t_end` is not after the current
    /// time, nothing is done and the result is empty.
    ///
    /// # Errors
    ///
    /// Returns [`OdeError::NonPositiveStep`] if `h_max <= 0`, and the first
    /// error from [`OdeSolver::step`] otherwise. The solver keeps the state of
    /// the last successful step.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_exact_math::{Interval, Rational};
    /// use tpt_fl_verified_ode::OdeSolver;
    ///
    /// let mut solver = OdeSolver::new(
    ///     |_t: &Interval, y: &Interval| y.clone(),
    ///     Rational::from(0),
    ///     Rational::from(1),
    /// );
    /// let traj = solver
    ///     .solve_until(&Rational::from_frac(1, 3), &Rational::from_frac(1, 4))
    ///     .unwrap();
    /// assert_eq!(traj.len(), 2);
    /// assert_eq!(solver.t(), &Rational::from_frac(1, 3));
    /// ```
    pub fn solve_until(
        &mut self,
        t_end: &Rational,
        h_max: &Rational,
    ) -> Result<Vec<(Rational, Rational, Rational)>, OdeError> {
        Ok(scalar_trajectory(self.core.solve_until(t_end, h_max)?))
    }
}

/// A verified ODE integrator for a vector system `y' = f(t, y)`.
///
/// This is the vector counterpart of [`OdeSolver`]. The state is a box with one
/// interval per component. A step is accepted only when the Picard iteration
/// verifies an enclosure for every component at once, and the step is halved
/// on failure in the same way.
///
/// # Example
///
/// ```rust
/// use tpt_fl_exact_math::{Interval, Rational};
/// use tpt_fl_verified_ode::VectorOdeSolver;
///
/// // Constant slopes: y0' = 2, y1' = -3, from y(0) = (0, 0).
/// let mut solver = VectorOdeSolver::new(
///     |_t: &Interval, _y: &[Interval]| {
///         vec![Interval::point(Rational::from(2)), Interval::point(Rational::from(-3))]
///     },
///     Rational::from(0),
///     vec![Rational::from(0), Rational::from(0)],
/// );
/// let end = solver.step(&Rational::from(1)).unwrap();
/// assert_eq!(end[0].lo(), &Rational::from(2));
/// assert_eq!(end[1].hi(), &Rational::from(-3));
/// ```
pub struct VectorOdeSolver<F> {
    core: Core<F>,
}

impl<F: VectorIntervalFn> VectorOdeSolver<F> {
    /// Creates a new solver for `y' = f(t, y)` with initial condition `y(t0) = y0`.
    ///
    /// The state dimension is `y0.len()`. Every component starts as the
    /// degenerate enclosure `[y0_i, y0_i]`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_exact_math::{Interval, Rational};
    /// use tpt_fl_verified_ode::VectorOdeSolver;
    ///
    /// let solver = VectorOdeSolver::new(
    ///     |_t: &Interval, y: &[Interval]| y.to_vec(),
    ///     Rational::from(0),
    ///     vec![Rational::from(1), Rational::from(2)],
    /// );
    /// assert_eq!(solver.y_box().len(), 2);
    /// ```
    pub fn new(f: F, t0: Rational, y0: Vec<Rational>) -> Self {
        Self {
            core: Core::new(f, t0, y0, picard_attempt::<F>),
        }
    }

    /// Returns the current time `t`.
    pub fn t(&self) -> &Rational {
        self.core.t()
    }

    /// Returns the current enclosure, one interval per component of `y(t)`.
    pub fn y_box(&self) -> &[Interval] {
        self.core.y_box()
    }

    /// Returns the midpoint of each component of the current enclosure.
    pub fn y(&self) -> Vec<Rational> {
        self.core.y_box().iter().map(Interval::midpoint).collect()
    }

    /// Verifies a single step of size `h` from the current state without
    /// halving and without advancing the solver.
    ///
    /// # Errors
    ///
    /// Returns [`OdeError::NonPositiveStep`] if `h <= 0`, and
    /// [`OdeError::NoEnclosure`] if the Picard iteration cannot verify the step.
    ///
    /// # Panics
    ///
    /// Panics if `f` returns a different number of components than the state.
    pub fn a_priori(&self, h: &Rational) -> Result<Vec<Interval>, OdeError> {
        self.core.a_priori(h)
    }

    /// Performs a single verified step of size `h`, returning the enclosure of
    /// every component at `t + h`.
    ///
    /// If the whole step cannot be verified, it is split in half, up to a fixed
    /// depth. On error the solver is left unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`OdeError::NonPositiveStep`] if `h <= 0`, and
    /// [`OdeError::NoEnclosure`] if no enclosure can be verified.
    ///
    /// # Panics
    ///
    /// Panics if `f` returns a different number of components than the state.
    pub fn step(&mut self, h: &Rational) -> Result<Vec<Interval>, OdeError> {
        self.core.step(h)
    }

    /// Integrates forward for `steps` steps of size `h`, returning
    /// `(t_i, enclosure_i)` for each step.
    ///
    /// # Errors
    ///
    /// Returns the first error produced by [`VectorOdeSolver::step`]. The solver
    /// keeps the state reached by the last successful step.
    pub fn solve(
        &mut self,
        steps: usize,
        h: &Rational,
    ) -> Result<Vec<(Rational, Vec<Interval>)>, OdeError> {
        self.core.solve(steps, h)
    }

    /// Integrates forward until time `t_end`, using steps of at most `h_max`.
    ///
    /// The last step is shortened so that the solver lands exactly on `t_end`,
    /// and each step is halved when it cannot be verified. If `t_end` is not
    /// after the current time, nothing is done and the result is empty.
    ///
    /// # Errors
    ///
    /// Returns [`OdeError::NonPositiveStep`] if `h_max <= 0`, and the first
    /// error from [`VectorOdeSolver::step`] otherwise.
    pub fn solve_until(
        &mut self,
        t_end: &Rational,
        h_max: &Rational,
    ) -> Result<Vec<(Rational, Vec<Interval>)>, OdeError> {
        self.core.solve_until(t_end, h_max)
    }
}

/// A verified integrator for a vector system `y' = f(t, y)` that uses Taylor models.
///
/// Each step fixes an a-priori box with the same Picard search as
/// [`VectorOdeSolver`]. It then integrates `f` on Taylor models of the given
/// order, starting from that box, and returns the enclosure at `t + h`. The
/// polynomial part gains one correct term per pass, so for smooth problems the
/// enclosure is usually much tighter than the Picard end point, at the cost of
/// more arithmetic per step.
///
/// The right-hand side is a [`TaylorFn`], written once for all algebras. Use
/// [`VectorOdeSolver`] when the right-hand side is only an interval extension.
///
/// # Example
///
/// ```rust
/// use tpt_fl_exact_math::Rational;
/// use tpt_fl_verified_ode::{TaylorArith, TaylorFn, TaylorOdeSolver};
///
/// // Harmonic oscillator: y0' = y1, y1' = -y0.
/// struct Oscillator;
/// impl TaylorFn for Oscillator {
///     fn eval<A: TaylorArith>(&self, _t: &A, y: &[A]) -> Vec<A> {
///         vec![y[1].clone(), -y[0].clone()]
///     }
/// }
///
/// let mut solver = TaylorOdeSolver::new(
///     Oscillator,
///     4,
///     Rational::from(0),
///     vec![Rational::from(1), Rational::from(0)],
/// );
/// let end = solver.step(&Rational::from_frac(1, 2)).unwrap();
/// // cos(1/2) ≈ 0.8776 lies in the enclosure of y0.
/// assert!(end[0].contains(&Rational::from_frac(8776, 10_000)));
/// ```
pub struct TaylorOdeSolver<F> {
    core: Core<TaylorRhs<F>>,
}

impl<F: TaylorFn> TaylorOdeSolver<F> {
    /// Creates a new solver for `y' = f(t, y)` with initial condition `y(t0) = y0`,
    /// using Taylor models of the given `order`.
    ///
    /// # Panics
    ///
    /// Panics if `order` is zero.
    pub fn new(f: F, order: usize, t0: Rational, y0: Vec<Rational>) -> Self {
        assert!(order >= 1, "Taylor order must be at least 1");
        Self {
            core: Core::new(TaylorRhs { f, order }, t0, y0, taylor_attempt::<F>),
        }
    }

    /// Returns the current time `t`.
    pub fn t(&self) -> &Rational {
        self.core.t()
    }

    /// Returns the current enclosure, one interval per component of `y(t)`.
    pub fn y_box(&self) -> &[Interval] {
        self.core.y_box()
    }

    /// Returns the midpoint of each component of the current enclosure.
    pub fn y(&self) -> Vec<Rational> {
        self.core.y_box().iter().map(Interval::midpoint).collect()
    }

    /// Verifies a single step of size `h` without advancing the solver.
    ///
    /// # Errors
    ///
    /// Returns [`OdeError::NonPositiveStep`] if `h <= 0`, and
    /// [`OdeError::NoEnclosure`] if the a-priori box cannot be verified.
    pub fn a_priori(&self, h: &Rational) -> Result<Vec<Interval>, OdeError> {
        self.core.a_priori(h)
    }

    /// Performs a single verified step of size `h`, returning the enclosure of
    /// every component at `t + h`.
    ///
    /// # Errors
    ///
    /// Returns [`OdeError::NonPositiveStep`] if `h <= 0`, and
    /// [`OdeError::NoEnclosure`] if no enclosure can be verified. On error the
    /// solver is left unchanged.
    pub fn step(&mut self, h: &Rational) -> Result<Vec<Interval>, OdeError> {
        self.core.step(h)
    }

    /// Integrates forward for `steps` steps of size `h`, returning
    /// `(t_i, enclosure_i)` for each step.
    ///
    /// # Errors
    ///
    /// Returns the first error produced by [`TaylorOdeSolver::step`]. The solver
    /// keeps the state reached by the last successful step.
    pub fn solve(
        &mut self,
        steps: usize,
        h: &Rational,
    ) -> Result<Vec<(Rational, Vec<Interval>)>, OdeError> {
        self.core.solve(steps, h)
    }

    /// Integrates forward until time `t_end`, using steps of at most `h_max`.
    ///
    /// # Errors
    ///
    /// Returns [`OdeError::NonPositiveStep`] if `h_max <= 0`, and the first
    /// error from [`TaylorOdeSolver::step`] otherwise.
    pub fn solve_until(
        &mut self,
        t_end: &Rational,
        h_max: &Rational,
    ) -> Result<Vec<(Rational, Vec<Interval>)>, OdeError> {
        self.core.solve_until(t_end, h_max)
    }
}

/// A right-hand side paired with the Taylor order its step uses.
struct TaylorRhs<F> {
    f: F,
    order: usize,
}

impl<F: TaylorFn> VectorIntervalFn for TaylorRhs<F> {
    fn eval(&self, t: &Interval, y: &[Interval]) -> Vec<Interval> {
        self.f.eval(t, y)
    }
}

/// The signature of one attempted step: given the right-hand side, the current
/// time, the state and the step size, return the enclosure at `t + h` if it can
/// be verified.
type Attempt<F> = fn(&F, &Rational, &[Interval], &Rational) -> Option<Vec<Interval>>;

/// The shared integrator. Every state component is an interval, and a step is
/// verified for the whole box at once.
struct Core<F> {
    f: F,
    t: Rational,
    y: Vec<Interval>,
    attempt: Attempt<F>,
}

impl<F> Core<F> {
    fn new(f: F, t0: Rational, y0: Vec<Rational>, attempt: Attempt<F>) -> Self {
        Self {
            f,
            t: t0,
            y: y0.into_iter().map(Interval::point).collect(),
            attempt,
        }
    }

    fn t(&self) -> &Rational {
        &self.t
    }

    fn y_box(&self) -> &[Interval] {
        &self.y
    }

    fn a_priori(&self, h: &Rational) -> Result<Vec<Interval>, OdeError> {
        check_step(h)?;
        (self.attempt)(&self.f, &self.t, &self.y, h).ok_or(OdeError::NoEnclosure)
    }

    /// Advances the state from `t` by `h`, halving the step when a whole-step
    /// enclosure cannot be verified. Returns the enclosure at `t + h`.
    fn advance(
        &self,
        t: &Rational,
        y: &[Interval],
        h: &Rational,
        depth: u32,
    ) -> Option<Vec<Interval>> {
        if let Some(end) = (self.attempt)(&self.f, t, y, h) {
            return Some(end);
        }
        if depth == MAX_HALVINGS {
            return None;
        }
        let half = h.clone() / Rational::from(2);
        let mid = self.advance(t, y, &half, depth + 1)?;
        self.advance(&(t.clone() + half.clone()), &mid, &half, depth + 1)
    }

    fn step(&mut self, h: &Rational) -> Result<Vec<Interval>, OdeError> {
        check_step(h)?;
        let end = self
            .advance(&self.t, &self.y, h, 0)
            .ok_or(OdeError::NoEnclosure)?;
        self.t = self.t.clone() + h.clone();
        self.y = end.clone();
        Ok(end)
    }

    fn solve(
        &mut self,
        steps: usize,
        h: &Rational,
    ) -> Result<Vec<(Rational, Vec<Interval>)>, OdeError> {
        let mut out = Vec::with_capacity(steps);
        for _ in 0..steps {
            self.step(h)?;
            out.push((self.t.clone(), self.y.clone()));
        }
        Ok(out)
    }

    fn solve_until(
        &mut self,
        t_end: &Rational,
        h_max: &Rational,
    ) -> Result<Vec<(Rational, Vec<Interval>)>, OdeError> {
        check_step(h_max)?;
        let mut out = Vec::new();
        while self.t < *t_end {
            let remaining = t_end.clone() - self.t.clone();
            let h = if remaining < *h_max {
                remaining
            } else {
                h_max.clone()
            };
            self.step(&h)?;
            out.push((self.t.clone(), self.y.clone()));
        }
        Ok(out)
    }
}

/// One Picard-verified step of the vector right-hand side `f`.
fn picard_attempt<F: VectorIntervalFn>(
    f: &F,
    t: &Rational,
    y0: &[Interval],
    h: &Rational,
) -> Option<Vec<Interval>> {
    verify_step(&|tb: &Interval, yb: &[Interval]| f.eval(tb, yb), t, y0, h).map(|(_, end)| end)
}

/// One Taylor-model step of order `w.order`.
///
/// The a-priori box `B` from the Picard search bounds the solution over the
/// step. Each pass integrates `f` on the current models, which gives one more
/// correct Taylor term. A pass keeps the models valid because `f` is evaluated
/// on models that contain the solution. The enclosure at `t + h` is the value
/// of the final models at `s = h`.
fn taylor_attempt<F: TaylorFn>(
    w: &TaylorRhs<F>,
    t: &Rational,
    y0: &[Interval],
    h: &Rational,
) -> Option<Vec<Interval>> {
    let rhs = |tb: &Interval, yb: &[Interval]| w.f.eval(tb, yb);
    let (b, _) = verify_step(&rhs, t, y0, h)?;

    let dom = Interval::new(Rational::zero(), h.clone());
    let order = w.order;
    // t = t0 + s, so the time model is exact.
    let t_model = TaylorModel::new(
        alloc::vec![Interval::point(t.clone()), Interval::point(Rational::one())],
        Interval::point(Rational::zero()),
        dom.clone(),
        order,
    );
    // The solution is y0 + (y − y0), and y − y0 lies in B − y0 over the domain.
    let mut y: Vec<TaylorModel> = y0
        .iter()
        .zip(&b)
        .map(|(y0_i, b_i)| {
            TaylorModel::new(
                alloc::vec![y0_i.clone()],
                b_i.clone() - y0_i.clone(),
                dom.clone(),
                order,
            )
        })
        .collect();
    for _ in 0..order {
        let fy = w.f.eval(&t_model, &y);
        assert_eq!(
            fy.len(),
            y0.len(),
            "vector field must return one component per state variable"
        );
        y = fy
            .into_iter()
            .zip(y0)
            .map(|(fi, y0_i)| fi.integrate() + constant_model(&dom, order, y0_i))
            .collect();
    }
    Some(y.iter().map(TaylorModel::at_end).collect())
}

/// The constant model `c`, on `dom` with the given order.
fn constant_model(dom: &Interval, order: usize, c: &Interval) -> TaylorModel {
    TaylorModel::new(
        alloc::vec![c.clone()],
        Interval::point(Rational::zero()),
        dom.clone(),
        order,
    )
}

/// Searches for an a priori box `B` with `hull(y0, y0 + h·f(T, B)) ⊆ B`,
/// where `T = [t, t + h]`. When the search succeeds, the true solution
/// stays in `B` over the step, and `y0 + h·f(T, B)` encloses `y(t + h)`.
///
/// Returns `(B, end)`.
fn verify_step(
    rhs: &impl Fn(&Interval, &[Interval]) -> Vec<Interval>,
    t: &Rational,
    y0: &[Interval],
    h: &Rational,
) -> Option<(Vec<Interval>, Vec<Interval>)> {
    let t_box = Interval::new(t.clone(), t.clone() + h.clone());
    // Start from the Euler guess and widen it until the iteration contracts.
    let (_, seed) = picard_map(rhs, &t_box, y0, h, y0);
    let mut radius = Rational::one() + max_width(&seed);
    for _ in 0..MAX_WIDEN {
        let mut b = inflate(&seed, &radius);
        let mut verified = None;
        for _ in 0..MAX_ROUNDS {
            let (end, next) = picard_map(rhs, &t_box, y0, h, &b);
            if contains_all(&b, &next) {
                // `b` is invariant, so `end` encloses y(t + h). `next` is then
                // invariant too and may give a tighter end point: keep going.
                verified = Some((b.clone(), end));
                if next == b {
                    break;
                }
                b = next;
            } else if verified.is_some() {
                // Tightening stopped; the last verified end point stands.
                break;
            } else {
                // A diverging iterate (e.g. near a finite-time blow-up) only
                // grows, and its magnitudes square each round. Give up on
                // this start box and let the outer loop try a wider one.
                if max_width(&next) > max_width(&b) {
                    break;
                }
                b = next;
            }
        }
        if verified.is_some() {
            return verified;
        }
        radius = radius.clone() + radius;
    }
    None
}

/// Evaluates one Picard operator `B ↦ y0 + h·f(T, B)`, componentwise.
///
/// Returns `(end, next)`, where `end = y0 + h·f(T, B)` encloses the state
/// at the end of the step and `next = hull(y0, end)` encloses the state
/// over the whole step. Because `[0, h]·F = hull({0, h·F})`, the hull is the
/// set `y0 + [0, h]·F` that the state can sweep through.
///
/// Both results are rounded outward. Without rounding, each iteration
/// squares the bit-length of the denominators, so the iteration never
/// finishes in practice on nonlinear right-hand sides.
///
/// # Panics
///
/// Panics if `rhs` does not return one component per state variable.
fn picard_map(
    rhs: &impl Fn(&Interval, &[Interval]) -> Vec<Interval>,
    t_box: &Interval,
    y0: &[Interval],
    h: &Rational,
    b: &[Interval],
) -> (Vec<Interval>, Vec<Interval>) {
    let fb = rhs(t_box, b);
    assert_eq!(
        fb.len(),
        y0.len(),
        "vector field must return one component per state variable"
    );
    let end: Vec<Interval> = y0
        .iter()
        .zip(fb)
        .map(|(y0_i, f_i)| {
            (y0_i.clone() + Interval::point(h.clone()) * f_i).round_outward(DYADIC_BITS)
        })
        .collect();
    let next = y0
        .iter()
        .zip(&end)
        .map(|(y0_i, end_i)| y0_i.hull(end_i).round_outward(DYADIC_BITS))
        .collect();
    (end, next)
}

/// Returns the single component of a scalar state.
fn first(mut y: Vec<Interval>) -> Interval {
    y.swap_remove(0)
}

fn bounds(y: &Interval) -> (Rational, Rational) {
    (y.lo().clone(), y.hi().clone())
}

fn scalar_trajectory(traj: Vec<(Rational, Vec<Interval>)>) -> Vec<(Rational, Rational, Rational)> {
    traj.into_iter()
        .map(|(t, y)| {
            let (lo, hi) = bounds(&y[0]);
            (t, lo, hi)
        })
        .collect()
}

fn check_step(h: &Rational) -> Result<(), OdeError> {
    if h.is_positive() {
        Ok(())
    } else {
        Err(OdeError::NonPositiveStep)
    }
}

/// The largest width over all components. Zero for an empty box.
fn max_width(b: &[Interval]) -> Rational {
    b.iter()
        .map(Interval::width)
        .fold(Rational::zero(), |m, w| if w > m { w } else { m })
}

/// Whether every component of `outer` contains the matching component of `inner`.
fn contains_all(outer: &[Interval], inner: &[Interval]) -> bool {
    outer.iter().zip(inner).all(|(o, i)| o.contains_interval(i))
}

/// Widens every component of `b` by `radius` on both sides.
fn inflate(b: &[Interval], radius: &Rational) -> Vec<Interval> {
    b.iter()
        .map(|c| {
            Interval::new(
                c.lo().clone() - radius.clone(),
                c.hi().clone() + radius.clone(),
            )
        })
        .collect()
}

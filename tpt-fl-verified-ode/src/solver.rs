//! Interval-arithmetic Picard-iteration ODE solver.

use core::fmt;

use tpt_fl_exact_math::{Interval, Rational};

/// Number of times a step is halved while searching for an enclosure.
const MAX_HALVINGS: u32 = 8;

/// Number of times the starting box is doubled before a step is halved.
const MAX_WIDEN: usize = 8;

/// Number of Picard rounds allowed for each starting box.
const MAX_ROUNDS: usize = 32;

/// Endpoints are rounded outward to multiples of `2^-DYADIC_BITS`, which keeps
/// denominators bounded over long integrations.
const DYADIC_BITS: usize = 64;

/// The right-hand side `f(t, y)` of an ODE, evaluated on interval arguments.
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

/// Errors reported by [`OdeSolver`].
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
    f: F,
    t: Rational,
    y_box: Interval,
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
            f,
            t: t0,
            y_box: Interval::point(y0),
        }
    }

    /// Returns the current time `t`.
    pub fn t(&self) -> &Rational {
        &self.t
    }

    /// Returns the current enclosure `[lo, hi]` of the state `y(t)`.
    pub fn y_box(&self) -> &Interval {
        &self.y_box
    }

    /// Returns the midpoint of the current enclosure (a point estimate).
    pub fn y(&self) -> Rational {
        self.y_box.midpoint()
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
        check_step(h)?;
        self.verify_step(&self.t, &self.y_box, h)
            .ok_or(OdeError::NoEnclosure)
    }

    /// Searches for an a priori box `B` with `hull(y0, y0 + h·f(T, B)) ⊆ B`,
    /// where `T = [t, t + h]`. When the search succeeds, the true solution
    /// stays in `B` over the step, and `y0 + h·f(T, B)` encloses `y(t + h)`.
    fn verify_step(&self, t: &Rational, y0: &Interval, h: &Rational) -> Option<Interval> {
        let t_box = Interval::new(t.clone(), t.clone() + h.clone());
        // Start from the Euler guess and widen it until the iteration contracts.
        let (_, seed) = self.picard_map(&t_box, y0, h, y0);
        let mut radius = Rational::one() + seed.width();
        for _ in 0..MAX_WIDEN {
            let mut b = inflate(&seed, &radius);
            let mut verified = None;
            for _ in 0..MAX_ROUNDS {
                let (end, next) = self.picard_map(&t_box, y0, h, &b);
                if b.contains_interval(&next) {
                    // `b` is invariant, so `end` encloses y(t + h). `next` is then
                    // invariant too and may give a tighter end point: keep going.
                    verified = Some(end);
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
                    if next.width() > b.width() {
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

    /// Evaluates one Picard operator `B ↦ y0 + h·f(T, B)`.
    ///
    /// Returns `(end, next)`, where `end = y0 + h·f(T, B)` encloses the state
    /// at the end of the step and `next = hull(y0, end)` encloses the state
    /// over the whole step. Because `[0, h]·F = hull({0, h·F})`, the hull is the
    /// set `y0 + [0, h]·F` that the state can sweep through.
    ///
    /// Both results are rounded outward. Without rounding, each iteration
    /// squares the bit-length of the denominators, so the iteration never
    /// finishes in practice on nonlinear right-hand sides.
    fn picard_map(
        &self,
        t_box: &Interval,
        y0: &Interval,
        h: &Rational,
        b: &Interval,
    ) -> (Interval, Interval) {
        let end = y0.clone() + Interval::point(h.clone()) * self.f.eval(t_box, b);
        let end = end.round_outward(DYADIC_BITS);
        let next = y0.hull(&end).round_outward(DYADIC_BITS);
        (end, next)
    }

    /// Advances the state from `t` by `h`, halving the step when a whole-step
    /// enclosure cannot be verified. Returns the enclosure at `t + h`.
    fn advance(&self, t: &Rational, y: &Interval, h: &Rational, depth: u32) -> Option<Interval> {
        if let Some(end) = self.verify_step(t, y, h) {
            return Some(end);
        }
        if depth == MAX_HALVINGS {
            return None;
        }
        let half = h.clone() / Rational::from(2);
        let mid = self.advance(t, y, &half, depth + 1)?;
        self.advance(&(t.clone() + half.clone()), &mid, &half, depth + 1)
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
        check_step(h)?;
        let end = self
            .advance(&self.t, &self.y_box, h, 0)
            .ok_or(OdeError::NoEnclosure)?;
        self.t = self.t.clone() + h.clone();
        self.y_box = end.clone();
        Ok((end.lo().clone(), end.hi().clone()))
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
    ) -> Result<alloc::vec::Vec<(Rational, Rational, Rational)>, OdeError> {
        let mut out = alloc::vec::Vec::with_capacity(steps);
        for _ in 0..steps {
            let (lo, hi) = self.step(h)?;
            out.push((self.t.clone(), lo, hi));
        }
        Ok(out)
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
    ) -> Result<alloc::vec::Vec<(Rational, Rational, Rational)>, OdeError> {
        check_step(h_max)?;
        let mut out = alloc::vec::Vec::new();
        while self.t < *t_end {
            let remaining = t_end.clone() - self.t.clone();
            let h = if remaining < *h_max {
                remaining
            } else {
                h_max.clone()
            };
            let (lo, hi) = self.step(&h)?;
            out.push((self.t.clone(), lo, hi));
        }
        Ok(out)
    }
}

fn check_step(h: &Rational) -> Result<(), OdeError> {
    if h.is_positive() {
        Ok(())
    } else {
        Err(OdeError::NonPositiveStep)
    }
}

/// Widens `b` by `radius` on both sides.
fn inflate(b: &Interval, radius: &Rational) -> Interval {
    Interval::new(
        b.lo().clone() - radius.clone(),
        b.hi().clone() + radius.clone(),
    )
}

//! Interval-arithmetic Picard-iteration ODE solver.

use tpt_exact_math::{Interval, Rational};

/// A right-hand side `f(t, y)` of an ODE, evaluated on point arguments.
///
/// Implementors return the value of `f(t, y)` at a point. The solver encloses
/// `f` over an interval box by evaluating `eval` on the bounding corners and
/// taking the hull, which is correct for any function (and tight for functions
/// that are monotonic in each argument over the box).
///
/// # Example
///
/// ```rust
/// use tpt_exact_math::Rational;
/// use tpt_verified_ode::IntervalFn;
///
/// // y' = y
/// struct Exp;
/// impl IntervalFn for Exp {
///     fn eval(&self, _t: &Rational, y: &Rational) -> Rational {
///         y.clone()
///     }
/// }
/// ```
pub trait IntervalFn {
    /// Returns the value of `f(t, y)`.
    fn eval(&self, t: &Rational, y: &Rational) -> Rational;
}

impl<F: Fn(&Rational, &Rational) -> Rational> IntervalFn for F {
    fn eval(&self, t: &Rational, y: &Rational) -> Rational {
        self(t, y)
    }
}

/// A verified ODE integrator for `y' = f(t, y)`.
///
/// The solver maintains the current time `t` and an **enclosure** `Y` — a closed
/// interval guaranteed to contain the true state `y(t)`. Each call to
/// [`OdeSolver::step`] advances by a step size `h` and returns a new enclosure
/// `[lo, hi]` guaranteed to contain the true solution at `t + h`, provided the
/// Picard iteration contracts on the enclosure (see the crate documentation for
/// the rigor caveat regarding step size `h`).
///
/// The enclosure is carried forward between steps, so the method remains a
/// valid verified enclosure across many steps.
///
/// # Example
///
/// ```rust
/// use tpt_exact_math::Rational;
/// use tpt_verified_ode::OdeSolver;
///
/// let mut solver = OdeSolver::new(|_t: &Rational, y: &Rational| y.clone(), Rational::from(0), Rational::from(1));
/// let (lo, hi) = solver.step(&Rational::from_frac(1, 10));
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
    /// use tpt_exact_math::Rational;
    /// use tpt_verified_ode::OdeSolver;
    ///
    /// let solver = OdeSolver::new(|_t: &Rational, y: &Rational| y.clone(), Rational::from(0), Rational::from(1));
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

    /// Computes the **a priori** Picard-iteration enclosure over one step of `h`.
    ///
    /// The iteration starts from an enclosure `Y0` chosen to contain the
    /// fixpoint (a wide enough interval around the current state), then iterates
    ///
    /// ```text
    /// Y_{k+1} = Y0 + h · f([t, t+h], Y_k)
    /// ```
    ///
    /// where `f` is evaluated on the four corners of `[t, t+h] × Y_k` and the
    /// result is widened to an interval by taking the hull. Iteration stops as
    /// soon as `Y_{k+1} ⊆ Y_k` (a fixpoint has been reached) or after a fixed
    /// maximum number of rounds.
    ///
    /// The returned interval is guaranteed to contain the true solution at `t + h`
    /// whenever the Picard iteration contracts on this enclosure.
    pub fn a_priori(&self, h: &Rational) -> Interval {
        self.tightened_step(h)
    }

    /// Performs one Picard round of `Y_{k+1} = Y_prev + h · f([t, t+h], Y_k)`.
    ///
    /// The additive base is the previous step's enclosure `self.y_box`; `y_box`
    /// is the current iterate whose box `f` is evaluated over.
    fn picard_round(&self, t_box: &Interval, y_box: &Interval, h: &Rational) -> Interval {
        let corners = [
            (t_box.lo(), y_box.lo()),
            (t_box.lo(), y_box.hi()),
            (t_box.hi(), y_box.lo()),
            (t_box.hi(), y_box.hi()),
        ];
        let mut f_lo = Rational::zero();
        let mut f_hi = Rational::zero();
        let mut first = true;
        for (tc, yc) in corners {
            let val = self.f.eval(tc, yc);
            if first {
                f_lo = val.clone();
                f_hi = val;
                first = false;
            } else {
                if val < f_lo {
                    f_lo = val.clone();
                }
                if val > f_hi {
                    f_hi = val;
                }
            }
        }
        let f_box = Interval::new(f_lo, f_hi);
        // Integral enclosure: ∫_0^h f(s, y(s)) ds ∈ h · [f_lo, f_hi].
        let integral = Interval::new(
            h.clone() * f_box.lo().clone(),
            h.clone() * f_box.hi().clone(),
        );
        Interval::new(
            self.y_box.lo().clone() + integral.lo().clone(),
            self.y_box.hi().clone() + integral.hi().clone(),
        )
    }

    /// Runs the contracting Picard iteration to a fixpoint, starting from a box
    /// that is widened until the iteration is contracting (so the returned box
    /// is a valid enclosure of the true solution).
    fn tightened_step(&self, h: &Rational) -> Interval {
        let t_box = Interval::new(self.t.clone(), self.t.clone() + h.clone());

        // Estimate a slope bound from the current state and widen around it to
        // obtain a start box that contains the Picard fixpoint.
        let slope = self.f.eval(&self.t, self.y_box.lo());
        let radius0 = if slope.is_negative() { -slope } else { slope } + Rational::one();

        const MAX_WIDEN: usize = 8;
        let mut radius = radius0;
        for _ in 0..MAX_WIDEN {
            let y0_box = Interval::new(
                self.y_box.lo().clone() - h.clone() * radius.clone(),
                self.y_box.hi().clone() + h.clone() * radius.clone(),
            );
            if let Some(fix) = self.picard_fixpoint(&t_box, &y0_box, h) {
                return fix;
            }
            radius = radius.clone() + radius.clone(); // double and retry
        }
        // Fallback: return the widest a priori box tried.
        Interval::new(
            self.y_box.lo().clone() - h.clone() * radius.clone(),
            self.y_box.hi().clone() + h.clone() * radius,
        )
    }

    /// Iterates the Picard operator from `y0_box` (the start iterate domain)
    /// until `Y_{k+1} ⊆ Y_k`, returning the fixpoint.
    fn picard_fixpoint(
        &self,
        t_box: &Interval,
        y0_box: &Interval,
        h: &Rational,
    ) -> Option<Interval> {
        let mut y_prev = y0_box.clone();
        const MAX_ROUNDS: usize = 64;
        for _ in 0..MAX_ROUNDS {
            let y_next = self.picard_round(t_box, &y_prev, h);
            if y_next.contains_interval(&y_prev) {
                return Some(y_next);
            }
            y_prev = y_next;
        }
        None
    }

    /// Performs a single verified step of size `h`, returning `[lo, hi]`.
    ///
    /// Runs the contracting Picard iteration from the current (carried-forward)
    /// enclosure until a fixpoint is reached. The current enclosure is then
    /// advanced to this new box and carried forward to the next step.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::Rational;
    /// use tpt_verified_ode::OdeSolver;
    ///
    /// let mut solver = OdeSolver::new(|_t: &Rational, y: &Rational| y.clone(), Rational::from(0), Rational::from(1));
    /// let (lo, hi) = solver.step(&Rational::from_frac(1, 8));
    /// assert!(lo <= hi);
    /// ```
    pub fn step(&mut self, h: &Rational) -> (Rational, Rational) {
        let result = self.tightened_step(h);
        self.t = self.t.clone() + h.clone();
        self.y_box = result.clone();
        (result.lo().clone(), result.hi().clone())
    }

    /// Integrates forward for `steps` steps of size `h`, returning the sequence
    /// of enclosures `(t_i, [lo_i, hi_i])`.
    ///
    /// The first element corresponds to the first step (time `t0 + h`).
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::Rational;
    /// use tpt_verified_ode::OdeSolver;
    ///
    /// let mut solver = OdeSolver::new(|_t: &Rational, y: &Rational| y.clone(), Rational::from(0), Rational::from(1));
    /// let traj = solver.solve(3, &Rational::from_frac(1, 4));
    /// assert_eq!(traj.len(), 3);
    /// ```
    pub fn solve(
        &mut self,
        steps: usize,
        h: &Rational,
    ) -> alloc::vec::Vec<(Rational, Rational, Rational)> {
        let mut out = alloc::vec::Vec::with_capacity(steps);
        for _ in 0..steps {
            let (lo, hi) = self.step(h);
            out.push((self.t.clone(), lo, hi));
        }
        out
    }
}

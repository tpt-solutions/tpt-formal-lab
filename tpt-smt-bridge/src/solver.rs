//! [`SmtSolver`] — fluent builder for SMT-LIB2 problems.

use crate::expr::Expr;
use crate::smtlib2::emit_smtlib2;
use crate::sort::Sort;

/// A builder that accumulates declarations and assertions, then emits a
/// complete SMT-LIB2 problem string.
///
/// # Example
///
/// ```rust
/// use tpt_smt_bridge::{SmtSolver, Sort, Expr};
///
/// let mut solver = SmtSolver::new();
/// solver.declare_const("n", Sort::Int);
/// solver.assert(Expr::gt(Expr::var("n", Sort::Int), Expr::int(0)));
/// solver.assert(Expr::lt(Expr::var("n", Sort::Int), Expr::int(10)));
///
/// let problem = solver.emit_check();
/// assert!(problem.contains("(declare-const n Int)"));
/// assert!(problem.contains("(check-sat)"));
/// ```
pub struct SmtSolver {
    declarations: Vec<String>,
    assertions: Vec<Expr>,
    logic: Option<String>,
}

impl SmtSolver {
    /// Creates a new, empty solver.
    pub fn new() -> Self {
        Self {
            declarations: Vec::new(),
            assertions: Vec::new(),
            logic: None,
        }
    }

    /// Sets the SMT-LIB2 logic (e.g. `"QF_LIA"`, `"LRA"`, `"AUFLIA"`).
    ///
    /// If set, emits `(set-logic <logic>)` at the top of the problem.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_smt_bridge::SmtSolver;
    ///
    /// let mut s = SmtSolver::new();
    /// s.set_logic("QF_LIA");
    /// assert!(s.emit_check().starts_with("(set-logic QF_LIA)"));
    /// ```
    pub fn set_logic(&mut self, logic: impl Into<String>) -> &mut Self {
        self.logic = Some(logic.into());
        self
    }

    /// Declares a constant with the given name and sort.
    ///
    /// Emits `(declare-const name Sort)` in the output.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_smt_bridge::{SmtSolver, Sort};
    ///
    /// let mut s = SmtSolver::new();
    /// s.declare_const("x", Sort::Int);
    /// assert!(s.emit_smtlib2().contains("(declare-const x Int)"));
    /// ```
    pub fn declare_const(&mut self, name: impl Into<String>, sort: Sort) -> &mut Self {
        self.declarations.push(format!(
            "(declare-const {} {})",
            name.into(),
            sort.to_smtlib2()
        ));
        self
    }

    /// Declares an uninterpreted function.
    ///
    /// Emits `(declare-fun name (arg_sorts...) return_sort)`.
    pub fn declare_fun(
        &mut self,
        name: impl Into<String>,
        arg_sorts: &[Sort],
        return_sort: Sort,
    ) -> &mut Self {
        let args: Vec<String> = arg_sorts.iter().map(|s| s.to_smtlib2()).collect();
        self.declarations.push(format!(
            "(declare-fun {} ({}) {})",
            name.into(),
            args.join(" "),
            return_sort.to_smtlib2()
        ));
        self
    }

    /// Adds an assertion to the problem.
    ///
    /// Emits `(assert <expr>)` in the output.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_smt_bridge::{SmtSolver, Sort, Expr};
    ///
    /// let mut s = SmtSolver::new();
    /// s.declare_const("x", Sort::Int);
    /// s.assert(Expr::gt(Expr::var("x", Sort::Int), Expr::int(0)));
    /// assert!(s.emit_smtlib2().contains("(assert (> x 0))"));
    /// ```
    pub fn assert(&mut self, expr: Expr) -> &mut Self {
        self.assertions.push(expr);
        self
    }

    /// Emits the problem as an SMT-LIB2 string without `(check-sat)`.
    ///
    /// Useful when you want to append your own commands at the end.
    pub fn emit_smtlib2(&self) -> String {
        self.build(false)
    }

    /// Emits the complete problem with `(check-sat)` and `(get-model)` appended.
    ///
    /// This is the most common form — pipe the result to a solver binary.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_smt_bridge::{SmtSolver, Sort, Expr};
    ///
    /// let mut s = SmtSolver::new();
    /// s.declare_const("x", Sort::Int);
    /// s.assert(Expr::gt(Expr::var("x", Sort::Int), Expr::int(0)));
    /// let problem = s.emit_check();
    /// assert!(problem.ends_with("(check-sat)\n(get-model)\n"));
    /// ```
    pub fn emit_check(&self) -> String {
        self.build(true)
    }

    fn build(&self, include_check: bool) -> String {
        let mut out = String::new();

        if let Some(logic) = &self.logic {
            out.push_str(&format!("(set-logic {logic})\n"));
        }

        for decl in &self.declarations {
            out.push_str(decl);
            out.push('\n');
        }

        for expr in &self.assertions {
            out.push_str(&format!("(assert {})\n", emit_smtlib2(expr)));
        }

        if include_check {
            out.push_str("(check-sat)\n");
            out.push_str("(get-model)\n");
        }

        out
    }
}

impl Default for SmtSolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Expr;

    #[test]
    fn empty_with_check() {
        let s = SmtSolver::new();
        assert_eq!(s.emit_check(), "(check-sat)\n(get-model)\n");
    }

    #[test]
    fn declare_and_assert() {
        let mut s = SmtSolver::new();
        s.declare_const("x", Sort::Int);
        s.assert(Expr::gt(Expr::var("x", Sort::Int), Expr::int(0)));
        let out = s.emit_smtlib2();
        assert!(out.contains("(declare-const x Int)"), "missing declaration");
        assert!(out.contains("(assert (> x 0))"), "missing assertion");
    }

    #[test]
    fn logic_prefix() {
        let mut s = SmtSolver::new();
        s.set_logic("QF_LIA");
        let out = s.emit_check();
        assert!(out.starts_with("(set-logic QF_LIA)\n"));
    }

    #[test]
    fn full_problem() {
        let mut s = SmtSolver::new();
        s.set_logic("QF_LIA");
        s.declare_const("x", Sort::Int);
        s.declare_const("y", Sort::Int);
        s.assert(Expr::gt(Expr::var("x", Sort::Int), Expr::int(0)));
        s.assert(Expr::gt(Expr::var("y", Sort::Int), Expr::int(0)));
        s.assert(Expr::eq(
            Expr::add(Expr::var("x", Sort::Int), Expr::var("y", Sort::Int)),
            Expr::int(10),
        ));
        let out = s.emit_check();
        assert!(out.contains("(declare-const x Int)"));
        assert!(out.contains("(declare-const y Int)"));
        assert!(out.contains("(assert (> x 0))"));
        assert!(out.contains("(assert (+ x y))") || out.contains("(assert (= (+ x y) 10))"));
        assert!(out.ends_with("(check-sat)\n(get-model)\n"));
    }
}

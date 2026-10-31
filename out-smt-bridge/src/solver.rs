//! [`SmtSolver`] — fluent builder for SMT-LIB2 problems.

use std::fmt;
use std::time::Duration;

use crate::expr::{Expr, ExprInner};
use crate::result::{self, Backend, SatResult, SolverError};
use crate::smtlib2::{emit_smtlib2, is_simple_symbol, quote_symbol};
use crate::sort::Sort;

/// A builder that accumulates commands and assertions, then emits a complete
/// SMT-LIB2 problem string.
///
/// Commands are kept in the order they were added, so `push` and `pop` scope
/// the declarations and assertions that follow them.
///
/// # Example
///
/// ```rust
/// use out_smt_bridge::{SmtSolver, Sort, Expr};
///
/// let mut solver = SmtSolver::new();
/// solver.declare_const("n", Sort::Int);
/// solver.assert(Expr::gt(Expr::var("n", Sort::Int), Expr::int(0)));
/// solver.assert(Expr::lt(Expr::var("n", Sort::Int), Expr::int(10)));
///
/// let problem = solver.emit_check();
/// assert!(problem.contains("(declare-const n Int)"));
/// assert!(problem.ends_with("(check-sat)\n"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct SmtSolver {
    logic: Option<String>,
    commands: Vec<Command>,
}

#[derive(Debug, Clone)]
enum Command {
    SetOption(String, String),
    DeclareConst(String, Sort),
    DeclareFun(String, Vec<Sort>, Sort),
    Assert(Expr),
    AssertNamed(String, Expr),
    Push,
    Pop,
}

/// Which check commands to append after the problem.
#[derive(Clone)]
enum Check {
    None,
    Sat,
    Model,
    Core,
    /// `(check-sat)` followed by `(get-value (...))` for these terms, printed.
    Values(Vec<String>),
}

/// An error found by [`SmtSolver::sort_of`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortError(String);

impl fmt::Display for SortError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SortError {}

impl SmtSolver {
    /// Creates a new, empty solver.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the SMT-LIB2 logic (e.g. `"QF_LIA"`, `"LRA"`, `"AUFLIA"`).
    ///
    /// If set, emits `(set-logic <logic>)` at the top of the problem.
    ///
    /// # Panics
    ///
    /// Panics unless `logic` is a simple SMT-LIB2 symbol made of letters,
    /// digits and the characters `~!@$%^&*_-+=<>.?/`. Anything else could
    /// inject extra commands into the problem.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::SmtSolver;
    ///
    /// let mut s = SmtSolver::new();
    /// s.set_logic("QF_LIA");
    /// assert!(s.emit_check().starts_with("(set-logic QF_LIA)"));
    /// ```
    pub fn set_logic(&mut self, logic: impl Into<String>) -> &mut Self {
        let logic = logic.into();
        assert!(
            is_simple_symbol(&logic),
            "logic `{logic}` is not a simple SMT-LIB2 symbol"
        );
        self.logic = Some(logic);
        self
    }

    /// Sets a solver option, emitting `(set-option :name value)`.
    ///
    /// `value` is written as given, so pass `"true"` or `"false"` for flags.
    ///
    /// # Panics
    ///
    /// Panics if `name` is not a simple symbol.
    pub fn set_option(&mut self, name: impl Into<String>, value: impl Into<String>) -> &mut Self {
        let name = name.into();
        assert!(
            is_simple_symbol(&name),
            "option `{name}` is not a simple symbol"
        );
        self.commands.push(Command::SetOption(name, value.into()));
        self
    }

    /// Declares a constant with the given name and sort.
    ///
    /// Emits `(declare-const name Sort)` in the output. Names that are not
    /// simple symbols are written with `|...|`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::{SmtSolver, Sort};
    ///
    /// let mut s = SmtSolver::new();
    /// s.declare_const("x", Sort::Int);
    /// assert!(s.emit_smtlib2().contains("(declare-const x Int)"));
    /// ```
    pub fn declare_const(&mut self, name: impl Into<String>, sort: Sort) -> &mut Self {
        self.commands.push(Command::DeclareConst(name.into(), sort));
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
        self.commands.push(Command::DeclareFun(
            name.into(),
            arg_sorts.to_vec(),
            return_sort,
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
    /// use out_smt_bridge::{SmtSolver, Sort, Expr};
    ///
    /// let mut s = SmtSolver::new();
    /// s.declare_const("x", Sort::Int);
    /// s.assert(Expr::gt(Expr::var("x", Sort::Int), Expr::int(0)));
    /// assert!(s.emit_smtlib2().contains("(assert (> x 0))"));
    /// ```
    pub fn assert(&mut self, expr: Expr) -> &mut Self {
        self.commands.push(Command::Assert(expr));
        self
    }

    /// Adds a named assertion, emitting `(assert (! expr :named name))`.
    ///
    /// Names identify the assertion in an unsat core. See
    /// [`SmtSolver::emit_check_core`] and [`result::parse_unsat_core`].
    pub fn assert_named(&mut self, name: impl Into<String>, expr: Expr) -> &mut Self {
        self.commands.push(Command::AssertNamed(name.into(), expr));
        self
    }

    /// Opens an assertion scope: `(push 1)`.
    pub fn push(&mut self) -> &mut Self {
        self.commands.push(Command::Push);
        self
    }

    /// Closes the innermost assertion scope: `(pop 1)`.
    pub fn pop(&mut self) -> &mut Self {
        self.commands.push(Command::Pop);
        self
    }

    /// Emits the problem as an SMT-LIB2 string with no check command.
    ///
    /// Useful when you want to append your own commands at the end.
    pub fn emit_smtlib2(&self) -> String {
        self.build(Check::None)
    }

    /// Emits the problem with `(check-sat)` appended.
    ///
    /// Use this when only the verdict matters. To read a model, use
    /// [`SmtSolver::emit_check_model`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::{SmtSolver, Sort, Expr};
    ///
    /// let mut s = SmtSolver::new();
    /// s.declare_const("x", Sort::Int);
    /// s.assert(Expr::gt(Expr::var("x", Sort::Int), Expr::int(0)));
    /// assert!(s.emit_check().ends_with("(assert (> x 0))\n(check-sat)\n"));
    /// ```
    pub fn emit_check(&self) -> String {
        self.build(Check::Sat)
    }

    /// Emits the problem with `(check-sat)` and `(get-model)`, and enables
    /// `:produce-models` if it is not already set.
    ///
    /// Only read the model when the verdict is `sat`. Solvers report an error
    /// for `(get-model)` after `unsat`.
    pub fn emit_check_model(&self) -> String {
        self.build(Check::Model)
    }

    /// Emits the problem with `(check-sat)` and `(get-unsat-core)`, and enables
    /// `:produce-unsat-cores` if it is not already set.
    pub fn emit_check_core(&self) -> String {
        self.build(Check::Core)
    }

    /// Emits the problem with `(check-sat)` and `(get-value (...))` for `terms`,
    /// and enables `:produce-models` if it is not already set.
    ///
    /// Read the output with [`result::parse_get_value`]. Only read the values
    /// when the verdict is `sat`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::{Expr, Sort, SmtSolver};
    ///
    /// let mut s = SmtSolver::new();
    /// s.declare_const("x", Sort::Int);
    /// let x = Expr::var("x", Sort::Int);
    /// let problem = s.emit_check_values(&[x.clone(), Expr::add(x, Expr::int(1))]);
    /// assert!(problem.ends_with("(check-sat)\n(get-value (x (+ x 1)))\n"));
    /// ```
    pub fn emit_check_values(&self, terms: &[Expr]) -> String {
        let printed = terms.iter().map(emit_smtlib2).collect();
        self.build(Check::Values(printed))
    }

    /// Runs `backend` on the problem and reads the verdict.
    ///
    /// The problem is emitted with [`SmtSolver::emit_check_model`], so a `sat`
    /// result carries its model. The solver is killed if it runs longer than
    /// `timeout`.
    ///
    /// # Errors
    ///
    /// Returns [`SolverError::NotFound`] if the program is not installed,
    /// [`SolverError::Timeout`] if it is killed, and [`SolverError::Solver`] if
    /// it reports an error.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use out_smt_bridge::{Backend, SatResult, Expr, Sort, SmtSolver};
    /// use std::time::Duration;
    ///
    /// let mut s = SmtSolver::new();
    /// s.declare_const("x", Sort::Int);
    /// s.assert(Expr::gt(Expr::var("x", Sort::Int), Expr::int(0)));
    /// match s.check_sat(&Backend::Z3, Duration::from_secs(10)) {
    ///     Ok(SatResult::Sat(model)) => println!("x = {:?}", model.get_int("x")),
    ///     Ok(other) => println!("{other:?}"),
    ///     Err(e) => eprintln!("{e}"),
    /// }
    /// ```
    pub fn check_sat(
        &self,
        backend: &Backend,
        timeout: Duration,
    ) -> Result<SatResult, SolverError> {
        let output = result::run(backend, &self.emit_check_model(), timeout)?;
        SatResult::parse(&output)
    }

    /// Runs `backend` and reads the verdict together with the values of `terms`.
    ///
    /// The values are only meaningful when the verdict is `sat`. Each is the
    /// printed term and its value as SMT-LIB2 text. See
    /// [`result::parse_get_value`].
    ///
    /// # Errors
    ///
    /// The same errors as [`SmtSolver::check_sat`].
    pub fn check_values(
        &self,
        terms: &[Expr],
        backend: &Backend,
        timeout: Duration,
    ) -> Result<(SatResult, Vec<(String, String)>), SolverError> {
        let output = result::run(backend, &self.emit_check_values(terms), timeout)?;
        Ok((SatResult::parse(&output)?, result::parse_get_value(&output)))
    }

    /// Returns the sort of `expr` under the declarations in this solver.
    ///
    /// Variables must be declared with a matching sort, or bound by a
    /// quantifier or `let` in `expr`. Declared functions and the built-in
    /// bit-vector and string operators are checked as well.
    ///
    /// # Errors
    ///
    /// Returns a [`SortError`] describing the first mismatch found.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::{Expr, Sort, SmtSolver};
    ///
    /// let mut s = SmtSolver::new();
    /// s.declare_const("x", Sort::Int);
    /// let ok = Expr::gt(Expr::var("x", Sort::Int), Expr::int(0));
    /// assert_eq!(s.sort_of(&ok).unwrap(), Sort::Bool);
    /// let bad = Expr::add(Expr::var("x", Sort::Int), Expr::bool(true));
    /// assert!(s.sort_of(&bad).is_err());
    /// ```
    pub fn sort_of(&self, expr: &Expr) -> Result<Sort, SortError> {
        let mut locals = Vec::new();
        self.sort_in(&expr.inner, &mut locals)
    }

    fn build(&self, check: Check) -> String {
        let mut out = String::new();

        if let Some(logic) = &self.logic {
            out.push_str(&format!("(set-logic {logic})\n"));
        }
        if matches!(check, Check::Model | Check::Values(_)) && !self.has_option("produce-models") {
            out.push_str("(set-option :produce-models true)\n");
        }
        if matches!(check, Check::Core) && !self.has_option("produce-unsat-cores") {
            out.push_str("(set-option :produce-unsat-cores true)\n");
        }

        for command in &self.commands {
            out.push_str(&emit_command(command));
            out.push('\n');
        }

        match check {
            Check::None => {}
            Check::Sat => out.push_str("(check-sat)\n"),
            Check::Model => out.push_str("(check-sat)\n(get-model)\n"),
            Check::Core => out.push_str("(check-sat)\n(get-unsat-core)\n"),
            Check::Values(terms) => {
                out.push_str("(check-sat)\n");
                out.push_str(&format!("(get-value ({}))\n", terms.join(" ")));
            }
        }
        out
    }

    fn has_option(&self, name: &str) -> bool {
        self.commands
            .iter()
            .any(|c| matches!(c, Command::SetOption(n, _) if n == name))
    }

    /// Finds the sort of a declared constant. The last declaration wins.
    fn declared_const(&self, name: &str) -> Option<&Sort> {
        self.commands.iter().rev().find_map(|c| match c {
            Command::DeclareConst(n, s) if n == name => Some(s),
            _ => None,
        })
    }

    /// Finds the signature of a declared function. The last declaration wins.
    fn declared_fun(&self, name: &str) -> Option<(&[Sort], &Sort)> {
        self.commands.iter().rev().find_map(|c| match c {
            Command::DeclareFun(n, args, ret) if n == name => Some((args.as_slice(), ret)),
            _ => None,
        })
    }

    fn sort_in(
        &self,
        inner: &ExprInner,
        locals: &mut Vec<(String, Sort)>,
    ) -> Result<Sort, SortError> {
        use ExprInner as E;
        let sort = |e: &Expr, locals: &mut Vec<(String, Sort)>| self.sort_in(&e.inner, locals);
        match inner {
            E::BoolLit(_) => Ok(Sort::Bool),
            E::IntLit(_) => Ok(Sort::Int),
            E::RealLit(..) => Ok(Sort::Real),
            E::BitVecLit(_, w) => Ok(Sort::BitVec(*w)),
            E::StrLit(_) => Ok(Sort::String),

            E::Var(name, written) => {
                let known = locals
                    .iter()
                    .rev()
                    .find(|(n, _)| n == name)
                    .map(|(_, s)| s)
                    .or_else(|| self.declared_const(name));
                match known {
                    Some(s) if s == written => Ok(s.clone()),
                    Some(s) => Err(mismatch(&format!("variable `{name}`"), written, s)),
                    None => Err(SortError(format!("undeclared variable `{name}`"))),
                }
            }

            E::Not(e) => {
                expect(&sort(e, locals)?, &Sort::Bool, "not")?;
                Ok(Sort::Bool)
            }
            E::And(args) | E::Or(args) => {
                for a in args {
                    expect(&sort(a, locals)?, &Sort::Bool, "boolean connective")?;
                }
                Ok(Sort::Bool)
            }
            E::Implies(a, b) | E::Iff(a, b) => {
                expect(&sort(a, locals)?, &Sort::Bool, "boolean connective")?;
                expect(&sort(b, locals)?, &Sort::Bool, "boolean connective")?;
                Ok(Sort::Bool)
            }

            E::Eq(a, b) => {
                same_sort(&sort(a, locals)?, &sort(b, locals)?, "=")?;
                Ok(Sort::Bool)
            }
            E::Distinct(args) => {
                let sorts = args
                    .iter()
                    .map(|a| sort(a, locals))
                    .collect::<Result<Vec<_>, _>>()?;
                if let Some((first, rest)) = sorts.split_first() {
                    for s in rest {
                        same_sort(first, s, "distinct")?;
                    }
                }
                Ok(Sort::Bool)
            }
            E::Lt(a, b) | E::Le(a, b) | E::Gt(a, b) | E::Ge(a, b) => {
                let s = same_sort(&sort(a, locals)?, &sort(b, locals)?, "comparison")?;
                numeric(&s, "comparison")?;
                Ok(Sort::Bool)
            }

            E::Add(args) | E::Mul(args) => {
                let mut result = Sort::Int;
                for (i, a) in args.iter().enumerate() {
                    let s = sort(a, locals)?;
                    numeric(&s, "arithmetic")?;
                    result = if i == 0 {
                        s
                    } else {
                        same_sort(&result, &s, "arithmetic")?
                    };
                }
                Ok(result)
            }
            E::Sub(a, b) => {
                let s = same_sort(&sort(a, locals)?, &sort(b, locals)?, "-")?;
                numeric(&s, "-")?;
                Ok(s)
            }
            E::RealDiv(a, b) => {
                expect(&sort(a, locals)?, &Sort::Real, "/")?;
                expect(&sort(b, locals)?, &Sort::Real, "/")?;
                Ok(Sort::Real)
            }
            E::IntDiv(a, b) | E::Mod(a, b) => {
                expect(&sort(a, locals)?, &Sort::Int, "integer operator")?;
                expect(&sort(b, locals)?, &Sort::Int, "integer operator")?;
                Ok(Sort::Int)
            }
            E::Neg(e) | E::Abs(e) => {
                let s = sort(e, locals)?;
                numeric(&s, "unary arithmetic")?;
                Ok(s)
            }

            E::Forall(vars, body) | E::Exists(vars, body) => {
                let mark = locals.len();
                locals.extend(vars.iter().cloned());
                let result = sort(body, locals);
                locals.truncate(mark);
                expect(&result?, &Sort::Bool, "quantifier body")?;
                Ok(Sort::Bool)
            }

            E::Ite(c, t, e) => {
                expect(&sort(c, locals)?, &Sort::Bool, "ite condition")?;
                same_sort(&sort(t, locals)?, &sort(e, locals)?, "ite branches")
            }

            E::Select(a, i) => match sort(a, locals)? {
                Sort::Array(domain, range) => {
                    same_sort(&domain, &sort(i, locals)?, "select index")?;
                    Ok(*range)
                }
                other => Err(SortError(format!("select expects an array, found {other}"))),
            },
            E::Store(a, i, v) => {
                let array = sort(a, locals)?;
                match &array {
                    Sort::Array(domain, range) => {
                        same_sort(domain, &sort(i, locals)?, "store index")?;
                        same_sort(range, &sort(v, locals)?, "store value")?;
                        Ok(array)
                    }
                    other => Err(SortError(format!("store expects an array, found {other}"))),
                }
            }

            E::Apply(name, args) => self.sort_of_application(name, args, locals),

            E::Let(bindings, body) => {
                let mark = locals.len();
                let mut bound = Vec::with_capacity(bindings.len());
                for (name, value) in bindings {
                    match sort(value, locals) {
                        Ok(s) => bound.push((name.clone(), s)),
                        Err(e) => {
                            locals.truncate(mark);
                            return Err(e);
                        }
                    }
                }
                locals.extend(bound);
                let result = sort(body, locals);
                locals.truncate(mark);
                result
            }
        }
    }

    fn sort_of_application(
        &self,
        name: &str,
        args: &[Expr],
        locals: &mut Vec<(String, Sort)>,
    ) -> Result<Sort, SortError> {
        let arg_sorts = args
            .iter()
            .map(|a| self.sort_in(&a.inner, locals))
            .collect::<Result<Vec<_>, _>>()?;

        match name {
            "bvadd" | "bvsub" | "bvmul" | "bvand" | "bvor" | "bvxor" => {
                let s = bitvec_args(name, &arg_sorts, 2)?;
                Ok(s)
            }
            "bvnot" => bitvec_args(name, &arg_sorts, 1),
            "bvult" | "bvslt" => {
                bitvec_args(name, &arg_sorts, 2)?;
                Ok(Sort::Bool)
            }
            "str.++" => {
                for s in &arg_sorts {
                    expect(s, &Sort::String, name)?;
                }
                Ok(Sort::String)
            }
            "str.len" => {
                expect_arity(name, &arg_sorts, 1)?;
                expect(&arg_sorts[0], &Sort::String, name)?;
                Ok(Sort::Int)
            }
            _ => {
                if arg_sorts.is_empty() {
                    if let Some(s) = self.declared_const(name) {
                        return Ok(s.clone());
                    }
                }
                let (params, ret) = self
                    .declared_fun(name)
                    .ok_or_else(|| SortError(format!("undeclared function `{name}`")))?;
                expect_arity(name, &arg_sorts, params.len())?;
                for (want, got) in params.iter().zip(&arg_sorts) {
                    same_sort(want, got, name)?;
                }
                Ok(ret.clone())
            }
        }
    }
}

fn emit_command(command: &Command) -> String {
    match command {
        Command::SetOption(name, value) => format!("(set-option :{name} {value})"),
        Command::DeclareConst(name, sort) => {
            format!(
                "(declare-const {} {})",
                quote_symbol(name),
                sort.to_smtlib2()
            )
        }
        Command::DeclareFun(name, args, ret) => {
            let args: Vec<String> = args.iter().map(Sort::to_smtlib2).collect();
            format!(
                "(declare-fun {} ({}) {})",
                quote_symbol(name),
                args.join(" "),
                ret.to_smtlib2()
            )
        }
        Command::Assert(expr) => format!("(assert {})", emit_smtlib2(expr)),
        Command::AssertNamed(name, expr) => format!(
            "(assert (! {} :named {}))",
            emit_smtlib2(expr),
            quote_symbol(name)
        ),
        Command::Push => "(push 1)".into(),
        Command::Pop => "(pop 1)".into(),
    }
}

fn mismatch(what: &str, want: &Sort, got: &Sort) -> SortError {
    SortError(format!("{what}: expected {want}, found {got}"))
}

fn expect(got: &Sort, want: &Sort, context: &str) -> Result<(), SortError> {
    if got == want {
        Ok(())
    } else {
        Err(mismatch(context, want, got))
    }
}

fn same_sort(a: &Sort, b: &Sort, context: &str) -> Result<Sort, SortError> {
    if a == b {
        Ok(a.clone())
    } else {
        Err(SortError(format!(
            "{context}: operands have sorts {a} and {b}"
        )))
    }
}

fn numeric(s: &Sort, context: &str) -> Result<(), SortError> {
    match s {
        Sort::Int | Sort::Real => Ok(()),
        other => Err(SortError(format!(
            "{context} expects Int or Real, found {other}"
        ))),
    }
}

fn expect_arity(name: &str, args: &[Sort], n: usize) -> Result<(), SortError> {
    if args.len() == n {
        Ok(())
    } else {
        Err(SortError(format!(
            "`{name}` expects {n} argument(s), found {}",
            args.len()
        )))
    }
}

/// Checks that `args` is `n` bit-vectors of one width and returns that sort.
fn bitvec_args(name: &str, args: &[Sort], n: usize) -> Result<Sort, SortError> {
    expect_arity(name, args, n)?;
    let first = &args[0];
    if !matches!(first, Sort::BitVec(_)) {
        return Err(SortError(format!(
            "`{name}` expects bit-vectors, found {first}"
        )));
    }
    for a in &args[1..] {
        same_sort(first, a, name)?;
    }
    Ok(first.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_with_check() {
        let s = SmtSolver::new();
        assert_eq!(s.emit_check(), "(check-sat)\n");
        assert_eq!(
            s.emit_check_model(),
            "(set-option :produce-models true)\n(check-sat)\n(get-model)\n"
        );
    }

    #[test]
    fn declare_and_assert() {
        let mut s = SmtSolver::new();
        s.declare_const("x", Sort::Int);
        s.assert(Expr::gt(Expr::var("x", Sort::Int), Expr::int(0)));
        let out = s.emit_smtlib2();
        assert!(
            out.contains("(declare-const x Int)\n"),
            "missing declaration"
        );
        assert!(out.contains("(assert (> x 0))\n"), "missing assertion");
    }

    #[test]
    fn logic_prefix() {
        let mut s = SmtSolver::new();
        s.set_logic("QF_LIA");
        assert!(s.emit_check().starts_with("(set-logic QF_LIA)\n"));
    }

    #[test]
    #[should_panic(expected = "not a simple SMT-LIB2 symbol")]
    fn logic_injection_is_rejected() {
        let mut s = SmtSolver::new();
        s.set_logic("QF_LIA) (exit)");
    }

    #[test]
    fn full_problem_is_exact() {
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
        assert_eq!(
            s.emit_check(),
            "(set-logic QF_LIA)\n\
             (declare-const x Int)\n\
             (declare-const y Int)\n\
             (assert (> x 0))\n\
             (assert (> y 0))\n\
             (assert (= (+ x y) 10))\n\
             (check-sat)\n"
        );
    }

    #[test]
    fn quoted_declarations_and_names() {
        let mut s = SmtSolver::new();
        s.declare_const("my var", Sort::Bool);
        assert!(s.emit_smtlib2().contains("(declare-const |my var| Bool)"));
    }

    #[test]
    fn declare_fun_emits_signature() {
        let mut s = SmtSolver::new();
        s.declare_fun("f", &[Sort::Int, Sort::Bool], Sort::Real);
        assert!(s.emit_smtlib2().contains("(declare-fun f (Int Bool) Real)"));
    }

    #[test]
    fn named_assertions_and_unsat_core_mode() {
        let mut s = SmtSolver::new();
        s.assert_named("a", Expr::bool(true));
        let out = s.emit_check_core();
        assert!(out.starts_with("(set-option :produce-unsat-cores true)\n"));
        assert!(out.contains("(assert (! true :named a))\n"));
        assert!(out.ends_with("(check-sat)\n(get-unsat-core)\n"));
    }

    #[test]
    fn push_and_pop_keep_command_order() {
        let mut s = SmtSolver::new();
        s.declare_const("x", Sort::Int);
        s.push();
        s.assert(Expr::bool(false));
        s.pop();
        let out = s.emit_smtlib2();
        let push = out.find("(push 1)").unwrap();
        let pop = out.find("(pop 1)").unwrap();
        assert!(push < pop);
        assert!(out.find("(assert false)").unwrap() < pop);
    }

    #[test]
    fn explicit_produce_models_is_not_duplicated() {
        let mut s = SmtSolver::new();
        s.set_option("produce-models", "true");
        let out = s.emit_check_model();
        assert_eq!(out.matches("produce-models").count(), 1);
    }

    #[test]
    fn sort_of_accepts_well_typed_terms() {
        let mut s = SmtSolver::new();
        s.declare_const("x", Sort::Int);
        s.declare_const("r", Sort::Real);
        let x = || Expr::var("x", Sort::Int);
        assert_eq!(s.sort_of(&Expr::add(x(), x())).unwrap(), Sort::Int);
        assert_eq!(
            s.sort_of(&Expr::idiv(x(), Expr::int(2))).unwrap(),
            Sort::Int
        );
        let r = Expr::var("r", Sort::Real);
        assert_eq!(s.sort_of(&Expr::rdiv(r.clone(), r)).unwrap(), Sort::Real);
        assert_eq!(
            s.sort_of(&Expr::ite(Expr::bool(true), x(), x())).unwrap(),
            Sort::Int
        );
    }

    #[test]
    fn sort_of_rejects_ill_typed_terms() {
        let mut s = SmtSolver::new();
        s.declare_const("x", Sort::Int);
        let x = || Expr::var("x", Sort::Int);
        assert!(s.sort_of(&Expr::add(x(), Expr::bool(true))).is_err());
        assert!(s.sort_of(&Expr::not(x())).is_err());
        assert!(
            s.sort_of(&Expr::rdiv(x(), Expr::int(2))).is_err(),
            "real division needs Real"
        );
        assert!(s.sort_of(&Expr::var("y", Sort::Int)).is_err(), "undeclared");
        assert!(
            s.sort_of(&Expr::var("x", Sort::Bool)).is_err(),
            "sort disagrees with declaration"
        );
    }

    #[test]
    fn sort_of_checks_arrays_and_functions() {
        let mut s = SmtSolver::new();
        let arr = Sort::Array(Box::new(Sort::Int), Box::new(Sort::Bool));
        s.declare_const("a", arr.clone());
        s.declare_fun("g", &[Sort::Int], Sort::Bool);
        let a = || Expr::var("a", arr.clone());
        assert_eq!(
            s.sort_of(&Expr::select(a(), Expr::int(0))).unwrap(),
            Sort::Bool
        );
        assert!(s.sort_of(&Expr::select(a(), Expr::bool(true))).is_err());
        assert_eq!(
            s.sort_of(&Expr::store(a(), Expr::int(0), Expr::bool(false)))
                .unwrap(),
            arr
        );
        assert_eq!(
            s.sort_of(&Expr::apply("g", vec![Expr::int(1)])).unwrap(),
            Sort::Bool
        );
        assert!(s.sort_of(&Expr::apply("g", vec![])).is_err(), "arity");
    }

    #[test]
    fn sort_of_checks_bitvectors_and_strings() {
        let mut s = SmtSolver::new();
        s.declare_const("b", Sort::BitVec(8));
        s.declare_const("t", Sort::String);
        let b = || Expr::var("b", Sort::BitVec(8));
        assert_eq!(s.sort_of(&Expr::bvadd(b(), b())).unwrap(), Sort::BitVec(8));
        assert_eq!(s.sort_of(&Expr::bvult(b(), b())).unwrap(), Sort::Bool);
        assert!(s.sort_of(&Expr::bvadd(b(), Expr::bitvec(1, 16))).is_err());
        let t = || Expr::var("t", Sort::String);
        assert_eq!(s.sort_of(&Expr::str_len(t())).unwrap(), Sort::Int);
        assert!(s.sort_of(&Expr::str_len(Expr::int(1))).is_err());
    }

    #[test]
    fn sort_of_handles_binders_and_let() {
        let s = SmtSolver::new();
        let body = Expr::ge(Expr::var("n", Sort::Int), Expr::int(0));
        let q = Expr::forall(vec![("n".into(), Sort::Int)], body);
        assert_eq!(s.sort_of(&q).unwrap(), Sort::Bool);
        let x = Expr::int(2);
        let e = Expr::let_in(
            vec![("t".into(), x)],
            Expr::add(Expr::var("t", Sort::Int), Expr::int(1)),
        );
        assert_eq!(s.sort_of(&e).unwrap(), Sort::Int);
        // The binder does not leak out of the quantifier.
        assert!(s.sort_of(&Expr::var("n", Sort::Int)).is_err());
    }
}

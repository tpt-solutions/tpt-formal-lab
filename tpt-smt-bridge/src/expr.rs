//! SMT expression AST.

use std::sync::Arc;
use crate::sort::Sort;

/// An SMT expression that can be emitted as SMT-LIB2.
///
/// Build expressions using the static constructor methods, then pass them
/// to [`SmtSolver::assert`](crate::SmtSolver::assert) or emit them directly
/// with [`emit_smtlib2`](crate::emit_smtlib2).
///
/// # Example
///
/// ```rust
/// use tpt_smt_bridge::{Expr, Sort, emit_smtlib2};
///
/// let x = Expr::var("x", Sort::Int);
/// let y = Expr::var("y", Sort::Int);
/// let sum = Expr::add(x, y);
/// assert_eq!(emit_smtlib2(&sum), "(+ x y)");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub(crate) inner: Arc<ExprInner>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ExprInner {
    // Literals
    BoolLit(bool),
    IntLit(i64),
    RealLit(i64, u64), // numerator, denominator for exact representation
    BitVecLit(u64, u32), // value, width

    // Variables
    Var(String, Sort),

    // Propositional
    Not(Expr),
    And(Vec<Expr>),
    Or(Vec<Expr>),
    Implies(Expr, Expr),
    Iff(Expr, Expr),

    // Equality / comparison
    Eq(Expr, Expr),
    Distinct(Vec<Expr>),
    Lt(Expr, Expr),
    Le(Expr, Expr),
    Gt(Expr, Expr),
    Ge(Expr, Expr),

    // Arithmetic
    Add(Vec<Expr>),
    Sub(Expr, Expr),
    Mul(Vec<Expr>),
    Div(Expr, Expr),
    Neg(Expr),
    Mod(Expr, Expr),
    Abs(Expr),

    // Quantifiers
    Forall(Vec<(String, Sort)>, Expr),
    Exists(Vec<(String, Sort)>, Expr),

    // Conditional
    Ite(Expr, Expr, Expr),
}

impl Expr {
    fn make(inner: ExprInner) -> Self {
        Self { inner: Arc::new(inner) }
    }

    // ── Literals ──────────────────────────────────────────────────────────

    /// Creates a boolean literal.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_smt_bridge::{Expr, emit_smtlib2};
    /// assert_eq!(emit_smtlib2(&Expr::bool(true)), "true");
    /// assert_eq!(emit_smtlib2(&Expr::bool(false)), "false");
    /// ```
    pub fn bool(b: bool) -> Self { Self::make(ExprInner::BoolLit(b)) }

    /// Creates an integer literal.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_smt_bridge::{Expr, emit_smtlib2};
    /// assert_eq!(emit_smtlib2(&Expr::int(42)), "42");
    /// ```
    pub fn int(n: i64) -> Self { Self::make(ExprInner::IntLit(n)) }

    /// Creates a real literal as an exact fraction `numer / denom`.
    pub fn real(numer: i64, denom: u64) -> Self {
        Self::make(ExprInner::RealLit(numer, denom))
    }

    /// Creates a bit-vector literal with the given value and bit width.
    pub fn bitvec(value: u64, width: u32) -> Self {
        Self::make(ExprInner::BitVecLit(value, width))
    }

    // ── Variables ─────────────────────────────────────────────────────────

    /// Creates a variable reference with a given name and sort.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_smt_bridge::{Expr, Sort, emit_smtlib2};
    /// let x = Expr::var("x", Sort::Int);
    /// assert_eq!(emit_smtlib2(&x), "x");
    /// ```
    pub fn var(name: impl Into<String>, sort: Sort) -> Self {
        Self::make(ExprInner::Var(name.into(), sort))
    }

    // ── Propositional ─────────────────────────────────────────────────────

    /// Logical negation: `(not e)`.
    #[allow(clippy::should_implement_trait)] // AST constructor, not std::ops::Not
    pub fn not(e: Expr) -> Self { Self::make(ExprInner::Not(e)) }

    /// Logical conjunction: `(and a b ...)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_smt_bridge::{Expr, emit_smtlib2};
    /// let f = Expr::and(vec![Expr::bool(true), Expr::bool(false)]);
    /// assert_eq!(emit_smtlib2(&f), "(and true false)");
    /// ```
    pub fn and(args: Vec<Expr>) -> Self { Self::make(ExprInner::And(args)) }

    /// Logical disjunction: `(or a b ...)`.
    pub fn or(args: Vec<Expr>) -> Self { Self::make(ExprInner::Or(args)) }

    /// Implication: `(=> a b)`.
    pub fn implies(a: Expr, b: Expr) -> Self { Self::make(ExprInner::Implies(a, b)) }

    /// Biconditional: `(= a b)` on boolean sorts.
    pub fn iff(a: Expr, b: Expr) -> Self { Self::make(ExprInner::Iff(a, b)) }

    // ── Comparison ────────────────────────────────────────────────────────

    /// Equality: `(= a b)`.
    pub fn eq(a: Expr, b: Expr) -> Self { Self::make(ExprInner::Eq(a, b)) }

    /// Distinctness: `(distinct a b ...)`.
    pub fn distinct(args: Vec<Expr>) -> Self { Self::make(ExprInner::Distinct(args)) }

    /// Less-than: `(< a b)`.
    pub fn lt(a: Expr, b: Expr) -> Self { Self::make(ExprInner::Lt(a, b)) }

    /// Less-than-or-equal: `(<= a b)`.
    pub fn le(a: Expr, b: Expr) -> Self { Self::make(ExprInner::Le(a, b)) }

    /// Greater-than: `(> a b)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_smt_bridge::{Expr, Sort, emit_smtlib2};
    /// let x = Expr::var("x", Sort::Int);
    /// let f = Expr::gt(x, Expr::int(0));
    /// assert_eq!(emit_smtlib2(&f), "(> x 0)");
    /// ```
    pub fn gt(a: Expr, b: Expr) -> Self { Self::make(ExprInner::Gt(a, b)) }

    /// Greater-than-or-equal: `(>= a b)`.
    pub fn ge(a: Expr, b: Expr) -> Self { Self::make(ExprInner::Ge(a, b)) }

    // ── Arithmetic ────────────────────────────────────────────────────────

    /// Addition: `(+ a b ...)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_smt_bridge::{Expr, Sort, emit_smtlib2};
    /// let x = Expr::var("x", Sort::Int);
    /// let y = Expr::var("y", Sort::Int);
    /// assert_eq!(emit_smtlib2(&Expr::add(x, y)), "(+ x y)");
    /// ```
    #[allow(clippy::should_implement_trait)] // AST constructor, not std::ops::Add
    pub fn add(a: Expr, b: Expr) -> Self { Self::make(ExprInner::Add(vec![a, b])) }

    /// Addition of multiple terms: `(+ a b c ...)`.
    pub fn add_many(args: Vec<Expr>) -> Self { Self::make(ExprInner::Add(args)) }

    /// Subtraction: `(- a b)`.
    #[allow(clippy::should_implement_trait)] // AST constructor, not std::ops::Sub
    pub fn sub(a: Expr, b: Expr) -> Self { Self::make(ExprInner::Sub(a, b)) }

    /// Multiplication: `(* a b)`.
    #[allow(clippy::should_implement_trait)] // AST constructor, not std::ops::Mul
    pub fn mul(a: Expr, b: Expr) -> Self { Self::make(ExprInner::Mul(vec![a, b])) }

    /// Division: `(div a b)` for integers, `(/ a b)` for reals.
    #[allow(clippy::should_implement_trait)] // AST constructor, not std::ops::Div
    pub fn div(a: Expr, b: Expr) -> Self { Self::make(ExprInner::Div(a, b)) }

    /// Integer modulo: `(mod a b)`.
    pub fn modulo(a: Expr, b: Expr) -> Self { Self::make(ExprInner::Mod(a, b)) }

    /// Unary negation: `(- e)`.
    #[allow(clippy::should_implement_trait)] // AST constructor, not std::ops::Neg
    pub fn neg(e: Expr) -> Self { Self::make(ExprInner::Neg(e)) }

    /// Absolute value: `(abs e)`.
    pub fn abs(e: Expr) -> Self { Self::make(ExprInner::Abs(e)) }

    // ── Quantifiers ───────────────────────────────────────────────────────

    /// Universal quantification: `(forall ((x Sort) ...) body)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_smt_bridge::{Expr, Sort, emit_smtlib2};
    ///
    /// let body = Expr::ge(Expr::var("x", Sort::Int), Expr::int(0));
    /// let f = Expr::forall(vec![("x".into(), Sort::Int)], body);
    /// assert_eq!(emit_smtlib2(&f), "(forall ((x Int)) (>= x 0))");
    /// ```
    pub fn forall(vars: Vec<(String, Sort)>, body: Expr) -> Self {
        Self::make(ExprInner::Forall(vars, body))
    }

    /// Existential quantification: `(exists ((x Sort) ...) body)`.
    pub fn exists(vars: Vec<(String, Sort)>, body: Expr) -> Self {
        Self::make(ExprInner::Exists(vars, body))
    }

    // ── Conditional ───────────────────────────────────────────────────────

    /// If-then-else: `(ite cond then else)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_smt_bridge::{Expr, Sort, emit_smtlib2};
    ///
    /// let cond = Expr::gt(Expr::var("x", Sort::Int), Expr::int(0));
    /// let f = Expr::ite(cond, Expr::int(1), Expr::int(-1));
    /// assert_eq!(emit_smtlib2(&f), "(ite (> x 0) 1 (- 1))");
    /// ```
    pub fn ite(cond: Expr, then: Expr, else_: Expr) -> Self {
        Self::make(ExprInner::Ite(cond, then, else_))
    }
}

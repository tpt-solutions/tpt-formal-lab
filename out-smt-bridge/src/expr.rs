//! SMT expression AST.

use crate::sort::Sort;
use std::sync::Arc;

/// An SMT expression that can be emitted as SMT-LIB2.
///
/// Build expressions using the static constructor methods, then pass them
/// to [`SmtSolver::assert`](crate::SmtSolver::assert) or emit them directly
/// with [`emit_smtlib2`](crate::emit_smtlib2).
///
/// # Example
///
/// ```rust
/// use out_smt_bridge::{Expr, Sort, emit_smtlib2};
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
    IntLit(i128),
    RealLit(i64, u64),   // numerator, denominator for exact representation
    BitVecLit(u64, u32), // value, width (1..=64); the value is masked to the width
    StrLit(String),

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
    RealDiv(Expr, Expr),
    IntDiv(Expr, Expr),
    Mod(Expr, Expr),
    Neg(Expr),
    Abs(Expr),

    // Quantifiers
    Forall(Vec<(String, Sort)>, Expr),
    Exists(Vec<(String, Sort)>, Expr),

    // Conditional
    Ite(Expr, Expr, Expr),

    // Arrays
    Select(Expr, Expr),
    Store(Expr, Expr, Expr),

    // Uninterpreted or theory function application: `(name args...)`
    Apply(String, Vec<Expr>),

    // Shared sub-expressions: `(let ((name value) ...) body)`
    Let(Vec<(String, Expr)>, Expr),
}

impl Expr {
    fn make(inner: ExprInner) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }

    // ── Literals ──────────────────────────────────────────────────────────

    /// Creates a boolean literal.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::{Expr, emit_smtlib2};
    /// assert_eq!(emit_smtlib2(&Expr::bool(true)), "true");
    /// assert_eq!(emit_smtlib2(&Expr::bool(false)), "false");
    /// ```
    pub fn bool(b: bool) -> Self {
        Self::make(ExprInner::BoolLit(b))
    }

    /// Creates an integer literal.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::{Expr, emit_smtlib2};
    /// assert_eq!(emit_smtlib2(&Expr::int(42)), "42");
    /// ```
    pub fn int(n: i64) -> Self {
        Self::make(ExprInner::IntLit(i128::from(n)))
    }

    /// Creates an integer literal from an `i128`, for values beyond `i64`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::{Expr, emit_smtlib2};
    /// let big = i128::from(i64::MAX) * 4;
    /// assert_eq!(emit_smtlib2(&Expr::int128(big)), "36893488147419103228");
    /// ```
    pub fn int128(n: i128) -> Self {
        Self::make(ExprInner::IntLit(n))
    }

    /// Creates a real literal as an exact fraction `numer / denom`.
    ///
    /// # Panics
    ///
    /// Panics if `denom` is zero.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::{Expr, emit_smtlib2};
    /// assert_eq!(emit_smtlib2(&Expr::real(-1, 2)), "(/ (- 1.0) 2.0)");
    /// ```
    pub fn real(numer: i64, denom: u64) -> Self {
        assert!(denom != 0, "real literal denominator must be non-zero");
        Self::make(ExprInner::RealLit(numer, denom))
    }

    /// Creates a bit-vector literal with the given value and bit width.
    ///
    /// Bits of `value` above `width` are discarded, so `bitvec(256, 8)` is `#x00`.
    ///
    /// # Panics
    ///
    /// Panics if `width` is zero or greater than 64, the widest width a `u64`
    /// value can hold.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::{Expr, emit_smtlib2};
    /// assert_eq!(emit_smtlib2(&Expr::bitvec(256, 8)), "(_ bv0 8)");
    /// ```
    pub fn bitvec(value: u64, width: u32) -> Self {
        assert!(
            (1..=64).contains(&width),
            "bit-vector width must be between 1 and 64, got {width}"
        );
        Self::make(ExprInner::BitVecLit(value, width))
    }

    /// Creates a string literal. Non-printable characters, backslash and the
    /// double quote are escaped, so the literal is always valid SMT-LIB2.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::{Expr, emit_smtlib2};
    /// assert_eq!(emit_smtlib2(&Expr::string("a\"b")), "\"a\"\"b\"");
    /// ```
    pub fn string(s: impl Into<String>) -> Self {
        Self::make(ExprInner::StrLit(s.into()))
    }

    // ── Variables ─────────────────────────────────────────────────────────

    /// Creates a variable reference with a given name and sort.
    ///
    /// Names that are not simple SMT-LIB2 symbols are written in `|...|` form.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::{Expr, Sort, emit_smtlib2};
    /// let x = Expr::var("x", Sort::Int);
    /// assert_eq!(emit_smtlib2(&x), "x");
    /// assert_eq!(emit_smtlib2(&Expr::var("my var", Sort::Int)), "|my var|");
    /// ```
    pub fn var(name: impl Into<String>, sort: Sort) -> Self {
        Self::make(ExprInner::Var(name.into(), sort))
    }

    // ── Propositional ─────────────────────────────────────────────────────

    /// Logical negation: `(not e)`.
    #[allow(clippy::should_implement_trait)] // AST constructor, not std::ops::Not
    pub fn not(e: Expr) -> Self {
        Self::make(ExprInner::Not(e))
    }

    /// Logical conjunction: `(and a b ...)`.
    ///
    /// An empty list is `true`, and a single argument is emitted as itself.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::{Expr, emit_smtlib2};
    /// let f = Expr::and(vec![Expr::bool(true), Expr::bool(false)]);
    /// assert_eq!(emit_smtlib2(&f), "(and true false)");
    /// assert_eq!(emit_smtlib2(&Expr::and(vec![])), "true");
    /// ```
    pub fn and(args: Vec<Expr>) -> Self {
        Self::make(ExprInner::And(args))
    }

    /// Logical disjunction: `(or a b ...)`.
    ///
    /// An empty list is `false`, and a single argument is emitted as itself.
    pub fn or(args: Vec<Expr>) -> Self {
        Self::make(ExprInner::Or(args))
    }

    /// Implication: `(=> a b)`.
    pub fn implies(a: Expr, b: Expr) -> Self {
        Self::make(ExprInner::Implies(a, b))
    }

    /// Biconditional: `(= a b)` on boolean sorts.
    pub fn iff(a: Expr, b: Expr) -> Self {
        Self::make(ExprInner::Iff(a, b))
    }

    // ── Comparison ────────────────────────────────────────────────────────

    /// Equality: `(= a b)`.
    pub fn eq(a: Expr, b: Expr) -> Self {
        Self::make(ExprInner::Eq(a, b))
    }

    /// Distinctness: `(distinct a b ...)`. Fewer than two arguments is `true`.
    pub fn distinct(args: Vec<Expr>) -> Self {
        Self::make(ExprInner::Distinct(args))
    }

    /// Less-than: `(< a b)`.
    pub fn lt(a: Expr, b: Expr) -> Self {
        Self::make(ExprInner::Lt(a, b))
    }

    /// Less-than-or-equal: `(<= a b)`.
    pub fn le(a: Expr, b: Expr) -> Self {
        Self::make(ExprInner::Le(a, b))
    }

    /// Greater-than: `(> a b)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::{Expr, Sort, emit_smtlib2};
    /// let x = Expr::var("x", Sort::Int);
    /// let f = Expr::gt(x, Expr::int(0));
    /// assert_eq!(emit_smtlib2(&f), "(> x 0)");
    /// ```
    pub fn gt(a: Expr, b: Expr) -> Self {
        Self::make(ExprInner::Gt(a, b))
    }

    /// Greater-than-or-equal: `(>= a b)`.
    pub fn ge(a: Expr, b: Expr) -> Self {
        Self::make(ExprInner::Ge(a, b))
    }

    // ── Arithmetic ────────────────────────────────────────────────────────

    /// Addition: `(+ a b ...)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::{Expr, Sort, emit_smtlib2};
    /// let x = Expr::var("x", Sort::Int);
    /// let y = Expr::var("y", Sort::Int);
    /// assert_eq!(emit_smtlib2(&Expr::add(x, y)), "(+ x y)");
    /// ```
    #[allow(clippy::should_implement_trait)] // AST constructor, not std::ops::Add
    pub fn add(a: Expr, b: Expr) -> Self {
        Self::make(ExprInner::Add(vec![a, b]))
    }

    /// Addition of multiple terms: `(+ a b c ...)`. An empty list is `0`.
    pub fn add_many(args: Vec<Expr>) -> Self {
        Self::make(ExprInner::Add(args))
    }

    /// Subtraction: `(- a b)`.
    #[allow(clippy::should_implement_trait)] // AST constructor, not std::ops::Sub
    pub fn sub(a: Expr, b: Expr) -> Self {
        Self::make(ExprInner::Sub(a, b))
    }

    /// Multiplication: `(* a b)`.
    #[allow(clippy::should_implement_trait)] // AST constructor, not std::ops::Mul
    pub fn mul(a: Expr, b: Expr) -> Self {
        Self::make(ExprInner::Mul(vec![a, b]))
    }

    /// Real division: `(/ a b)`. Use [`Expr::idiv`] for integer division.
    #[allow(clippy::should_implement_trait)] // AST constructor, not std::ops::Div
    pub fn div(a: Expr, b: Expr) -> Self {
        Self::make(ExprInner::RealDiv(a, b))
    }

    /// Real division: `(/ a b)` on `Real` terms. Same as [`Expr::div`].
    pub fn rdiv(a: Expr, b: Expr) -> Self {
        Self::make(ExprInner::RealDiv(a, b))
    }

    /// Integer division: `(div a b)` on `Int` terms.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::{Expr, emit_smtlib2};
    /// assert_eq!(emit_smtlib2(&Expr::idiv(Expr::int(7), Expr::int(2))), "(div 7 2)");
    /// assert_eq!(emit_smtlib2(&Expr::rdiv(Expr::int(7), Expr::int(2))), "(/ 7 2)");
    /// ```
    pub fn idiv(a: Expr, b: Expr) -> Self {
        Self::make(ExprInner::IntDiv(a, b))
    }

    /// Integer modulo: `(mod a b)`.
    pub fn modulo(a: Expr, b: Expr) -> Self {
        Self::make(ExprInner::Mod(a, b))
    }

    /// Unary negation: `(- e)`.
    #[allow(clippy::should_implement_trait)] // AST constructor, not std::ops::Neg
    pub fn neg(e: Expr) -> Self {
        Self::make(ExprInner::Neg(e))
    }

    /// Absolute value: `(abs e)`.
    pub fn abs(e: Expr) -> Self {
        Self::make(ExprInner::Abs(e))
    }

    // ── Quantifiers ───────────────────────────────────────────────────────

    /// Universal quantification: `(forall ((x Sort) ...) body)`.
    ///
    /// # Panics
    ///
    /// Panics if `vars` is empty. SMT-LIB2 requires at least one binder.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::{Expr, Sort, emit_smtlib2};
    ///
    /// let body = Expr::ge(Expr::var("x", Sort::Int), Expr::int(0));
    /// let f = Expr::forall(vec![("x".into(), Sort::Int)], body);
    /// assert_eq!(emit_smtlib2(&f), "(forall ((x Int)) (>= x 0))");
    /// ```
    pub fn forall(vars: Vec<(String, Sort)>, body: Expr) -> Self {
        assert!(!vars.is_empty(), "forall requires at least one binder");
        Self::make(ExprInner::Forall(vars, body))
    }

    /// Existential quantification: `(exists ((x Sort) ...) body)`.
    ///
    /// # Panics
    ///
    /// Panics if `vars` is empty.
    pub fn exists(vars: Vec<(String, Sort)>, body: Expr) -> Self {
        assert!(!vars.is_empty(), "exists requires at least one binder");
        Self::make(ExprInner::Exists(vars, body))
    }

    // ── Conditional ───────────────────────────────────────────────────────

    /// If-then-else: `(ite cond then else)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::{Expr, Sort, emit_smtlib2};
    ///
    /// let cond = Expr::gt(Expr::var("x", Sort::Int), Expr::int(0));
    /// let f = Expr::ite(cond, Expr::int(1), Expr::int(-1));
    /// assert_eq!(emit_smtlib2(&f), "(ite (> x 0) 1 (- 1))");
    /// ```
    pub fn ite(cond: Expr, then: Expr, else_: Expr) -> Self {
        Self::make(ExprInner::Ite(cond, then, else_))
    }

    // ── Arrays ────────────────────────────────────────────────────────────

    /// Array read: `(select array index)`.
    pub fn select(array: Expr, index: Expr) -> Self {
        Self::make(ExprInner::Select(array, index))
    }

    /// Array write: `(store array index value)`, which returns a new array.
    pub fn store(array: Expr, index: Expr, value: Expr) -> Self {
        Self::make(ExprInner::Store(array, index, value))
    }

    // ── Functions and let ─────────────────────────────────────────────────

    /// Application of a declared or theory function: `(name args...)`.
    ///
    /// With no arguments the name is emitted alone. Use this for functions
    /// declared with [`SmtSolver::declare_fun`](crate::SmtSolver::declare_fun)
    /// and for theory operators that have no dedicated constructor.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::{Expr, emit_smtlib2};
    /// let e = Expr::apply("f", vec![Expr::int(1), Expr::int(2)]);
    /// assert_eq!(emit_smtlib2(&e), "(f 1 2)");
    /// ```
    pub fn apply(name: impl Into<String>, args: Vec<Expr>) -> Self {
        Self::make(ExprInner::Apply(name.into(), args))
    }

    /// Shared bindings: `(let ((name value) ...) body)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use out_smt_bridge::{Expr, Sort, emit_smtlib2};
    /// let x = Expr::var("x", Sort::Int);
    /// let e = Expr::let_in(vec![("t".into(), Expr::add(x.clone(), x))], Expr::var("t", Sort::Int));
    /// assert_eq!(emit_smtlib2(&e), "(let ((t (+ x x))) t)");
    /// ```
    pub fn let_in(bindings: Vec<(String, Expr)>, body: Expr) -> Self {
        Self::make(ExprInner::Let(bindings, body))
    }

    // ── Bit-vector and string operators ───────────────────────────────────

    /// Bit-vector addition: `(bvadd a b)`.
    pub fn bvadd(a: Expr, b: Expr) -> Self {
        Self::apply("bvadd", vec![a, b])
    }

    /// Bit-vector subtraction: `(bvsub a b)`.
    pub fn bvsub(a: Expr, b: Expr) -> Self {
        Self::apply("bvsub", vec![a, b])
    }

    /// Bit-vector multiplication: `(bvmul a b)`.
    pub fn bvmul(a: Expr, b: Expr) -> Self {
        Self::apply("bvmul", vec![a, b])
    }

    /// Bit-wise and: `(bvand a b)`.
    pub fn bvand(a: Expr, b: Expr) -> Self {
        Self::apply("bvand", vec![a, b])
    }

    /// Bit-wise or: `(bvor a b)`.
    pub fn bvor(a: Expr, b: Expr) -> Self {
        Self::apply("bvor", vec![a, b])
    }

    /// Bit-wise xor: `(bvxor a b)`.
    pub fn bvxor(a: Expr, b: Expr) -> Self {
        Self::apply("bvxor", vec![a, b])
    }

    /// Bit-wise not: `(bvnot a)`.
    pub fn bvnot(a: Expr) -> Self {
        Self::apply("bvnot", vec![a])
    }

    /// Unsigned less-than: `(bvult a b)`.
    pub fn bvult(a: Expr, b: Expr) -> Self {
        Self::apply("bvult", vec![a, b])
    }

    /// Signed less-than: `(bvslt a b)`.
    pub fn bvslt(a: Expr, b: Expr) -> Self {
        Self::apply("bvslt", vec![a, b])
    }

    /// String concatenation: `(str.++ a b ...)`.
    pub fn str_concat(args: Vec<Expr>) -> Self {
        Self::apply("str.++", args)
    }

    /// String length: `(str.len s)`.
    pub fn str_len(s: Expr) -> Self {
        Self::apply("str.len", vec![s])
    }
}

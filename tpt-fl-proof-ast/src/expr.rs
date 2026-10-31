//! Core AST expression type, parameterised over [`Kind`].

use std::collections::BTreeSet;
use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::kinds::Kind;

/// The inner content of an AST node, shared between all kinds via `Arc`.
///
/// Most users interact with [`Expr<K>`] rather than `ExprNode` directly.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprNode {
    // ── Shared across kinds ──────────────────────────────────────────────
    /// A named variable (e.g. `x`, `y`, `result`).
    Var(String),

    // ── Term nodes ───────────────────────────────────────────────────────
    /// An integer literal.
    IntLit(i64),
    /// A boolean literal.
    BoolLit(bool),
    /// Binary arithmetic: `(op, lhs, rhs)`.
    BinArith(ArithOp, Arc<ExprNode>, Arc<ExprNode>),
    /// Unary negation of a term: `-e`.
    Neg(Arc<ExprNode>),
    /// Application of a named function to terms: `f(a, b)`.
    App(String, Vec<Arc<ExprNode>>),

    // ── Formula (proposition) nodes ──────────────────────────────────────
    /// Logical conjunction: `A ∧ B`.
    And(Arc<ExprNode>, Arc<ExprNode>),
    /// Logical disjunction: `A ∨ B`.
    Or(Arc<ExprNode>, Arc<ExprNode>),
    /// Logical negation: `¬A`.
    Not(Arc<ExprNode>),
    /// Logical implication: `A → B`.
    Implies(Arc<ExprNode>, Arc<ExprNode>),
    /// Logical biconditional: `A ↔ B`.
    Iff(Arc<ExprNode>, Arc<ExprNode>),
    /// Comparison relation: `(rel, lhs, rhs)`.
    Cmp(CmpOp, Arc<ExprNode>, Arc<ExprNode>),
    /// Application of a named predicate to terms: `P(a, b)`.
    Pred(String, Vec<Arc<ExprNode>>),
    /// Universal quantification: `∀ binder. body`.
    Forall(Binder, Arc<ExprNode>),
    /// Existential quantification: `∃ binder. body`.
    Exists(Binder, Arc<ExprNode>),
}

/// A quantified variable, with an optional sort.
///
/// # Example
///
/// ```rust
/// use tpt_fl_proof_ast::{AstBuilder, Binder, BinderSort};
///
/// let b = AstBuilder::new();
/// let body = b.gt(b.var_term("n"), b.int_term(0));
/// let f = b.forall_typed("n", BinderSort::Int, body);
/// assert_eq!(f.to_string(), "(∀n:Int. (n > 0))");
/// # let _ = Binder::untyped("n");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Binder {
    /// The bound name.
    pub name: String,
    /// The sort of the variable, if it was declared.
    pub sort: Option<BinderSort>,
}

impl Binder {
    /// Creates a binder with no declared sort.
    pub fn untyped(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            sort: None,
        }
    }

    /// Creates a binder with a declared sort.
    pub fn typed(name: impl Into<String>, sort: BinderSort) -> Self {
        Self {
            name: name.into(),
            sort: Some(sort),
        }
    }
}

/// The sort of a quantified variable.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BinderSort {
    /// Integers.
    Int,
    /// Real numbers.
    Real,
    /// Booleans.
    Bool,
    /// A sort known by name, such as an uninterpreted sort `U`.
    Named(String),
}

impl fmt::Display for BinderSort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinderSort::Int => f.write_str("Int"),
            BinderSort::Real => f.write_str("Real"),
            BinderSort::Bool => f.write_str("Bool"),
            BinderSort::Named(name) => f.write_str(name),
        }
    }
}

/// Arithmetic binary operations for [`ExprNode::BinArith`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArithOp {
    /// Addition.
    Add,
    /// Subtraction.
    Sub,
    /// Multiplication.
    Mul,
    /// Division.
    Div,
}

/// Comparison operators for [`ExprNode::Cmp`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CmpOp {
    /// Equal (`=`).
    Eq,
    /// Not equal (`≠`).
    Ne,
    /// Less than (`<`).
    Lt,
    /// Less than or equal (`≤`).
    Le,
    /// Greater than (`>`).
    Gt,
    /// Greater than or equal (`≥`).
    Ge,
}

/// A typed AST expression node, parameterised over a [`Kind`].
///
/// The `PhantomData<K>` field carries the kind at the type level without
/// adding any runtime storage. Constructing an `Expr<PropKind>` where an
/// `Expr<TermKind>` is expected is a **compile error**.
///
/// Prefer the [`AstBuilder`](crate::AstBuilder) API over constructing
/// `Expr` values directly.
#[derive(Clone, PartialEq)]
pub struct Expr<K: Kind> {
    pub(crate) node: Arc<ExprNode>,
    _kind: PhantomData<K>,
}

impl<K: Kind> Expr<K> {
    pub(crate) fn new(node: ExprNode) -> Self {
        Self {
            node: Arc::new(node),
            _kind: PhantomData,
        }
    }

    /// Returns the inner [`ExprNode`] for inspection or traversal.
    pub fn node(&self) -> &ExprNode {
        &self.node
    }

    /// Visits every node in pre-order: a node first, then its children, left to
    /// right. Binders are visited as part of their quantifier node.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_proof_ast::{AstBuilder, ExprNode};
    ///
    /// let b = AstBuilder::new();
    /// let f = b.and(b.var_formula("P"), b.var_formula("Q"));
    /// let mut names = Vec::new();
    /// f.walk(|n| {
    ///     if let ExprNode::Var(name) = n {
    ///         names.push(name.clone());
    ///     }
    /// });
    /// assert_eq!(names, ["P", "Q"]);
    /// ```
    pub fn walk(&self, mut visit: impl FnMut(&ExprNode)) {
        walk_node(&self.node, &mut visit);
    }

    /// Returns the free variables of the expression, in sorted order.
    ///
    /// A variable is free if no enclosing quantifier binds it. Function and
    /// predicate names are not variables.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_proof_ast::AstBuilder;
    ///
    /// let b = AstBuilder::new();
    /// let body = b.gt(b.var_term("x"), b.var_term("y"));
    /// let f = b.forall("x", body);
    /// assert_eq!(f.free_vars().into_iter().collect::<Vec<_>>(), ["y"]);
    /// ```
    pub fn free_vars(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        free_vars_node(&self.node, &mut BTreeSet::new(), &mut out);
        out
    }

    /// Replaces the free occurrences of variable `name` with `replacement`.
    ///
    /// Substitution does not capture variables. If a quantifier would capture a
    /// free variable of `replacement`, its binder is renamed first (by adding
    /// primes to its name). An occurrence of `name` that a quantifier binds is
    /// left alone.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_fl_proof_ast::AstBuilder;
    ///
    /// let b = AstBuilder::new();
    /// // (∀y. x < y)[x := y] must not capture the substituted y.
    /// let body = b.lt(b.var_term("x"), b.var_term("y"));
    /// let f = b.forall("y", body);
    /// let g = f.substitute("x", &b.var_term("y"));
    /// assert_eq!(g.to_string(), "(∀y'. (y < y'))");
    /// ```
    pub fn substitute<R: Kind>(&self, name: &str, replacement: &Expr<R>) -> Expr<K> {
        let repl_free: BTreeSet<String> = replacement.free_vars();
        Expr::new(subst_node(&self.node, name, &replacement.node, &repl_free))
    }
}

fn walk_node(node: &ExprNode, visit: &mut impl FnMut(&ExprNode)) {
    visit(node);
    for child in children(node) {
        walk_node(child, visit);
    }
}

/// The direct sub-expressions of a node, left to right. A quantifier's body
/// is its only child, because its binder is not an expression.
fn children(node: &ExprNode) -> Vec<&Arc<ExprNode>> {
    match node {
        ExprNode::Var(_) | ExprNode::IntLit(_) | ExprNode::BoolLit(_) => Vec::new(),
        ExprNode::Neg(e) | ExprNode::Not(e) => vec![e],
        ExprNode::BinArith(_, a, b)
        | ExprNode::And(a, b)
        | ExprNode::Or(a, b)
        | ExprNode::Implies(a, b)
        | ExprNode::Iff(a, b)
        | ExprNode::Cmp(_, a, b) => vec![a, b],
        ExprNode::App(_, args) | ExprNode::Pred(_, args) => args.iter().collect(),
        ExprNode::Forall(_, body) | ExprNode::Exists(_, body) => vec![body],
    }
}

fn free_vars_node(node: &ExprNode, bound: &mut BTreeSet<String>, out: &mut BTreeSet<String>) {
    match node {
        ExprNode::Var(name) => {
            if !bound.contains(name) {
                out.insert(name.clone());
            }
        }
        ExprNode::Forall(binder, body) | ExprNode::Exists(binder, body) => {
            let added = bound.insert(binder.name.clone());
            free_vars_node(body, bound, out);
            if added {
                bound.remove(&binder.name);
            }
        }
        other => {
            for child in children(other) {
                free_vars_node(child, bound, out);
            }
        }
    }
}

/// Substitutes `replacement` for the free occurrences of `name` in `node`.
fn subst_node(
    node: &ExprNode,
    name: &str,
    replacement: &ExprNode,
    repl_free: &BTreeSet<String>,
) -> ExprNode {
    let s = |n: &Arc<ExprNode>| Arc::new(subst_node(n, name, replacement, repl_free));
    match node {
        ExprNode::Var(v) if v == name => replacement.clone(),
        ExprNode::Var(_) | ExprNode::IntLit(_) | ExprNode::BoolLit(_) => node.clone(),
        ExprNode::BinArith(op, a, b) => ExprNode::BinArith(op.clone(), s(a), s(b)),
        ExprNode::Neg(e) => ExprNode::Neg(s(e)),
        ExprNode::App(f, args) => ExprNode::App(f.clone(), args.iter().map(s).collect()),
        ExprNode::And(a, b) => ExprNode::And(s(a), s(b)),
        ExprNode::Or(a, b) => ExprNode::Or(s(a), s(b)),
        ExprNode::Not(e) => ExprNode::Not(s(e)),
        ExprNode::Implies(a, b) => ExprNode::Implies(s(a), s(b)),
        ExprNode::Iff(a, b) => ExprNode::Iff(s(a), s(b)),
        ExprNode::Cmp(op, a, b) => ExprNode::Cmp(op.clone(), s(a), s(b)),
        ExprNode::Pred(p, args) => ExprNode::Pred(p.clone(), args.iter().map(s).collect()),
        ExprNode::Forall(binder, body) => {
            let (binder, body) = subst_under_binder(binder, body, name, replacement, repl_free);
            ExprNode::Forall(binder, body)
        }
        ExprNode::Exists(binder, body) => {
            let (binder, body) = subst_under_binder(binder, body, name, replacement, repl_free);
            ExprNode::Exists(binder, body)
        }
    }
}

/// Substitutes under a binder, renaming the binder when it would capture a
/// free variable of the replacement.
fn subst_under_binder(
    binder: &Binder,
    body: &ExprNode,
    name: &str,
    replacement: &ExprNode,
    repl_free: &BTreeSet<String>,
) -> (Binder, Arc<ExprNode>) {
    // The binder shadows `name`, so there is nothing to substitute inside.
    if binder.name == name {
        return (binder.clone(), Arc::new(body.clone()));
    }
    if repl_free.contains(&binder.name) {
        let mut taken = repl_free.clone();
        taken.extend(free_vars_of(body));
        taken.insert(name.to_string());
        let fresh = fresh_name(&binder.name, &taken);
        let renamed = subst_node(
            body,
            &binder.name,
            &ExprNode::Var(fresh.clone()),
            &BTreeSet::new(),
        );
        let new_binder = Binder {
            name: fresh,
            sort: binder.sort.clone(),
        };
        let new_body = subst_node(&renamed, name, replacement, repl_free);
        return (new_binder, Arc::new(new_body));
    }
    (
        binder.clone(),
        Arc::new(subst_node(body, name, replacement, repl_free)),
    )
}

fn free_vars_of(node: &ExprNode) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    free_vars_node(node, &mut BTreeSet::new(), &mut out);
    out
}

/// Appends primes to `base` until the result is not in `taken`.
fn fresh_name(base: &str, taken: &BTreeSet<String>) -> String {
    let mut candidate = format!("{base}'");
    while taken.contains(&candidate) {
        candidate.push('\'');
    }
    candidate
}

impl<K: Kind> fmt::Debug for Expr<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.node)
    }
}

impl<K: Kind> fmt::Display for Expr<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        display_node(&self.node, f)
    }
}

fn display_args(args: &[Arc<ExprNode>], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "(")?;
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            write!(f, ", ")?;
        }
        display_node(arg, f)?;
    }
    write!(f, ")")
}

fn display_binder(
    binder: &Binder,
    quantifier: &str,
    body: &ExprNode,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    // A quantifier's scope extends as far right as possible, so it is
    // parenthesised. Otherwise `A ∧ ∀x. B` would be ambiguous with
    // `(A ∧ ∀x. B) ∧ C`.
    write!(f, "({quantifier}{}", binder.name)?;
    if let Some(sort) = &binder.sort {
        write!(f, ":{sort}")?;
    }
    write!(f, ". ")?;
    display_node(body, f)?;
    write!(f, ")")
}

fn display_node(node: &ExprNode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match node {
        ExprNode::Var(name) => write!(f, "{name}"),
        ExprNode::IntLit(n) => write!(f, "{n}"),
        ExprNode::BoolLit(b) => write!(f, "{b}"),
        ExprNode::Neg(e) => {
            write!(f, "(-")?;
            display_node(e, f)?;
            write!(f, ")")
        }
        ExprNode::BinArith(op, l, r) => {
            let sym = match op {
                ArithOp::Add => "+",
                ArithOp::Sub => "-",
                ArithOp::Mul => "*",
                ArithOp::Div => "/",
            };
            write!(f, "(")?;
            display_node(l, f)?;
            write!(f, " {sym} ")?;
            display_node(r, f)?;
            write!(f, ")")
        }
        ExprNode::App(name, args) => {
            write!(f, "{name}")?;
            display_args(args, f)
        }
        ExprNode::And(l, r) => {
            write!(f, "(")?;
            display_node(l, f)?;
            write!(f, " ∧ ")?;
            display_node(r, f)?;
            write!(f, ")")
        }
        ExprNode::Or(l, r) => {
            write!(f, "(")?;
            display_node(l, f)?;
            write!(f, " ∨ ")?;
            display_node(r, f)?;
            write!(f, ")")
        }
        ExprNode::Not(e) => {
            write!(f, "¬")?;
            display_node(e, f)
        }
        ExprNode::Implies(l, r) => {
            write!(f, "(")?;
            display_node(l, f)?;
            write!(f, " → ")?;
            display_node(r, f)?;
            write!(f, ")")
        }
        ExprNode::Iff(l, r) => {
            write!(f, "(")?;
            display_node(l, f)?;
            write!(f, " ↔ ")?;
            display_node(r, f)?;
            write!(f, ")")
        }
        ExprNode::Cmp(op, l, r) => {
            let sym = match op {
                CmpOp::Eq => "=",
                CmpOp::Ne => "≠",
                CmpOp::Lt => "<",
                CmpOp::Le => "≤",
                CmpOp::Gt => ">",
                CmpOp::Ge => "≥",
            };
            write!(f, "(")?;
            display_node(l, f)?;
            write!(f, " {sym} ")?;
            display_node(r, f)?;
            write!(f, ")")
        }
        ExprNode::Pred(name, args) => {
            write!(f, "{name}")?;
            display_args(args, f)
        }
        ExprNode::Forall(binder, body) => display_binder(binder, "∀", body, f),
        ExprNode::Exists(binder, body) => display_binder(binder, "∃", body, f),
    }
}

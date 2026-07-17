//! Core AST expression type, parameterised over [`Kind`].

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
    /// Universal quantification: `∀ name. body`.
    Forall(String, Arc<ExprNode>),
    /// Existential quantification: `∃ name. body`.
    Exists(String, Arc<ExprNode>),
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
        ExprNode::Forall(v, e) => {
            write!(f, "∀{v}. ")?;
            display_node(e, f)
        }
        ExprNode::Exists(v, e) => {
            write!(f, "∃{v}. ")?;
            display_node(e, f)
        }
    }
}

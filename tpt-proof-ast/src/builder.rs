//! Fluent [`AstBuilder`] for constructing well-typed AST nodes.

use crate::expr::{ArithOp, CmpOp, Expr, ExprNode};
use crate::{Formula, Term};

/// A factory for constructing typed AST nodes.
///
/// `AstBuilder` is the primary API surface for building [`Formula`] and [`Term`]
/// values. It is a zero-cost wrapper — all state lives in the returned `Expr`
/// nodes themselves.
///
/// # Example
///
/// ```rust
/// use tpt_proof_ast::AstBuilder;
///
/// let b = AstBuilder::new();
/// let x = b.var_term("x");
/// let y = b.var_term("y");
/// let sum = b.add(x, y);
/// let stmt = b.gt(sum, b.int_term(0));
/// println!("{}", stmt); // (x + y) > 0
/// ```
#[derive(Default)]
pub struct AstBuilder;

impl AstBuilder {
    /// Creates a new `AstBuilder`.
    pub fn new() -> Self {
        Self
    }

    // ── Term constructors ─────────────────────────────────────────────────

    /// Creates a term variable with the given name.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_proof_ast::AstBuilder;
    /// let b = AstBuilder::new();
    /// let x = b.var_term("x");
    /// assert_eq!(x.to_string(), "x");
    /// ```
    pub fn var_term(&self, name: impl Into<String>) -> Term {
        Expr::new(ExprNode::Var(name.into()))
    }

    /// Creates an integer literal term.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_proof_ast::AstBuilder;
    /// let b = AstBuilder::new();
    /// assert_eq!(b.int_term(42).to_string(), "42");
    /// ```
    pub fn int_term(&self, n: i64) -> Term {
        Expr::new(ExprNode::IntLit(n))
    }

    /// Creates an addition term: `lhs + rhs`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_proof_ast::AstBuilder;
    /// let b = AstBuilder::new();
    /// let t = b.add(b.var_term("x"), b.int_term(1));
    /// assert_eq!(t.to_string(), "(x + 1)");
    /// ```
    pub fn add(&self, lhs: Term, rhs: Term) -> Term {
        Expr::new(ExprNode::BinArith(ArithOp::Add, lhs.node, rhs.node))
    }

    /// Creates a subtraction term: `lhs - rhs`.
    pub fn sub(&self, lhs: Term, rhs: Term) -> Term {
        Expr::new(ExprNode::BinArith(ArithOp::Sub, lhs.node, rhs.node))
    }

    /// Creates a multiplication term: `lhs * rhs`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_proof_ast::AstBuilder;
    /// let b = AstBuilder::new();
    /// let x = b.var_term("x");
    /// let sq = b.mul(x.clone(), x);
    /// assert_eq!(sq.to_string(), "(x * x)");
    /// ```
    pub fn mul(&self, lhs: Term, rhs: Term) -> Term {
        Expr::new(ExprNode::BinArith(ArithOp::Mul, lhs.node, rhs.node))
    }

    /// Creates a division term: `lhs / rhs`.
    pub fn div(&self, lhs: Term, rhs: Term) -> Term {
        Expr::new(ExprNode::BinArith(ArithOp::Div, lhs.node, rhs.node))
    }

    /// Creates a negation term: `-e`.
    pub fn neg_term(&self, e: Term) -> Term {
        Expr::new(ExprNode::Neg(e.node))
    }

    // ── Formula constructors ──────────────────────────────────────────────

    /// Creates a formula variable (a propositional variable).
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_proof_ast::AstBuilder;
    /// let b = AstBuilder::new();
    /// let p = b.var_formula("P");
    /// assert_eq!(p.to_string(), "P");
    /// ```
    pub fn var_formula(&self, name: impl Into<String>) -> Formula {
        Expr::new(ExprNode::Var(name.into()))
    }

    /// Creates the boolean literal `true` as a formula.
    pub fn tru(&self) -> Formula {
        Expr::new(ExprNode::BoolLit(true))
    }

    /// Creates the boolean literal `false` as a formula.
    pub fn fals(&self) -> Formula {
        Expr::new(ExprNode::BoolLit(false))
    }

    /// Creates a conjunction: `A ∧ B`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_proof_ast::AstBuilder;
    /// let b = AstBuilder::new();
    /// let f = b.and(b.var_formula("P"), b.var_formula("Q"));
    /// assert_eq!(f.to_string(), "(P ∧ Q)");
    /// ```
    pub fn and(&self, lhs: Formula, rhs: Formula) -> Formula {
        Expr::new(ExprNode::And(lhs.node, rhs.node))
    }

    /// Creates a disjunction: `A ∨ B`.
    pub fn or(&self, lhs: Formula, rhs: Formula) -> Formula {
        Expr::new(ExprNode::Or(lhs.node, rhs.node))
    }

    /// Creates a logical negation: `¬A`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_proof_ast::AstBuilder;
    /// let b = AstBuilder::new();
    /// let f = b.not(b.var_formula("P"));
    /// assert_eq!(f.to_string(), "¬P");
    /// ```
    pub fn not(&self, e: Formula) -> Formula {
        Expr::new(ExprNode::Not(e.node))
    }

    /// Creates an implication: `A → B`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_proof_ast::AstBuilder;
    /// let b = AstBuilder::new();
    /// let f = b.implies(b.var_formula("P"), b.var_formula("Q"));
    /// assert_eq!(f.to_string(), "(P → Q)");
    /// ```
    pub fn implies(&self, lhs: Formula, rhs: Formula) -> Formula {
        Expr::new(ExprNode::Implies(lhs.node, rhs.node))
    }

    /// Creates a biconditional: `A ↔ B`.
    pub fn iff(&self, lhs: Formula, rhs: Formula) -> Formula {
        Expr::new(ExprNode::Iff(lhs.node, rhs.node))
    }

    /// Creates a universal quantification: `∀ name. body`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_proof_ast::AstBuilder;
    /// let b = AstBuilder::new();
    /// let x = b.var_term("x");
    /// let body = b.gt(x, b.int_term(0));
    /// let f = b.forall("x", body);
    /// assert_eq!(f.to_string(), "∀x. (x > 0)");
    /// ```
    pub fn forall(&self, var: impl Into<String>, body: Formula) -> Formula {
        Expr::new(ExprNode::Forall(var.into(), body.node))
    }

    /// Creates an existential quantification: `∃ name. body`.
    pub fn exists(&self, var: impl Into<String>, body: Formula) -> Formula {
        Expr::new(ExprNode::Exists(var.into(), body.node))
    }

    // ── Comparison constructors (Term × Term → Formula) ───────────────────

    /// Creates an equality formula: `lhs = rhs`.
    pub fn eq(&self, lhs: Term, rhs: Term) -> Formula {
        Expr::new(ExprNode::Cmp(CmpOp::Eq, lhs.node, rhs.node))
    }

    /// Creates a not-equal formula: `lhs ≠ rhs`.
    pub fn ne(&self, lhs: Term, rhs: Term) -> Formula {
        Expr::new(ExprNode::Cmp(CmpOp::Ne, lhs.node, rhs.node))
    }

    /// Creates a less-than formula: `lhs < rhs`.
    pub fn lt(&self, lhs: Term, rhs: Term) -> Formula {
        Expr::new(ExprNode::Cmp(CmpOp::Lt, lhs.node, rhs.node))
    }

    /// Creates a less-than-or-equal formula: `lhs ≤ rhs`.
    pub fn le(&self, lhs: Term, rhs: Term) -> Formula {
        Expr::new(ExprNode::Cmp(CmpOp::Le, lhs.node, rhs.node))
    }

    /// Creates a greater-than formula: `lhs > rhs`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_proof_ast::AstBuilder;
    /// let b = AstBuilder::new();
    /// let f = b.gt(b.var_term("x"), b.int_term(0));
    /// assert_eq!(f.to_string(), "(x > 0)");
    /// ```
    pub fn gt(&self, lhs: Term, rhs: Term) -> Formula {
        Expr::new(ExprNode::Cmp(CmpOp::Gt, lhs.node, rhs.node))
    }

    /// Creates a greater-than-or-equal formula: `lhs ≥ rhs`.
    pub fn ge(&self, lhs: Term, rhs: Term) -> Formula {
        Expr::new(ExprNode::Cmp(CmpOp::Ge, lhs.node, rhs.node))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn term_display() {
        let b = AstBuilder::new();
        let x = b.var_term("x");
        let y = b.int_term(5);
        let t = b.add(x, y);
        assert_eq!(t.to_string(), "(x + 5)");
    }

    #[test]
    fn formula_display() {
        let b = AstBuilder::new();
        let p = b.var_formula("P");
        let q = b.var_formula("Q");
        let f = b.implies(p, q);
        assert_eq!(f.to_string(), "(P → Q)");
    }

    #[test]
    fn forall_display() {
        let b = AstBuilder::new();
        let x = b.var_term("x");
        let body = b.gt(x, b.int_term(0));
        let f = b.forall("x", body);
        assert_eq!(f.to_string(), "∀x. (x > 0)");
    }

    #[test]
    fn nested_formula() {
        let b = AstBuilder::new();
        let x = b.var_term("x");
        let zero = b.int_term(0);
        let pos = b.gt(x.clone(), zero.clone());
        let sq = b.mul(x.clone(), x);
        let sq_pos = b.gt(sq, zero);
        let body = b.implies(pos, sq_pos);
        let f = b.forall("x", body);
        assert_eq!(f.to_string(), "∀x. ((x > 0) → ((x * x) > 0))");
    }

    #[test]
    fn not_and_or() {
        let b = AstBuilder::new();
        let p = b.var_formula("P");
        let q = b.var_formula("Q");
        let not_p = b.not(p);
        let f = b.or(not_p, q);
        assert_eq!(f.to_string(), "(¬P ∨ Q)");
    }
}

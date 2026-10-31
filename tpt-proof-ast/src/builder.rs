//! Fluent [`AstBuilder`] for constructing well-typed AST nodes.

use crate::expr::{ArithOp, Binder, BinderSort, CmpOp, Expr, ExprNode};
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
    /// assert_eq!(f.to_string(), "(∀x. (x > 0))");
    /// ```
    pub fn forall(&self, var: impl Into<String>, body: Formula) -> Formula {
        Expr::new(ExprNode::Forall(Binder::untyped(var), body.node))
    }

    /// Creates a universal quantification over a variable with a declared sort:
    /// `∀ name:Sort. body`.
    pub fn forall_typed(&self, var: impl Into<String>, sort: BinderSort, body: Formula) -> Formula {
        Expr::new(ExprNode::Forall(Binder::typed(var, sort), body.node))
    }

    /// Creates an existential quantification: `∃ name. body`.
    pub fn exists(&self, var: impl Into<String>, body: Formula) -> Formula {
        Expr::new(ExprNode::Exists(Binder::untyped(var), body.node))
    }

    /// Creates an existential quantification over a variable with a declared
    /// sort: `∃ name:Sort. body`.
    pub fn exists_typed(&self, var: impl Into<String>, sort: BinderSort, body: Formula) -> Formula {
        Expr::new(ExprNode::Exists(Binder::typed(var, sort), body.node))
    }

    /// Applies a named function to terms: `f(a, b)`. The result is a term.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_proof_ast::AstBuilder;
    ///
    /// let b = AstBuilder::new();
    /// let t = b.app_term("f", vec![b.var_term("x"), b.int_term(2)]);
    /// assert_eq!(t.to_string(), "f(x, 2)");
    /// ```
    pub fn app_term(&self, name: impl Into<String>, args: Vec<Term>) -> Term {
        Expr::new(ExprNode::App(
            name.into(),
            args.into_iter().map(|a| a.node).collect(),
        ))
    }

    /// Applies a named predicate to terms: `P(a, b)`. The result is a formula.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_proof_ast::AstBuilder;
    ///
    /// let b = AstBuilder::new();
    /// let p = b.pred("Prime", vec![b.var_term("n")]);
    /// assert_eq!(p.to_string(), "Prime(n)");
    /// ```
    pub fn pred(&self, name: impl Into<String>, args: Vec<Term>) -> Formula {
        Expr::new(ExprNode::Pred(
            name.into(),
            args.into_iter().map(|a| a.node).collect(),
        ))
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
        assert_eq!(f.to_string(), "(∀x. (x > 0))");
    }

    #[test]
    fn quantifier_scope_is_parenthesised_inside_connectives() {
        let b = AstBuilder::new();
        let p = b.var_formula("P");
        let q = b.var_formula("Q");
        let all = b.forall("x", q.clone());
        let f = b.and(p.clone(), all.clone());
        assert_eq!(f.to_string(), "(P ∧ (∀x. Q))");
        // Without the parentheses this would read as `(P ∧ ∀x. Q) ∧ ...`.
        let g = b.and(f, p);
        assert_eq!(g.to_string(), "((P ∧ (∀x. Q)) ∧ P)");
    }

    #[test]
    fn exists_display() {
        let b = AstBuilder::new();
        let x = b.var_term("x");
        let f = b.exists("x", b.eq(x, b.int_term(3)));
        assert_eq!(f.to_string(), "(∃x. (x = 3))");
    }

    #[test]
    fn iff_display() {
        let b = AstBuilder::new();
        let p = b.var_formula("P");
        let q = b.var_formula("Q");
        assert_eq!(b.iff(p, q).to_string(), "(P ↔ Q)");
    }

    #[test]
    fn comparison_display_for_every_operator() {
        let b = AstBuilder::new();
        let x = || b.var_term("x");
        let y = || b.var_term("y");
        assert_eq!(b.eq(x(), y()).to_string(), "(x = y)");
        assert_eq!(b.ne(x(), y()).to_string(), "(x ≠ y)");
        assert_eq!(b.lt(x(), y()).to_string(), "(x < y)");
        assert_eq!(b.le(x(), y()).to_string(), "(x ≤ y)");
        assert_eq!(b.gt(x(), y()).to_string(), "(x > y)");
        assert_eq!(b.ge(x(), y()).to_string(), "(x ≥ y)");
    }

    #[test]
    fn sub_div_neg_display() {
        let b = AstBuilder::new();
        let x = || b.var_term("x");
        assert_eq!(b.sub(x(), b.int_term(1)).to_string(), "(x - 1)");
        assert_eq!(b.div(x(), b.int_term(2)).to_string(), "(x / 2)");
        assert_eq!(b.neg_term(x()).to_string(), "(-x)");
    }

    #[test]
    fn truth_constants_display() {
        let b = AstBuilder::new();
        assert_eq!(b.tru().to_string(), "true");
        assert_eq!(b.fals().to_string(), "false");
        assert_eq!(b.and(b.tru(), b.fals()).to_string(), "(true ∧ false)");
    }

    #[test]
    fn debug_shows_the_node_tree() {
        let b = AstBuilder::new();
        let t = b.neg_term(b.int_term(4));
        assert_eq!(format!("{t:?}"), "Neg(IntLit(4))");
        let f = b.not(b.tru());
        assert_eq!(format!("{f:?}"), "Not(BoolLit(true))");
    }

    #[test]
    fn expr_equality_is_structural() {
        let b = AstBuilder::new();
        let a = b.add(b.var_term("x"), b.int_term(1));
        let c = b.add(b.var_term("x"), b.int_term(1));
        assert_eq!(a, c);
        assert_ne!(a, b.add(b.var_term("x"), b.int_term(2)));
    }

    #[test]
    fn typed_quantifiers_display_their_sort() {
        let b = AstBuilder::new();
        let body = b.eq(b.var_term("n"), b.int_term(3));
        assert_eq!(
            b.forall_typed("n", BinderSort::Int, body.clone())
                .to_string(),
            "(∀n:Int. (n = 3))"
        );
        assert_eq!(
            b.exists_typed("n", BinderSort::Named("U".into()), body)
                .to_string(),
            "(∃n:U. (n = 3))"
        );
    }

    #[test]
    fn function_and_predicate_application() {
        let b = AstBuilder::new();
        let fx = b.app_term("f", vec![b.var_term("x")]);
        let p = b.pred("P", vec![fx.clone(), b.int_term(0)]);
        assert_eq!(p.to_string(), "P(f(x), 0)");
        assert_eq!(b.pred("Q", vec![]).to_string(), "Q()");
        // Predicate applications are formulas, so they combine with connectives.
        assert_eq!(b.not(p).to_string(), "¬P(f(x), 0)");
    }

    #[test]
    fn walk_visits_every_node_in_preorder() {
        let b = AstBuilder::new();
        let f = b.and(b.var_formula("P"), b.not(b.var_formula("Q")));
        let mut kinds = Vec::new();
        f.walk(|n| {
            kinds.push(match n {
                ExprNode::And(..) => "and",
                ExprNode::Var(_) => "var",
                ExprNode::Not(_) => "not",
                _ => "other",
            })
        });
        assert_eq!(kinds, ["and", "var", "not", "var"]);
    }

    #[test]
    fn free_vars_respect_binders_and_skip_names() {
        let b = AstBuilder::new();
        // ∀x. (x < y) ∧ P(z)  — free: y, z (P is a name, not a variable)
        let body = b.and(
            b.lt(b.var_term("x"), b.var_term("y")),
            b.pred("P", vec![b.var_term("z")]),
        );
        let f = b.forall("x", body);
        let free: Vec<String> = f.free_vars().into_iter().collect();
        assert_eq!(free, ["y", "z"]);
    }

    #[test]
    fn substitute_replaces_free_occurrences_only() {
        let b = AstBuilder::new();
        // (x < 1) ∧ ∀x. (x < 1) — the bound x is not replaced.
        let lt = b.lt(b.var_term("x"), b.int_term(1));
        let f = b.and(lt.clone(), b.forall("x", lt));
        let g = f.substitute("x", &b.int_term(7));
        assert_eq!(g.to_string(), "((7 < 1) ∧ (∀x. (x < 1)))");
    }

    #[test]
    fn substitute_avoids_variable_capture() {
        let b = AstBuilder::new();
        // (∃y. x = y)[x := y] would capture the y; the binder is renamed.
        let f = b.exists("y", b.eq(b.var_term("x"), b.var_term("y")));
        let g = f.substitute("x", &b.var_term("y"));
        assert_eq!(g.to_string(), "(∃y'. (y = y'))");
        assert_eq!(g.free_vars().into_iter().collect::<Vec<_>>(), ["y"]);
    }

    #[test]
    fn substitute_keeps_binder_sorts_when_renaming() {
        let b = AstBuilder::new();
        let body = b.eq(b.var_term("x"), b.var_term("y"));
        let f = b.forall_typed("y", BinderSort::Real, body);
        let g = f.substitute("x", &b.var_term("y"));
        assert_eq!(g.to_string(), "(∀y':Real. (y = y'))");
    }

    #[test]
    fn substitute_a_term_into_a_formula_is_kind_checked() {
        let b = AstBuilder::new();
        let f = b.lt(b.var_term("x"), b.int_term(0));
        // Substituting a term into a term-level position is well-typed.
        let g: Formula = f.substitute("x", &b.app_term("g", vec![]));
        assert_eq!(g.to_string(), "(g() < 0)");
    }

    #[test]
    fn node_accessor_exposes_structure() {
        let b = AstBuilder::new();
        let f = b.tru();
        assert!(matches!(f.node(), ExprNode::BoolLit(true)));
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
        assert_eq!(f.to_string(), "(∀x. ((x > 0) → ((x * x) > 0)))");
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

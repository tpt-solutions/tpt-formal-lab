//! Conversion from `tpt-fl-proof-ast` formulas and terms to SMT expressions.
//!
//! The proof AST does not record the sort of a variable, so the caller supplies
//! one for each free variable. Quantifier binders must carry a sort, which the
//! AST records when the binder is typed.

use std::fmt;

use tpt_fl_proof_ast::{ArithOp, Binder, BinderSort, CmpOp, Expr as AstExpr, ExprNode, Kind};

use crate::{Expr, Sort};

/// Errors from [`from_proof_ast`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConvertError {
    /// A free variable has no sort in the lookup supplied by the caller.
    UnknownVariable(String),
    /// A quantifier binder has no declared sort.
    UntypedBinder(String),
    /// A binder uses a sort that has no SMT-LIB2 counterpart in this crate.
    UnsupportedSort(String),
}

impl fmt::Display for ConvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConvertError::UnknownVariable(name) => {
                write!(f, "no sort supplied for free variable `{name}`")
            }
            ConvertError::UntypedBinder(name) => {
                write!(f, "quantifier binder `{name}` has no declared sort")
            }
            ConvertError::UnsupportedSort(name) => {
                write!(f, "sort `{name}` has no SMT-LIB2 counterpart")
            }
        }
    }
}

impl std::error::Error for ConvertError {}

/// Converts a proof-AST formula or term to an SMT expression.
///
/// `sort_of` gives the sort of each free variable, and returns `None` for a
/// name it does not know. Bound variables take the sort declared on their
/// binder, so they never reach `sort_of`.
///
/// The mapping is direct. `Ne` becomes `(not (= a b))`, and `Div` becomes real
/// division `(/ a b)`. For integer division, build the expression with
/// [`Expr::idiv`] instead.
///
/// # Errors
///
/// Returns [`ConvertError::UnknownVariable`] for a free variable that `sort_of`
/// does not know, [`ConvertError::UntypedBinder`] for a quantifier without a
/// sort, and [`ConvertError::UnsupportedSort`] for a named binder sort.
///
/// # Example
///
/// ```rust
/// use tpt_fl_proof_ast::{AstBuilder, BinderSort};
/// use tpt_fl_smt_bridge::{emit_smtlib2, from_proof_ast, Sort};
///
/// let b = AstBuilder::new();
/// let body = b.gt(b.var_term("n"), b.int_term(0));
/// let f = b.forall_typed("n", BinderSort::Int, body);
///
/// let smt = from_proof_ast(&f, |_| None).unwrap();
/// assert_eq!(emit_smtlib2(&smt), "(forall ((n Int)) (> n 0))");
/// ```
pub fn from_proof_ast<K: Kind>(
    ast: &AstExpr<K>,
    sort_of: impl Fn(&str) -> Option<Sort>,
) -> Result<Expr, ConvertError> {
    let mut converter = Converter {
        sort_of: &sort_of,
        bound: Vec::new(),
    };
    converter.node(ast.node())
}

struct Converter<'a> {
    sort_of: &'a dyn Fn(&str) -> Option<Sort>,
    bound: Vec<(String, Sort)>,
}

impl Converter<'_> {
    fn node(&mut self, node: &ExprNode) -> Result<Expr, ConvertError> {
        Ok(match node {
            ExprNode::Var(name) => Expr::var(name.as_str(), self.var_sort(name)?),
            ExprNode::IntLit(n) => Expr::int(*n),
            ExprNode::BoolLit(b) => Expr::bool(*b),
            ExprNode::BinArith(op, a, b) => {
                let (a, b) = (self.node(a)?, self.node(b)?);
                match op {
                    ArithOp::Add => Expr::add(a, b),
                    ArithOp::Sub => Expr::sub(a, b),
                    ArithOp::Mul => Expr::mul(a, b),
                    ArithOp::Div => Expr::div(a, b),
                }
            }
            ExprNode::Neg(a) => Expr::neg(self.node(a)?),
            ExprNode::App(name, args) | ExprNode::Pred(name, args) => {
                Expr::apply(name.as_str(), self.nodes(args)?)
            }
            ExprNode::And(a, b) => Expr::and(vec![self.node(a)?, self.node(b)?]),
            ExprNode::Or(a, b) => Expr::or(vec![self.node(a)?, self.node(b)?]),
            ExprNode::Not(a) => Expr::not(self.node(a)?),
            ExprNode::Implies(a, b) => Expr::implies(self.node(a)?, self.node(b)?),
            ExprNode::Iff(a, b) => Expr::iff(self.node(a)?, self.node(b)?),
            ExprNode::Cmp(op, a, b) => {
                let (a, b) = (self.node(a)?, self.node(b)?);
                match op {
                    CmpOp::Eq => Expr::eq(a, b),
                    CmpOp::Ne => Expr::not(Expr::eq(a, b)),
                    CmpOp::Lt => Expr::lt(a, b),
                    CmpOp::Le => Expr::le(a, b),
                    CmpOp::Gt => Expr::gt(a, b),
                    CmpOp::Ge => Expr::ge(a, b),
                }
            }
            ExprNode::Forall(binder, body) => self.quantified(binder, body, Expr::forall)?,
            ExprNode::Exists(binder, body) => self.quantified(binder, body, Expr::exists)?,
        })
    }

    fn nodes(&mut self, nodes: &[std::sync::Arc<ExprNode>]) -> Result<Vec<Expr>, ConvertError> {
        nodes.iter().map(|n| self.node(n)).collect()
    }

    /// Converts a quantifier body with the binder in scope, then builds the
    /// quantified expression with `build`.
    fn quantified(
        &mut self,
        binder: &Binder,
        body: &ExprNode,
        build: fn(Vec<(String, Sort)>, Expr) -> Expr,
    ) -> Result<Expr, ConvertError> {
        let sort = binder_sort(binder)?;
        self.bound.push((binder.name.clone(), sort.clone()));
        let body = self.node(body);
        self.bound.pop();
        Ok(build(vec![(binder.name.clone(), sort)], body?))
    }

    /// The sort of a variable: the innermost binder with that name, or else the
    /// caller's lookup.
    fn var_sort(&self, name: &str) -> Result<Sort, ConvertError> {
        if let Some((_, sort)) = self.bound.iter().rev().find(|(n, _)| n == name) {
            return Ok(sort.clone());
        }
        (self.sort_of)(name).ok_or_else(|| ConvertError::UnknownVariable(name.to_owned()))
    }
}

fn binder_sort(binder: &Binder) -> Result<Sort, ConvertError> {
    match &binder.sort {
        None => Err(ConvertError::UntypedBinder(binder.name.clone())),
        Some(BinderSort::Int) => Ok(Sort::Int),
        Some(BinderSort::Real) => Ok(Sort::Real),
        Some(BinderSort::Bool) => Ok(Sort::Bool),
        Some(BinderSort::Named(name)) => Err(ConvertError::UnsupportedSort(name.clone())),
    }
}

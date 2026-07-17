//! SMT-LIB2 string emission.

use crate::expr::{Expr, ExprInner};

/// Converts an [`Expr`] into its SMT-LIB2 string representation.
///
/// The output is a single valid SMT-LIB2 s-expression, suitable for embedding
/// in a `(assert ...)` command or evaluating directly.
///
/// # Example
///
/// ```rust
/// use tpt_smt_bridge::{Expr, Sort, emit_smtlib2};
///
/// let x = Expr::var("x", Sort::Int);
/// let constraint = Expr::gt(x, Expr::int(0));
/// assert_eq!(emit_smtlib2(&constraint), "(> x 0)");
/// ```
pub fn emit_smtlib2(expr: &Expr) -> String {
    emit_inner(&expr.inner)
}

fn emit_inner(inner: &ExprInner) -> String {
    match inner {
        ExprInner::BoolLit(b) => {
            if *b {
                "true".into()
            } else {
                "false".into()
            }
        }
        ExprInner::IntLit(n) => {
            if *n < 0 {
                format!("(- {})", n.unsigned_abs())
            } else {
                n.to_string()
            }
        }
        ExprInner::RealLit(n, d) => {
            if *d == 1 {
                format!("{n}.0")
            } else {
                format!("(/ {n}.0 {d}.0)")
            }
        }
        ExprInner::BitVecLit(v, w) => format!("(_ bv{v} {w})"),
        ExprInner::Var(name, _sort) => name.clone(),

        ExprInner::Not(e) => format!("(not {})", emit_inner(&e.inner)),
        ExprInner::And(args) => nary("and", args),
        ExprInner::Or(args) => nary("or", args),
        ExprInner::Implies(a, b) => binary("=>", a, b),
        ExprInner::Iff(a, b) => binary("=", a, b),

        ExprInner::Eq(a, b) => binary("=", a, b),
        ExprInner::Distinct(args) => nary("distinct", args),
        ExprInner::Lt(a, b) => binary("<", a, b),
        ExprInner::Le(a, b) => binary("<=", a, b),
        ExprInner::Gt(a, b) => binary(">", a, b),
        ExprInner::Ge(a, b) => binary(">=", a, b),

        ExprInner::Add(args) => nary("+", args),
        ExprInner::Sub(a, b) => binary("-", a, b),
        ExprInner::Mul(args) => nary("*", args),
        ExprInner::Div(a, b) => binary("/", a, b),
        ExprInner::Mod(a, b) => binary("mod", a, b),
        ExprInner::Neg(e) => format!("(- {})", emit_inner(&e.inner)),
        ExprInner::Abs(e) => format!("(abs {})", emit_inner(&e.inner)),

        ExprInner::Forall(vars, body) => quantifier("forall", vars, body),
        ExprInner::Exists(vars, body) => quantifier("exists", vars, body),

        ExprInner::Ite(c, t, e) => format!(
            "(ite {} {} {})",
            emit_inner(&c.inner),
            emit_inner(&t.inner),
            emit_inner(&e.inner)
        ),
    }
}

fn binary(op: &str, a: &Expr, b: &Expr) -> String {
    format!("({op} {} {})", emit_inner(&a.inner), emit_inner(&b.inner))
}

fn nary(op: &str, args: &[Expr]) -> String {
    let parts: Vec<String> = args.iter().map(|e| emit_inner(&e.inner)).collect();
    format!("({op} {})", parts.join(" "))
}

fn quantifier(q: &str, vars: &[(String, crate::sort::Sort)], body: &Expr) -> String {
    let bindings: Vec<String> = vars
        .iter()
        .map(|(name, sort)| format!("({name} {})", sort.to_smtlib2()))
        .collect();
    format!("({q} ({}) {})", bindings.join(" "), emit_inner(&body.inner))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Expr, Sort};

    #[test]
    fn bool_literals() {
        assert_eq!(emit_smtlib2(&Expr::bool(true)), "true");
        assert_eq!(emit_smtlib2(&Expr::bool(false)), "false");
    }

    #[test]
    fn int_literal_positive() {
        assert_eq!(emit_smtlib2(&Expr::int(42)), "42");
    }

    #[test]
    fn int_literal_negative() {
        assert_eq!(emit_smtlib2(&Expr::int(-5)), "(- 5)");
    }

    #[test]
    fn variable() {
        assert_eq!(emit_smtlib2(&Expr::var("x", Sort::Int)), "x");
    }

    #[test]
    fn not() {
        let e = Expr::not(Expr::bool(true));
        assert_eq!(emit_smtlib2(&e), "(not true)");
    }

    #[test]
    fn and_two() {
        let e = Expr::and(vec![Expr::bool(true), Expr::bool(false)]);
        assert_eq!(emit_smtlib2(&e), "(and true false)");
    }

    #[test]
    fn gt() {
        let x = Expr::var("x", Sort::Int);
        let e = Expr::gt(x, Expr::int(0));
        assert_eq!(emit_smtlib2(&e), "(> x 0)");
    }

    #[test]
    fn add() {
        let x = Expr::var("x", Sort::Int);
        let y = Expr::var("y", Sort::Int);
        let e = Expr::add(x, y);
        assert_eq!(emit_smtlib2(&e), "(+ x y)");
    }

    #[test]
    fn forall() {
        let body = Expr::ge(Expr::var("x", Sort::Int), Expr::int(0));
        let e = Expr::forall(vec![("x".into(), Sort::Int)], body);
        assert_eq!(emit_smtlib2(&e), "(forall ((x Int)) (>= x 0))");
    }

    #[test]
    fn exists() {
        let body = Expr::eq(Expr::var("x", Sort::Int), Expr::int(5));
        let e = Expr::exists(vec![("x".into(), Sort::Int)], body);
        assert_eq!(emit_smtlib2(&e), "(exists ((x Int)) (= x 5))");
    }

    #[test]
    fn ite() {
        let cond = Expr::gt(Expr::var("x", Sort::Int), Expr::int(0));
        let e = Expr::ite(cond, Expr::int(1), Expr::int(-1));
        assert_eq!(emit_smtlib2(&e), "(ite (> x 0) 1 (- 1))");
    }

    #[test]
    fn bitvec_literal() {
        let e = Expr::bitvec(255, 8);
        assert_eq!(emit_smtlib2(&e), "(_ bv255 8)");
    }
}

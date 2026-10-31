//! SMT-LIB2 string emission.

use crate::expr::{Expr, ExprInner};
use crate::sort::Sort;

/// Converts an [`Expr`] into its SMT-LIB2 string representation.
///
/// The output is a single valid SMT-LIB2 s-expression, suitable for embedding
/// in a `(assert ...)` command or evaluating directly.
///
/// # Example
///
/// ```rust
/// use out_smt_bridge::{Expr, Sort, emit_smtlib2};
///
/// let x = Expr::var("x", Sort::Int);
/// let constraint = Expr::gt(x, Expr::int(0));
/// assert_eq!(emit_smtlib2(&constraint), "(> x 0)");
/// ```
pub fn emit_smtlib2(expr: &Expr) -> String {
    emit_inner(&expr.inner)
}

/// Writes `name` as an SMT-LIB2 symbol.
///
/// A simple symbol is written as-is. Any other name is wrapped in `|...|`,
/// which is how SMT-LIB2 writes names containing spaces or other reserved
/// characters.
///
/// # Panics
///
/// Panics if `name` contains `|` or `\`. Those cannot appear inside a quoted
/// symbol, so no spelling of the name would be valid.
///
/// # Example
///
/// ```rust
/// use out_smt_bridge::quote_symbol;
///
/// assert_eq!(quote_symbol("x_1"), "x_1");
/// assert_eq!(quote_symbol("a b"), "|a b|");
/// assert_eq!(quote_symbol("1st"), "|1st|");
/// ```
pub fn quote_symbol(name: &str) -> String {
    if is_simple_symbol(name) {
        name.to_string()
    } else {
        assert!(
            !name.contains(['|', '\\']),
            "symbol `{name}` cannot contain `|` or `\\`"
        );
        format!("|{name}|")
    }
}

/// Returns `true` if `name` is a simple SMT-LIB2 symbol.
pub(crate) fn is_simple_symbol(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if !c.is_ascii_digit() && is_symbol_char(c) => {}
        _ => return false,
    }
    chars.all(is_symbol_char)
}

fn is_symbol_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "~!@$%^&*_-+=<>.?/".contains(c)
}

/// Writes a string literal, escaping `"` by doubling it and writing any
/// character outside printable ASCII as `\u{...}`. A backslash is also written
/// as an escape, because `\` starts an escape sequence in SMT-LIB2 strings.
pub(crate) fn string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\"\""),
            '\\' => out.push_str("\\u{5c}"),
            ' '..='~' => out.push(c),
            _ => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
        }
    }
    out.push('"');
    out
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
        ExprInner::RealLit(n, d) => real_text(*n, *d),
        ExprInner::BitVecLit(v, w) => {
            let masked = if *w >= 64 { *v } else { *v & ((1u64 << w) - 1) };
            format!("(_ bv{masked} {w})")
        }
        ExprInner::StrLit(s) => string_literal(s),
        ExprInner::Var(name, _sort) => quote_symbol(name),

        ExprInner::Not(e) => format!("(not {})", emit_inner(&e.inner)),
        ExprInner::And(args) => normalised_nary("and", "true", args),
        ExprInner::Or(args) => normalised_nary("or", "false", args),
        ExprInner::Implies(a, b) => binary("=>", a, b),
        ExprInner::Iff(a, b) => binary("=", a, b),

        ExprInner::Eq(a, b) => binary("=", a, b),
        ExprInner::Distinct(args) => {
            if args.len() < 2 {
                "true".into()
            } else {
                nary("distinct", args)
            }
        }
        ExprInner::Lt(a, b) => binary("<", a, b),
        ExprInner::Le(a, b) => binary("<=", a, b),
        ExprInner::Gt(a, b) => binary(">", a, b),
        ExprInner::Ge(a, b) => binary(">=", a, b),

        ExprInner::Add(args) => normalised_nary("+", "0", args),
        ExprInner::Sub(a, b) => binary("-", a, b),
        ExprInner::Mul(args) => normalised_nary("*", "1", args),
        ExprInner::RealDiv(a, b) => binary("/", a, b),
        ExprInner::IntDiv(a, b) => binary("div", a, b),
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

        ExprInner::Select(a, i) => binary("select", a, i),
        ExprInner::Store(a, i, v) => format!(
            "(store {} {} {})",
            emit_inner(&a.inner),
            emit_inner(&i.inner),
            emit_inner(&v.inner)
        ),

        ExprInner::Apply(name, args) => {
            if args.is_empty() {
                quote_symbol(name)
            } else {
                nary(&quote_symbol(name), args)
            }
        }

        ExprInner::Let(bindings, body) => {
            let bound: Vec<String> = bindings
                .iter()
                .map(|(name, value)| {
                    format!("({} {})", quote_symbol(name), emit_inner(&value.inner))
                })
                .collect();
            format!("(let ({}) {})", bound.join(" "), emit_inner(&body.inner))
        }
    }
}

/// Text for a real literal: `3.0`, `(- 3.0)`, `(/ 1.0 2.0)` or `(/ (- 1.0) 2.0)`.
fn real_text(numer: i64, denom: u64) -> String {
    let num = if numer < 0 {
        format!("(- {}.0)", numer.unsigned_abs())
    } else {
        format!("{numer}.0")
    };
    if denom == 1 {
        num
    } else {
        format!("(/ {num} {denom}.0)")
    }
}

fn binary(op: &str, a: &Expr, b: &Expr) -> String {
    format!("({op} {} {})", emit_inner(&a.inner), emit_inner(&b.inner))
}

fn nary(op: &str, args: &[Expr]) -> String {
    let parts: Vec<String> = args.iter().map(|e| emit_inner(&e.inner)).collect();
    format!("({op} {})", parts.join(" "))
}

/// Emits an n-ary operator, using its identity for no arguments and the
/// argument itself for one.
fn normalised_nary(op: &str, identity: &str, args: &[Expr]) -> String {
    match args {
        [] => identity.to_string(),
        [only] => emit_inner(&only.inner),
        _ => nary(op, args),
    }
}

fn quantifier(q: &str, vars: &[(String, Sort)], body: &Expr) -> String {
    let bindings: Vec<String> = vars
        .iter()
        .map(|(name, sort)| format!("({} {})", quote_symbol(name), sort.to_smtlib2()))
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
    fn int_literal_extremes_and_bignum() {
        assert_eq!(
            emit_smtlib2(&Expr::int(i64::MIN)),
            "(- 9223372036854775808)"
        );
        assert_eq!(
            emit_smtlib2(&Expr::int128(i128::MIN)),
            "(- 170141183460469231731687303715884105728)"
        );
    }

    #[test]
    fn real_literals_including_negatives() {
        assert_eq!(emit_smtlib2(&Expr::real(3, 1)), "3.0");
        assert_eq!(emit_smtlib2(&Expr::real(1, 2)), "(/ 1.0 2.0)");
        assert_eq!(emit_smtlib2(&Expr::real(-3, 1)), "(- 3.0)");
        assert_eq!(emit_smtlib2(&Expr::real(-1, 2)), "(/ (- 1.0) 2.0)");
    }

    #[test]
    #[should_panic(expected = "denominator must be non-zero")]
    fn real_zero_denominator_is_rejected() {
        let _ = Expr::real(1, 0);
    }

    #[test]
    fn bitvec_literal() {
        let e = Expr::bitvec(255, 8);
        assert_eq!(emit_smtlib2(&e), "(_ bv255 8)");
    }

    #[test]
    fn bitvec_value_is_masked_to_width() {
        assert_eq!(emit_smtlib2(&Expr::bitvec(0x1ff, 8)), "(_ bv255 8)");
        assert_eq!(
            emit_smtlib2(&Expr::bitvec(u64::MAX, 64)),
            format!("(_ bv{} 64)", u64::MAX)
        );
    }

    #[test]
    #[should_panic(expected = "width must be between 1 and 64")]
    fn bitvec_zero_width_is_rejected() {
        let _ = Expr::bitvec(0, 0);
    }

    #[test]
    #[should_panic(expected = "width must be between 1 and 64")]
    fn bitvec_wider_than_u64_is_rejected() {
        let _ = Expr::bitvec(0, 65);
    }

    #[test]
    fn string_literals_are_escaped() {
        assert_eq!(emit_smtlib2(&Expr::string("ab")), "\"ab\"");
        assert_eq!(emit_smtlib2(&Expr::string("a\"b")), "\"a\"\"b\"");
        assert_eq!(
            emit_smtlib2(&Expr::string("tab\there")),
            "\"tab\\u{9}here\""
        );
        assert_eq!(
            emit_smtlib2(&Expr::string("back\\slash")),
            "\"back\\u{5c}slash\""
        );
        assert_eq!(emit_smtlib2(&Expr::string("é")), "\"\\u{e9}\"");
    }

    #[test]
    fn variable() {
        assert_eq!(emit_smtlib2(&Expr::var("x", Sort::Int)), "x");
    }

    #[test]
    fn unusual_names_are_quoted() {
        assert_eq!(emit_smtlib2(&Expr::var("my var", Sort::Int)), "|my var|");
        assert_eq!(emit_smtlib2(&Expr::var("1st", Sort::Int)), "|1st|");
        assert_eq!(emit_smtlib2(&Expr::var("x'", Sort::Int)), "|x'|");
    }

    #[test]
    #[should_panic(expected = "cannot contain")]
    fn names_with_pipe_are_rejected() {
        let _ = quote_symbol("a|b");
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
    fn empty_and_single_argument_forms_are_normalised() {
        assert_eq!(emit_smtlib2(&Expr::and(vec![])), "true");
        assert_eq!(emit_smtlib2(&Expr::or(vec![])), "false");
        assert_eq!(emit_smtlib2(&Expr::add_many(vec![])), "0");
        assert_eq!(emit_smtlib2(&Expr::add_many(vec![Expr::int(4)])), "4");
        assert_eq!(emit_smtlib2(&Expr::and(vec![Expr::bool(false)])), "false");
        let mul_empty = Expr {
            inner: std::sync::Arc::new(ExprInner::Mul(vec![])),
        };
        assert_eq!(emit_smtlib2(&mul_empty), "1");
        assert_eq!(emit_smtlib2(&Expr::distinct(vec![Expr::int(1)])), "true");
    }

    #[test]
    fn gt() {
        let x = Expr::var("x", Sort::Int);
        let e = Expr::gt(x, Expr::int(0));
        assert_eq!(emit_smtlib2(&e), "(> x 0)");
    }

    #[test]
    fn all_comparison_operators() {
        let a = || Expr::var("a", Sort::Int);
        let b = || Expr::var("b", Sort::Int);
        assert_eq!(emit_smtlib2(&Expr::eq(a(), b())), "(= a b)");
        assert_eq!(emit_smtlib2(&Expr::lt(a(), b())), "(< a b)");
        assert_eq!(emit_smtlib2(&Expr::le(a(), b())), "(<= a b)");
        assert_eq!(emit_smtlib2(&Expr::gt(a(), b())), "(> a b)");
        assert_eq!(emit_smtlib2(&Expr::ge(a(), b())), "(>= a b)");
        assert_eq!(
            emit_smtlib2(&Expr::distinct(vec![a(), b()])),
            "(distinct a b)"
        );
    }

    #[test]
    fn all_arithmetic_operators() {
        let a = || Expr::var("a", Sort::Int);
        let b = || Expr::var("b", Sort::Int);
        assert_eq!(emit_smtlib2(&Expr::add(a(), b())), "(+ a b)");
        assert_eq!(emit_smtlib2(&Expr::sub(a(), b())), "(- a b)");
        assert_eq!(emit_smtlib2(&Expr::mul(a(), b())), "(* a b)");
        assert_eq!(emit_smtlib2(&Expr::idiv(a(), b())), "(div a b)");
        assert_eq!(emit_smtlib2(&Expr::rdiv(a(), b())), "(/ a b)");
        assert_eq!(emit_smtlib2(&Expr::modulo(a(), b())), "(mod a b)");
        assert_eq!(emit_smtlib2(&Expr::neg(a())), "(- a)");
        assert_eq!(emit_smtlib2(&Expr::abs(a())), "(abs a)");
    }

    #[test]
    fn all_propositional_operators() {
        let p = || Expr::var("p", Sort::Bool);
        let q = || Expr::var("q", Sort::Bool);
        assert_eq!(emit_smtlib2(&Expr::or(vec![p(), q()])), "(or p q)");
        assert_eq!(emit_smtlib2(&Expr::implies(p(), q())), "(=> p q)");
        assert_eq!(emit_smtlib2(&Expr::iff(p(), q())), "(= p q)");
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
    fn multi_binder_quantifier() {
        let body = Expr::lt(Expr::var("x", Sort::Int), Expr::var("y", Sort::Real));
        let vars = vec![("x".into(), Sort::Int), ("y".into(), Sort::Real)];
        assert_eq!(
            emit_smtlib2(&Expr::forall(vars, body)),
            "(forall ((x Int) (y Real)) (< x y))"
        );
    }

    #[test]
    #[should_panic(expected = "at least one binder")]
    fn empty_quantifier_binders_are_rejected() {
        let _ = Expr::forall(vec![], Expr::bool(true));
    }

    #[test]
    fn ite() {
        let cond = Expr::gt(Expr::var("x", Sort::Int), Expr::int(0));
        let e = Expr::ite(cond, Expr::int(1), Expr::int(-1));
        assert_eq!(emit_smtlib2(&e), "(ite (> x 0) 1 (- 1))");
    }

    #[test]
    fn arrays() {
        let a = || Expr::var("a", Sort::Array(Box::new(Sort::Int), Box::new(Sort::Int)));
        let stored = Expr::store(a(), Expr::int(1), Expr::int(7));
        assert_eq!(emit_smtlib2(&stored), "(store a 1 7)");
        assert_eq!(
            emit_smtlib2(&Expr::select(stored, Expr::int(1))),
            "(select (store a 1 7) 1)"
        );
    }

    #[test]
    fn function_application_and_constant_symbols() {
        assert_eq!(
            emit_smtlib2(&Expr::apply("f", vec![Expr::int(1), Expr::int(2)])),
            "(f 1 2)"
        );
        assert_eq!(emit_smtlib2(&Expr::apply("c", vec![])), "c");
        assert_eq!(
            emit_smtlib2(&Expr::apply("my f", vec![Expr::int(1)])),
            "(|my f| 1)"
        );
    }

    #[test]
    fn let_bindings() {
        let x = Expr::var("x", Sort::Int);
        let e = Expr::let_in(
            vec![("t".into(), Expr::add(x.clone(), x))],
            Expr::var("t", Sort::Int),
        );
        assert_eq!(emit_smtlib2(&e), "(let ((t (+ x x))) t)");
    }

    #[test]
    fn bitvector_and_string_operators() {
        let b = || Expr::var("b", Sort::BitVec(8));
        assert_eq!(emit_smtlib2(&Expr::bvadd(b(), b())), "(bvadd b b)");
        assert_eq!(emit_smtlib2(&Expr::bvnot(b())), "(bvnot b)");
        assert_eq!(emit_smtlib2(&Expr::bvult(b(), b())), "(bvult b b)");
        let s = || Expr::var("s", Sort::String);
        assert_eq!(
            emit_smtlib2(&Expr::str_concat(vec![s(), Expr::string("!")])),
            "(str.++ s \"!\")"
        );
        assert_eq!(emit_smtlib2(&Expr::str_len(s())), "(str.len s)");
    }
}

//! Tests for the `tpt-fl-proof-ast` → SMT conversion.

use tpt_fl_proof_ast::{AstBuilder, BinderSort};
use tpt_fl_smt_bridge::{emit_smtlib2, from_proof_ast, ConvertError, Sort};

fn ints(name: &str) -> Option<Sort> {
    (name == "x" || name == "y").then_some(Sort::Int)
}

#[test]
fn quantified_formula_keeps_binder_sort() {
    let b = AstBuilder::new();
    let f = b.forall_typed("n", BinderSort::Int, b.gt(b.var_term("n"), b.int_term(0)));
    let smt = from_proof_ast(&f, |_| None).unwrap();
    assert_eq!(emit_smtlib2(&smt), "(forall ((n Int)) (> n 0))");
}

#[test]
fn free_variables_take_sorts_from_the_lookup() {
    let b = AstBuilder::new();
    let f = b.lt(b.var_term("x"), b.add(b.var_term("y"), b.int_term(1)));
    let smt = from_proof_ast(&f, ints).unwrap();
    assert_eq!(emit_smtlib2(&smt), "(< x (+ y 1))");
}

#[test]
fn unknown_free_variable_is_an_error() {
    let b = AstBuilder::new();
    let f = b.eq(b.var_term("z"), b.int_term(0));
    assert_eq!(
        from_proof_ast(&f, ints),
        Err(ConvertError::UnknownVariable("z".to_owned()))
    );
}

#[test]
fn untyped_binder_is_an_error() {
    let b = AstBuilder::new();
    let f = b.forall("n", b.gt(b.var_term("n"), b.int_term(0)));
    assert_eq!(
        from_proof_ast(&f, |_| None),
        Err(ConvertError::UntypedBinder("n".to_owned()))
    );
}

#[test]
fn named_binder_sort_is_unsupported() {
    let b = AstBuilder::new();
    let body = b.pred("p", vec![b.var_term("u")]);
    let f = b.forall_typed("u", BinderSort::Named("U".to_owned()), body);
    assert_eq!(
        from_proof_ast(&f, |_| None),
        Err(ConvertError::UnsupportedSort("U".to_owned()))
    );
}

#[test]
fn inner_binder_shadows_free_variable_sort() {
    // Outer x is Int from the lookup; the inner binder redeclares x as Bool.
    let b = AstBuilder::new();
    let inner = b.forall_typed("x", BinderSort::Bool, b.var_formula("x"));
    let f = b.and(b.eq(b.var_term("x"), b.int_term(0)), inner);
    let smt = from_proof_ast(&f, ints).unwrap();
    assert_eq!(emit_smtlib2(&smt), "(and (= x 0) (forall ((x Bool)) x))");
}

#[test]
fn ne_becomes_negated_equality() {
    let b = AstBuilder::new();
    let f = b.ne(b.var_term("x"), b.int_term(2));
    let smt = from_proof_ast(&f, ints).unwrap();
    assert_eq!(emit_smtlib2(&smt), "(not (= x 2))");
}

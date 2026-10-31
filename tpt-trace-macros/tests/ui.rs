//! Compile-fail tests: invalid `#[traces(...)]` uses must produce errors.

#[test]
fn invalid_uses_are_rejected() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}

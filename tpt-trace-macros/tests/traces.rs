//! Integration tests for `tpt-trace-macros`.

use tpt_trace_macros::traces;

#[traces("REQ-INT-001", "REQ-INT-002")]
pub fn sample_fn() -> u32 {
    42
}

#[traces("REQ-SINGLE")]
pub fn single_req() {}

#[test]
fn generated_const_matches_macro_args() {
    assert_eq!(__TPT_TRACES_SAMPLE_FN, &["REQ-INT-001", "REQ-INT-002"]);
    assert_eq!(__TPT_TRACES_SINGLE_REQ, &["REQ-SINGLE"]);
}

#[test]
fn function_still_works() {
    assert_eq!(sample_fn(), 42);
}

#[allow(non_snake_case)]
#[traces("REQ-CAMEL-001")]
pub fn camelCaseFn() {}

#[test]
fn upper_snake_constant_name() {
    assert_eq!(__TPT_TRACES_CAMEL_CASE_FN, &["REQ-CAMEL-001"]);
}

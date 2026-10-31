//! Contracts on methods, early exits, `?`, `old(...)`, renamed results,
//! `async`/`const`/`unsafe` functions, and loop invariants.

use tpt_fl_verify_macros::{ensures, invariant, loop_invariant, pure, requires};

// ── Early exits and `?` ───────────────────────────────────────────────────────

#[ensures(result >= 0)]
fn clamp_non_negative(x: i32) -> i32 {
    if x < 0 {
        return 0;
    }
    x
}

#[ensures(result > 0)]
fn early_exit_violates(x: i32) -> i32 {
    if x < 0 {
        return 0; // breaks the postcondition on this path
    }
    x
}

#[ensures(result.is_ok() == s.parse::<u16>().is_ok())]
fn parse_port(s: &str) -> Result<u16, std::num::ParseIntError> {
    let n: u16 = s.parse()?;
    Ok(n)
}

#[test]
fn early_return_still_checked_on_satisfying_path() {
    assert_eq!(clamp_non_negative(-5), 0);
    assert_eq!(clamp_non_negative(7), 7);
    assert_eq!(parse_port("8080"), Ok(8080));
    assert!(parse_port("not a port").is_err());
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "Postcondition violated")]
fn early_return_path_violation_panics() {
    early_exit_violates(-1);
}

// ── Methods ───────────────────────────────────────────────────────────────────

struct Stack {
    items: Vec<u32>,
}

impl Stack {
    #[requires(!self.items.is_empty())]
    #[ensures(self.items.len() == old(self.items.len()) - 1)]
    fn pop_checked(&mut self) -> u32 {
        self.items.pop().unwrap_or(0)
    }

    #[ensures(self.items.len() == old(self.items.len()) + 1)]
    fn push(&mut self, v: u32) {
        self.items.push(v);
    }

    #[ensures(self.items.len() == old(self.items.len()) + 1)]
    fn push_forgets(&mut self, _v: u32) {}
}

#[test]
fn method_contracts_with_mut_self_and_old() {
    let mut s = Stack { items: vec![1] };
    s.push(2);
    assert_eq!(s.items.len(), 2);
    assert_eq!(s.pop_checked(), 2);
    assert_eq!(s.items, vec![1]);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "Postcondition violated")]
fn method_postcondition_violation_panics() {
    let mut s = Stack { items: Vec::new() };
    s.push_forgets(1);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "Precondition violated")]
fn method_precondition_violation_panics() {
    let mut s = Stack { items: Vec::new() };
    s.pop_checked();
}

// ── old(...) on free functions ────────────────────────────────────────────────

#[ensures(result == old(v) * 2)]
fn doubled(v: u32) -> u32 {
    v * 2
}

#[test]
fn old_snapshots_the_entry_value() {
    assert_eq!(doubled(21), 42);
}

// ── Renamed result binding ────────────────────────────────────────────────────

#[ensures(|total| total >= 10)]
fn sum_pair(a: u32, b: u32) -> u32 {
    a + b
}

#[test]
fn renamed_result_binding_is_used_by_the_condition() {
    assert_eq!(sum_pair(6, 4), 10);
}

// A parameter may share the name `result`; the postcondition refers to the
// returned value.
#[ensures(result == 5)]
fn echo_named_result(result: i32) -> i32 {
    result
}

#[test]
fn parameter_named_result_does_not_clash() {
    assert_eq!(echo_named_result(5), 5);
}

// ── Generics ──────────────────────────────────────────────────────────────────

#[requires(!xs.is_empty())]
#[ensures(|m| m.is_some())]
fn first_max<T: Ord>(xs: &[T]) -> Option<&T> {
    xs.iter().max()
}

#[test]
fn generic_function_contracts() {
    assert_eq!(first_max(&[3, 9, 4]), Some(&9));
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "Precondition violated")]
fn generic_precondition_violation_panics() {
    let empty: [u8; 0] = [];
    first_max(&empty);
}

// ── Stacked attributes ────────────────────────────────────────────────────────

#[requires(x < 1000)]
#[ensures(result <= 2000)]
#[pure]
fn stacked(x: u32) -> u32 {
    x * 2
}

#[test]
fn stacked_contracts_compose() {
    assert_eq!(stacked(10), 20);
}

// ── async ─────────────────────────────────────────────────────────────────────

/// Polls a future that never suspends to completion.
fn block_on<F: core::future::Future>(fut: F) -> F::Output {
    use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    fn raw_noop() -> RawWaker {
        fn clone(_: *const ()) -> RawWaker {
            raw_noop()
        }
        fn noop(_: *const ()) {}
        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
        RawWaker::new(core::ptr::null(), &VTABLE)
    }

    // SAFETY: the vtable's functions ignore the data pointer, so the waker is
    // sound to use.
    let waker = unsafe { Waker::from_raw(raw_noop()) };
    let mut cx = Context::from_waker(&waker);
    let mut fut = std::pin::pin!(fut);
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

async fn inner_value() -> u32 {
    7
}

#[ensures(result == 7)]
async fn awaits_inner() -> u32 {
    let v = inner_value().await;
    v
}

#[ensures(result >= 0)]
async fn async_early_exit(x: i32) -> i32 {
    if x < 0 {
        return 0;
    }
    x
}

#[test]
fn async_functions_are_supported() {
    assert_eq!(block_on(awaits_inner()), 7);
    assert_eq!(block_on(async_early_exit(-3)), 0);
    assert_eq!(block_on(async_early_exit(4)), 4);
}

// ── const and unsafe ─────────────────────────────────────────────────────────

#[requires(x < 100)]
#[ensures(result == x * 2)]
const fn const_double(x: u32) -> u32 {
    x * 2
}

const DOUBLED: u32 = const_double(21);

#[test]
fn const_functions_are_supported() {
    assert_eq!(DOUBLED, 42);
}

#[ensures(result == *p)]
unsafe fn read_through(p: *const u32) -> u32 {
    *p
}

#[test]
fn unsafe_functions_are_supported() {
    let v = 99u32;
    // SAFETY: `&v` is a valid, aligned pointer to an initialised `u32`.
    let got = unsafe { read_through(&v as *const u32) };
    assert_eq!(got, 99);
}

// ── Invariants on entry and exit ─────────────────────────────────────────────

struct Bounded {
    value: u32,
    max: u32,
}

impl Bounded {
    #[invariant(self.value <= self.max)]
    fn set_unchecked(&mut self, v: u32) {
        self.value = v;
    }

    #[invariant(self.value <= self.max)]
    fn into_value(self) -> u32 {
        self.value
    }
}

#[test]
fn invariant_holds_on_entry_and_exit() {
    let mut b = Bounded { value: 1, max: 5 };
    b.set_unchecked(4);
    assert_eq!(b.value, 4);
    assert_eq!(Bounded { value: 2, max: 5 }.into_value(), 2);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "Invariant violated on exit")]
fn invariant_is_checked_on_exit() {
    let mut b = Bounded { value: 1, max: 5 };
    b.set_unchecked(10);
}

// ── Loop invariants ───────────────────────────────────────────────────────────

fn count_to(n: u32) -> u32 {
    let mut i = 0;
    while i < n {
        i += 1;
        loop_invariant!(i <= n);
    }
    i
}

#[test]
fn loop_invariant_holds() {
    assert_eq!(count_to(5), 5);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "Loop invariant violated")]
fn loop_invariant_violation_panics() {
    let mut i = 0;
    while i < 3 {
        i += 1;
        loop_invariant!(i < 2);
    }
}

// ── Conditions containing braces ──────────────────────────────────────────────

#[ensures(match result { Some(v) => v > 0, None => true })]
fn brace_condition(x: i32) -> Option<i32> {
    if x > 0 {
        Some(x)
    } else {
        None
    }
}

#[test]
fn condition_with_braces_is_accepted() {
    assert_eq!(brace_condition(3), Some(3));
    assert_eq!(brace_condition(-1), None);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "Postcondition violated")]
fn braced_condition_violation_panics_with_message() {
    #[ensures(match result { Some(v) => v > 0, None => false })]
    fn always_none(_x: i32) -> Option<i32> {
        None
    }
    always_none(1);
}

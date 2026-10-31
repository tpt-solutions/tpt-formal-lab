//! Items the macro must accept beyond plain free functions: raw identifiers,
//! methods, generics and non-public functions.

use tpt_trace_macros::traces;

#[traces("REQ-RAW-1")]
pub fn r#type() -> u8 {
    1
}

#[test]
fn raw_identifier_constant_name() {
    assert_eq!(__TPT_TRACES_TYPE, &["REQ-RAW-1"]);
    assert_eq!(r#type(), 1);
}

pub struct Gauge {
    level: u32,
}

impl Gauge {
    #[traces("REQ-GAUGE-1", "REQ-GAUGE-2")]
    pub fn read(&self) -> u32 {
        self.level
    }

    #[traces("REQ-GAUGE-3")]
    fn private_reset(&mut self) {
        self.level = 0;
    }
}

struct Other;

impl Other {
    // Same method name as on `Gauge`: the associated constants do not clash.
    #[traces("REQ-OTHER-1")]
    pub fn read(&self) -> u32 {
        7
    }
}

#[test]
fn method_constants_are_associated_and_do_not_collide() {
    assert_eq!(Gauge::__TPT_TRACES_READ, &["REQ-GAUGE-1", "REQ-GAUGE-2"]);
    assert_eq!(Other::__TPT_TRACES_READ, &["REQ-OTHER-1"]);
    assert_eq!(Gauge { level: 5 }.read(), 5);
    assert_eq!(Other.read(), 7);
}

#[test]
fn private_method_keeps_its_constant() {
    assert_eq!(Gauge::__TPT_TRACES_PRIVATE_RESET, &["REQ-GAUGE-3"]);
    let mut g = Gauge { level: 9 };
    g.private_reset();
    assert_eq!(g.read(), 0);
}

#[traces("REQ-GEN-1")]
pub fn largest<T: PartialOrd + Copy>(xs: &[T]) -> Option<T> {
    let mut it = xs.iter().copied();
    let first = it.next()?;
    Some(it.fold(first, |a, b| if b > a { b } else { a }))
}

#[test]
fn generic_function_keeps_its_constant() {
    assert_eq!(__TPT_TRACES_LARGEST, &["REQ-GEN-1"]);
    assert_eq!(largest(&[1, 3, 2]), Some(3));
}

#[traces("REQ-PRIV-1")]
fn hidden() -> &'static str {
    "hidden"
}

#[traces("REQ-CRATE-1")]
pub(crate) fn crate_visible() {}

#[test]
fn non_public_functions_are_supported() {
    assert_eq!(__TPT_TRACES_HIDDEN, &["REQ-PRIV-1"]);
    assert_eq!(hidden(), "hidden");
    assert_eq!(__TPT_TRACES_CRATE_VISIBLE, &["REQ-CRATE-1"]);
    crate_visible();
}

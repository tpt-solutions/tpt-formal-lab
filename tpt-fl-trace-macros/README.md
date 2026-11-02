# tpt-fl-trace-macros

[![crates.io](https://img.shields.io/crates/v/tpt-fl-trace-macros.svg)](https://crates.io/crates/tpt-fl-trace-macros)
[![docs.rs](https://docs.rs/tpt-fl-trace-macros/badge.svg)](https://docs.rs/tpt-fl-trace-macros)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

The `#[traces("REQ-…")]` attribute macro: link code to the requirements it implements.

**Requirement links appear in the generated documentation, and they are also available
as constants that trace-coverage tools can read.**

## Contents

- [The problem](#the-problem)
- [What the macro does](#what-the-macro-does)
- [Quick start](#quick-start)
- [Reading the trace constants](#reading-the-trace-constants)
- [Supported items](#supported-items)
- [Compile errors](#compile-errors)
- [Guarantees](#guarantees)
- [Limitations](#limitations)
- [Crate features and MSRV](#crate-features-and-msrv)
- [Testing](#testing)
- [Changelog](CHANGELOG.md)
- [License](#license)

## The problem

In regulated, safety-critical development (DO-178C, ISO 26262, IEC 61508), every
piece of code must be traceable to a requirement. The trace matrix is usually kept in
a spreadsheet or a requirements tool. Keeping it in sync with the code by hand is
slow, and drift is easy to miss.

Putting the requirement identifiers next to the function that implements them keeps
the link where the code is reviewed. This crate makes that link visible in the
documentation and machine-readable in the build.

## What the macro does

Annotate a function with one or more requirement identifiers:

```rust
use tpt_fl_trace_macros::traces;

/// Returns the calibrated throttle position.
#[traces("REQ-THR-001", "REQ-SAFE-002")]
pub fn throttle(position: u8) -> u8 {
    position.min(100)
}
```

The macro does two things:

1. **Documentation.** It adds a line `**Traces:** REQ-THR-001, REQ-SAFE-002` to the
   function's doc comment, so `cargo doc` shows the links.
2. **A constant.** It defines `__TPT_TRACES_<FN_NAME_UPPER>`, a `&[&str]` holding the
   identifiers exactly as written:

```rust,ignore
pub const __TPT_TRACES_THROTTLE: &[&str] = &["REQ-THR-001", "REQ-SAFE-002"];
```

The constant has the same visibility as the function.

## Quick start

```toml
[dependencies]
tpt-fl-trace-macros = "0.1"
```

Each function can carry any number of identifiers:

```rust
use tpt_fl_trace_macros::traces;

#[traces("REQ-BRAKE-001")]
pub fn braking_distance(speed: f64) -> f64 {
    0.5 * speed * speed
}
```

## Reading the trace constants

The constants can be used from tests or from tooling that checks coverage:

```rust
use tpt_fl_trace_macros::traces;

#[traces("REQ-DOC-1", "REQ-DOC-2")]
pub fn example() {}

assert_eq!(__TPT_TRACES_EXAMPLE, &["REQ-DOC-1", "REQ-DOC-2"]);
```

A coverage check can gather every `__TPT_TRACES_*` constant in the crate and compare
the identifiers with the requirements list. This crate does not perform that check
itself.

## Supported items

- Free functions, public or private, and `pub(crate)` functions.
- Methods in `impl` blocks. Each method's constant is an **associated constant**, so
  two methods with the same name on different types do not collide.
- Generic functions.
- Raw identifiers such as `r#type`. The constant name is derived from the
  identifier without the `r#`.

`#[traces]` is for functions and methods only. Applying it to a struct, a constant
or a module is a compile error.

## Compile errors

Errors are reported at the offending token:

| Mistake | Result |
|---------|--------|
| `#[traces]` with no identifiers | Error: at least one requirement id is required |
| An argument that is not a string literal | Error at the argument |
| An empty identifier (`""`) | Error at the identifier |
| The same identifier given twice | Error at the duplicate |
| Applied to something other than a function | Error |

## Guarantees

- **Verbatim identifiers.** The constant and the doc line contain the identifiers as
  written, in the order written.
- **Visibility is preserved.** A private function gets a private constant, and a
  public function gets a public one.
- **No runtime cost.** The attribute expands to a doc attribute and a constant. The
  function body is unchanged.
- **No `unsafe` code.**

## Limitations

- The macro records links. It does not check that a requirement exists, that its
  wording matches, or that the code meets it.
- Requirement identifiers are not validated beyond being non-empty and unique
  within one attribute. Format rules such as `REQ-<AREA>-<NNN>` are the caller's
  responsibility.
- Traces on a function apply to that function only. Link a requirement to every
  function that implements it.

## Crate features and MSRV

- Proc-macro crate. No optional features.
- Minimum supported Rust version: **1.75**.

## Testing

```sh
cargo test -p tpt-fl-trace-macros
```

The test suite covers:

- expansion, documentation and constant names (`tests/traces.rs`, `tests/variants.rs`),
  including raw identifiers, methods, generic functions and private items,
- compile-fail diagnostics for each error in the table above (`tests/ui/`, run with
  `trybuild`).

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.

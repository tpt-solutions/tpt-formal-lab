# Changelog

All notable changes to `tpt-fl-trace-macros` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Not yet published: `publish = false` in `Cargo.toml`. See `todo.md`, Phase 16, for the
publish plan.

---

## [Unreleased]

### Added

- Compile errors are reported at the offending token for: no requirement ids, a
  non-string argument, an empty id and a duplicate id.
- Raw identifiers are supported. `#[traces]` on `r#type` generates
  `__TPT_TRACES_TYPE`.
- Methods in `impl` blocks. Each method's constant is an associated constant, so
  methods with the same name on different types do not collide.
- Visibility-matching constants: a `pub(crate)` or private function gets a constant
  with the same visibility.
- `trybuild` compile-fail tests for each error.

---

## Initial implementation

- `#[traces("REQ-…")]` attribute macro for free functions.
- A `**Traces:**` doc line on each traced function.
- A generated `__TPT_TRACES_<FN_NAME_UPPER>: &[&str]` constant holding the ids.

---

[Unreleased]: https://github.com/tpt-solutions/tpt-formal-lab/commits/master/tpt-fl-trace-macros

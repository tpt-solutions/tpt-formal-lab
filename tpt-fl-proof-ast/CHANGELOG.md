# Changelog

All notable changes to `tpt-fl-proof-ast` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Target release: **0.1.0** (part of the first crates.io batch).

---

## [Unreleased]

### Changed (breaking, pre-release)

- `TypeKind` is removed. It could not be constructed.
- Quantifiers are parenthesised in `Display`, so nested formulas print
  unambiguously.
- `ExprNode::Forall` and `ExprNode::Exists` hold a `Binder` instead of a bare
  variable name.

### Added

- A `compile_fail` doctest that rejects a `Term` where a `Formula` is expected.
- `Binder`, with an optional `BinderSort` (`Int`, `Real`, `Bool`, or a named sort).
- `AstBuilder::forall_typed` and `AstBuilder::exists_typed`.
- Function application (`app_term`) and predicate application (`pred`).
- `Expr::walk`, `Expr::free_vars` and capture-avoiding `Expr::substitute`.

---

## Initial implementation

- `Kind` trait, sealed, with the `PropKind` and `TermKind` markers.
- `Expr<K>`: typed AST nodes parameterised over their kind.
- `Formula` and `Term` type aliases.
- `AstBuilder`: fluent API for constructing well-kinded nodes.

---

[Unreleased]: https://github.com/tpt-solutions/tpt-formal-lab/commits/master/tpt-fl-proof-ast

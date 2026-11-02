# tpt-fl-proof-ast

[![crates.io](https://img.shields.io/crates/v/tpt-fl-proof-ast.svg)](https://crates.io/crates/tpt-fl-proof-ast)
[![docs.rs](https://docs.rs/tpt-fl-proof-ast/badge.svg)](https://docs.rs/tpt-fl-proof-ast)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

A strongly-typed, proof-native AST framework for Rust.

**If it compiles, the AST is structurally valid, by construction.**

`tpt-fl-proof-ast` represents logical formulas and mathematical terms as a typed
syntax tree. Every node carries its *kind*, either proposition or term, in the
type system. Mixing them up is a compile error, not a panic or a silently wrong
proof obligation.

## Contents

- [The problem](#the-problem)
- [Features](#features)
- [Quick start](#quick-start)
- [Core concepts](#core-concepts)
- [Building expressions](#building-expressions)
- [Inspecting and transforming](#inspecting-and-transforming)
- [Guarantees](#guarantees)
- [Limitations](#limitations)
- [Crate features and MSRV](#crate-features-and-msrv)
- [Testing](#testing)
- [Changelog](CHANGELOG.md)
- [License](#license)

## The problem

Most AST types are plain enums. Nothing stops code from building a tree that makes
no sense:

```rust,ignore
// With a plain enum, this compiles and fails only at runtime, if at all:
// let bad = Formula::And(Box::new(term_node), Box::new(formula_node));
```

In verification pipelines, an ill-formed tree is a silent soundness hazard: the
solver is asked about the wrong thing. `tpt-fl-proof-ast` moves that check into
the compiler.

## Features

- **Kinds in the type system.** `Expr<K>` is parameterised by a sealed `Kind`
  marker: `PropKind` for formulas and `TermKind` for terms.
- **Aliases.** `Formula = Expr<PropKind>` and `Term = Expr<TermKind>`.
- **Fluent builder.** `AstBuilder` constructs nodes with the right kinds, so the
  call sites read like the mathematics.
- **Quantifiers with sorts.** Binders can be untyped or carry a `BinderSort`
  (`Int`, `Real`, `Bool`, or a named sort).
- **Function and predicate application** through `app_term` and `pred`.
- **Analysis and rewriting.** `walk`, `free_vars` and capture-avoiding
  `substitute`.
- **Pretty printing.** `Display` prints quantifiers in parentheses, so the output
  is unambiguous.
- **No dependencies and no `unsafe` code.**

## Quick start

```rust
use tpt_fl_proof_ast::{AstBuilder, Formula, Term};

let b = AstBuilder::new();

// Build the formula: forall x. (x > 0) -> (x * x > 0)
let x = b.var_term("x");
let zero = b.int_term(0);
let x_pos = b.gt(x.clone(), zero.clone());     // Formula
let x_sq = b.mul(x.clone(), x.clone());        // Term
let sq_pos = b.gt(x_sq, zero);                 // Formula
let body = b.implies(x_pos, sq_pos);           // Formula
let stmt: Formula = b.forall("x", body);       // Formula

println!("{stmt}");
```

Add the crate to your project:

```toml
[dependencies]
tpt-fl-proof-ast = "0.1"
```

## Core concepts

| Type | Role |
|------|------|
| `Kind` | Sealed marker trait implemented by `PropKind` and `TermKind` |
| `Expr<K>` | An expression node of kind `K`. Cheap to clone (shared `Arc` nodes) |
| `Formula` | `Expr<PropKind>`: a logical proposition |
| `Term` | `Expr<TermKind>`: a mathematical term |
| `ExprNode` | The untyped node enum behind `Expr`. Use it to match on structure |
| `Binder` | A bound variable, optionally with a `BinderSort` |
| `ArithOp`, `CmpOp` | Arithmetic and comparison operators |
| `AstBuilder` | Builder for well-kinded expressions |

Because `Kind` is sealed, downstream code cannot add new kinds. Every `Expr` is
one of the two built-in kinds.

## Building expressions

Terms are built from variables, literals, arithmetic and function application:

```rust
use tpt_fl_proof_ast::AstBuilder;

let b = AstBuilder::new();
let x = b.var_term("x");
let y = b.var_term("y");

let sum = b.add(x.clone(), b.int_term(1));    // x + 1
let prod = b.mul(sum, y);                     // (x + 1) * y
let neg = b.neg_term(x.clone());              // -x
let call = b.app_term("f", vec![x]);          // f(x)
```

Formulas are built from comparisons, connectives and quantifiers:

```rust
use tpt_fl_proof_ast::AstBuilder;

let b = AstBuilder::new();
let x = b.var_term("x");

let positive = b.gt(x.clone(), b.int_term(0));
let bounded = b.le(x.clone(), b.int_term(10));
let range = b.and(positive, bounded);         // 0 < x && x <= 10
let claim = b.exists("x", range);             // exists x. 0 < x <= 10

let p = b.var_formula("P");
let q = b.pred("Q", vec![b.var_term("y")]);    // Q(y)
let law = b.iff(p.clone(), b.not(p));          // P <-> !P (a contradiction)
```

Typed quantifiers record the sort of the bound variable:

```rust
use tpt_fl_proof_ast::{AstBuilder, BinderSort};

let b = AstBuilder::new();
let body = b.ge(b.var_term("n"), b.int_term(0));
let all_nat = b.forall_typed("n", BinderSort::Int, body);
```

### Type errors are compile errors

```compile_fail
use tpt_fl_proof_ast::AstBuilder;

let b = AstBuilder::new();
let p = b.var_formula("P"); // Formula
let x = b.var_term("x");    // Term
let bad = b.add(p, x);      // error: `add` expects two `Term`s
```

```compile_fail
use tpt_fl_proof_ast::AstBuilder;

let b = AstBuilder::new();
let x = b.var_term("x");
let y = b.var_term("y");
let bad = b.and(x, y);      // error: `and` expects two `Formula`s
```

## Inspecting and transforming

```rust
use tpt_fl_proof_ast::AstBuilder;

let b = AstBuilder::new();
let body = b.gt(b.var_term("x"), b.var_term("c"));
let stmt = b.forall("x", body);

// The free variables are the names not bound by a quantifier
let free = stmt.free_vars();
assert!(free.contains("c"));
assert!(!free.contains("x"));

// Count the nodes that match a pattern
let mut vars = 0;
stmt.walk(|node| {
    if let tpt_fl_proof_ast::ExprNode::Var(_) = node {
        vars += 1;
    }
});
```

`substitute(name, replacement)` replaces the free occurrences of `name`. It does
not replace occurrences under a binder of the same name, and it avoids capturing
free variables of the replacement.

## Guarantees

- **Kind safety.** A `Term` cannot be used where a `Formula` is required, and the
  reverse. Both directions are checked by `compile_fail` doctests.
- **Sealed kinds.** Only the built-in kinds exist.
- **No runtime validation is needed for kind errors.** Construction cannot fail.
- **No `unsafe` code.**

## Limitations

- The crate represents syntax. It does not type-check sorts beyond the kind
  distinction, and it does not prove anything. Use
  [`tpt-fl-smt-bridge`](https://github.com/tpt-solutions/tpt-formal-lab/tree/master/tpt-fl-smt-bridge) to hand formulas to a solver.
- There is no direct conversion from `Formula` to `tpt-fl-smt-bridge::Expr` yet.
  This would add a dependency between the two crates and is still undecided.
- Literals are `i64` integers and booleans. Real and string literals are not
  included.

## Crate features and MSRV

- No optional features and no dependencies.
- Minimum supported Rust version: **1.75**.

## Testing

```sh
cargo test -p tpt-fl-proof-ast
```

Tests cover the builder, expression construction and kinds. The `compile_fail`
doctests check that kind mismatches are rejected.

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.

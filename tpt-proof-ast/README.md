# tpt-proof-ast

[![crates.io](https://img.shields.io/crates/v/tpt-proof-ast.svg)](https://crates.io/crates/tpt-proof-ast)
[![docs.rs](https://docs.rs/tpt-proof-ast/badge.svg)](https://docs.rs/tpt-proof-ast)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../LICENSE-MIT)

A strongly-typed, proof-native AST framework for Rust.

**If it compiles, the AST is structurally valid — by construction.**

## The problem

Standard ASTs are enums. Nothing stops you from accidentally building
an invalid tree at runtime:

```rust
// With a standard AST, you discover mistakes at runtime:
// let bad = Formula::And(Box::new(term_node), Box::new(formula_node)); // oops
```

## Solution

`tpt-proof-ast` encodes **kind constraints** at the type level using
`PhantomData` markers. Attaching a `Term` where a `Formula` is expected
is a **compile error**, not a runtime panic.

```rust
use tpt_proof_ast::{AstBuilder, Formula, Term};

let b = AstBuilder::new();

// Build ∀x. (x > 0) → (x * x > 0)
let x    = b.var_term("x");
let zero = b.int_term(0);
let pos  = b.gt(x.clone(), zero.clone());     // Formula: x > 0
let sq   = b.mul(x.clone(), x);               // Term: x * x
let body = b.implies(pos, b.gt(sq, zero));    // Formula
let stmt = b.forall("x", body);               // Formula

println!("{stmt}"); // ∀x. ((x > 0) → ((x * x) > 0))
```

## Kind system

| Kind | Rust type | Meaning |
|------|-----------|---------|
| `PropKind` | `Formula = Expr<PropKind>` | Logical propositions (true/false) |
| `TermKind` | `Term = Expr<TermKind>` | Mathematical terms (have a value) |
| `TypeKind` | `Expr<TypeKind>` | Type-level expressions |

## Usage

```toml
[dependencies]
tpt-proof-ast = "0.1"
```

## License

MIT OR Apache-2.0

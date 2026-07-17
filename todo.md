# tpt-formal-lab — Build Checklist

> Optimised for crates.io release at v0.1.0 (all 5 original crates simultaneously).
> Publish order: tpt-exact-math → tpt-proof-ast → tpt-verify-macros → tpt-deterministic-sim → tpt-smt-bridge
>
> 5 additional crates are in development (see Phases 10–14 below) and are **not**
> part of the v0.1.0 publish batch — publishing them is a separate future decision.
> Planned dependency order: tpt-exact-math → tpt-verified-ode; tpt-verify-macros → tpt-verified-algorithms;
> tpt-redundancy, tpt-trace-macros, tpt-det-proptest are standalone.

---

## Phase 1 — Workspace Scaffolding

- [x] `spec.txt` — design spec (done)
- [x] `todo.md` — this file
- [x] `Cargo.toml` — workspace manifest
- [x] `README.md` — top-level overview of the ecosystem
- [x] `CHANGELOG.md` — initial v0.1.0 entry
- [x] `LICENSE-MIT`
- [x] `LICENSE-APACHE`
- [x] `.github/workflows/ci.yml` — CI pipeline
- [x] `.rustfmt.toml` — shared formatting config
- [x] `.clippy.toml` — shared lint config

---

## Phase 2 — `tpt-exact-math`

### Setup
- [x] `tpt-exact-math/Cargo.toml` — package manifest with crates.io metadata
- [x] `tpt-exact-math/README.md` — crate-level docs (becomes docs.rs landing page)

### Implementation
- [x] `tpt-exact-math/src/lib.rs` — `#![no_std]`, `#![deny(missing_docs)]`, re-exports
- [x] `tpt-exact-math/src/rational.rs` — `Rational` type (BigInt numerator/denominator, always reduced)
  - [x] `From<i64>`, `From<i32>`, `From<u64>`, `From<BigInt>`
  - [x] `Add`, `Sub`, `Mul`, `Div`, `Neg`
  - [x] `PartialOrd`, `Ord`, `Eq`
  - [x] `Display`, `Debug`
  - [x] `zero()`, `one()`, `is_zero()`, `is_negative()`, `abs()`, `recip()`
- [x] `tpt-exact-math/src/interval.rs` — `Interval` (lower/upper bounds)
  - [x] `Interval::new(lo, hi)`, `Interval::point(x)`
  - [x] `Add`, `Sub`, `Mul`, `Div`, `Neg` (monotone interval propagation)
  - [x] `contains(&self, x)`, `contains_interval`, `width()`, `midpoint()`, `hull()`
  - [x] `Display`, `Debug`

### Tests
- [x] Unit tests for `Rational` (11 tests — arithmetic, reduction, edge cases)
- [x] Unit tests for `Interval` (10 tests — arithmetic, containment)
- [x] Test: `a * a.recip() == 1` for non-zero rationals
- [x] Test: `0.1 + 0.2` exact via `Rational` equals `3/10`

### no_std verification
- [x] `cargo build -p tpt-exact-math --target thumbv7em-none-eabihf` passes (run this manually — requires cross-compilation target)

---

## Phase 3 — `tpt-proof-ast`

### Setup
- [x] `tpt-proof-ast/Cargo.toml`
- [x] `tpt-proof-ast/README.md`

### Implementation
- [x] `tpt-proof-ast/src/lib.rs` — `#![deny(missing_docs)]`, re-exports, `Formula`/`Term` type aliases
- [x] `tpt-proof-ast/src/kinds.rs` — `PropKind`, `TermKind`, `TypeKind` sealed marker types
- [x] `tpt-proof-ast/src/expr.rs` — `Expr<K>` with `PhantomData<K>`, full `ExprNode` enum
- [x] `tpt-proof-ast/src/builder.rs` — `AstBuilder` with term, formula, comparison, quantifier builders

### Tests
- [x] 5 unit tests (term display, formula display, forall, not/or, nested formula)
- [x] 12 doc-tests across all public API items
- [x] Compile-fail test: `Formula` where `Term` expected → add a `compile_fail` test

---

## Phase 4 — `tpt-verify-macros`

### Setup
- [x] `tpt-verify-macros/Cargo.toml` — `proc-macro = true`, syn/quote deps
- [x] `tpt-verify-macros/README.md`

### Implementation
- [x] `tpt-verify-macros/src/lib.rs` — all four macros in a single file
  - [x] `#[requires(expr)]` — `debug_assert!` + doc injection
  - [x] `#[ensures(expr)]` — wraps return, binds `result`, `debug_assert!`
  - [x] `#[invariant(expr)]` — `debug_assert!` at entry + doc injection
  - [x] `#[pure]` — doc marker only

### Tests
- [x] 5 doc-tests covering all four macros
- [x] Integration tests: panic-on-violation in debug (add `tests/contracts.rs`)
- [x] Integration tests: zero-cost in release (`cargo test --release`)

---

## Phase 5 — `tpt-deterministic-sim`

### Setup
- [x] `tpt-deterministic-sim/Cargo.toml` — with optional `"exact-math"` feature
- [x] `tpt-deterministic-sim/README.md`

### Implementation
- [x] `tpt-deterministic-sim/src/lib.rs` — `#![deny(missing_docs)]`, feature flags, re-exports
- [x] `tpt-deterministic-sim/src/fixed_point.rs` — `FixedPoint<const DENOM: i64>`
  - [x] `from_raw`, `from_int`, `from_f64_truncating`, `to_f64`
  - [x] `Add`, `Sub`, `Mul`, `Div`, `Neg` (overflow-checked)
  - [x] `Display` (decimal format), `Debug`
  - [x] `PartialOrd`, `Ord`, `Eq`, `Copy`
- [x] `tpt-deterministic-sim/src/sim.rs` — `DeterministicSim<S>`
  - [x] `BTreeMap<EntityId, S>` entity registry
  - [x] `spawn`, `despawn`, `get`, `get_mut`, `iter`
  - [x] `add_system`, `remove_system` (name-sorted BTreeMap)
  - [x] `step`, `step_n`, `step_count`

### Tests
- [x] 11 `FixedPoint` tests (arithmetic, display, ordering, determinism)
- [x] 7 `DeterministicSim` tests (spawn/despawn, system order, determinism, iteration order)
- [x] 9 doc-tests

---

## Phase 6 — `tpt-smt-bridge`

### Setup
- [x] `tpt-smt-bridge/Cargo.toml`
- [x] `tpt-smt-bridge/README.md`

### Implementation
- [x] `tpt-smt-bridge/src/lib.rs` — `#![deny(missing_docs)]`, re-exports
- [x] `tpt-smt-bridge/src/sort.rs` — `Sort` enum with `to_smtlib2()`
- [x] `tpt-smt-bridge/src/expr.rs` — `Expr` with full propositional/arithmetic/quantifier support
- [x] `tpt-smt-bridge/src/smtlib2.rs` — `emit_smtlib2(expr: &Expr) -> String`
- [x] `tpt-smt-bridge/src/solver.rs` — `SmtSolver`: `declare_const`, `declare_fun`, `assert`, `set_logic`, `emit_smtlib2`, `emit_check`
- [x] `tpt-smt-bridge/src/counter_example.rs` — `CounterExample` parser for `(model ...)` output

### Tests
- [x] 22 unit tests (smtlib2 emission, solver builder, counterexample parsing, sort display)
- [x] 22 doc-tests

---

## Phase 7 — crates.io Polish

- [x] All crates: `#![deny(missing_docs)]` — zero missing doc warnings
- [x] All crates: `///` doc comments on every `pub` item with at least one `# Example`
- [x] All crates: `#![doc(html_root_url = "https://docs.rs/<name>/<version>/")]`
- [x] All crates: `rust-version = "1.75"` in `Cargo.toml`
- [x] All crates: `keywords` (max 5) and `categories` set correctly
- [x] All crates: `exclude` field to skip CI, spec, snapshot files
- [x] All `README.md` files: badges (crates.io version, docs.rs, CI status, license)
- [x] Top-level `README.md`: ecosystem overview with links to each crate
- [x] Set `repository` field to actual GitHub URL once repo is created (`github.com/tpt-solutions/tpt-formal-lab`, matching the pushed remote)

---

## Phase 8 — CI & Quality Gates

- [x] `.github/workflows/ci.yml` — matrix: stable + beta + nightly
- [x] CI job: `cargo test --workspace`
- [x] CI job: `cargo clippy --workspace -- -D warnings`
- [x] CI job: `cargo fmt --all -- --check`
- [x] CI job: `cargo doc --workspace --no-deps`
- [x] CI job: no_std check (`--target thumbv7em-none-eabihf` for tpt-exact-math)
- [x] CI job: `cargo publish --dry-run` for each crate (in dep order)
- [x] `.rustfmt.toml` — `edition = "2021"`, `max_width = 100`
- [x] Push to GitHub to activate CI (`origin/master` = local `master` @ `6f8225f`)

---

## Phase 9 — Final Publish Checklist

- [x] `cargo test --workspace` — all green
- [x] `cargo clippy --workspace -- -D warnings` — zero warnings
- [x] `cargo doc --workspace --no-deps` — zero missing docs
- [x] `cargo publish --dry-run -p tpt-exact-math`
- [x] `cargo publish --dry-run -p tpt-proof-ast`
- [x] `cargo publish --dry-run -p tpt-verify-macros`
- [ ] `cargo publish --dry-run -p tpt-deterministic-sim` — packaging succeeds; the registry-lookup step for its `tpt-exact-math` dependency can only be verified for real once `tpt-exact-math` is actually published (dry-run can't see unpublished sibling crates)
- [x] `cargo publish --dry-run -p tpt-smt-bridge`
- [ ] Tag git commit `v0.1.0`
- [ ] `cargo publish -p tpt-exact-math` → wait for index
- [ ] `cargo publish -p tpt-proof-ast` → wait for index
- [ ] `cargo publish -p tpt-verify-macros` → wait for index
- [ ] `cargo publish -p tpt-deterministic-sim` → wait for index
- [ ] `cargo publish -p tpt-smt-bridge` → wait for index
- [ ] Verify all 5 crates appear on crates.io and docs.rs

---

## Phase 10 — `tpt-redundancy`

### Setup
- [x] `tpt-redundancy/Cargo.toml` — package manifest with crates.io metadata
- [x] `tpt-redundancy/README.md` — crate-level docs (becomes docs.rs landing page)

### Implementation
- [x] `tpt-redundancy/src/lib.rs` — `#![no_std]`, `#![deny(missing_docs)]`, `#![deny(unsafe_code)]`, re-exports
- [x] `tpt-redundancy/src/vote.rs` — `Replicated<T, const N: usize>`, `VoteResult<T>`
  - [x] `majority_vote()` — `Unanimous`/`Majority`/`NoMajority`
  - [x] `median_vote()` — odd `N`, `Ord + Clone`
  - [x] `bitwise_vote()` — bit-by-bit majority for unsigned integer types

### Tests
- [x] Unanimous / majority / no-majority cases
- [x] Even-`N` tie handling
- [x] Median vote against known arrays
- [x] Bitwise voter against hand-computed bit patterns

---

## Phase 11 — `tpt-verified-ode`

### Setup
- [x] `tpt-verified-ode/Cargo.toml` — depends on `tpt-exact-math`
- [x] `tpt-verified-ode/README.md` — states the rigor caveat (small-`h` Picard contraction requirement)

### Implementation
- [x] `tpt-verified-ode/src/lib.rs` — `#![no_std]` + `alloc`, `#![deny(missing_docs)]`, re-exports
- [x] `tpt-verified-ode/src/solver.rs` — `IntervalFn` trait, `OdeSolver<F>`
  - [x] A priori Picard-iteration enclosure step
  - [x] Tightened step using the a priori box
  - [x] `step()`, `solve(steps)`

### Tests
- [x] `y' = y` enclosure contains rational approximations of `eʰ`, `e²ʰ`, …
- [x] `y' = -y`
- [x] `y' = c` (constant, exact enclosure)
- [x] Enclosure-width sanity checks across step counts

---

## Phase 12 — `tpt-trace-macros`

### Setup
- [x] `tpt-trace-macros/Cargo.toml` — `proc-macro = true`, syn/quote/proc-macro2 deps
- [x] `tpt-trace-macros/README.md`

### Implementation
- [x] `tpt-trace-macros/src/lib.rs` — `#[traces("REQ-…")]` attribute macro
  - [x] `**Traces:** …` doc injection
  - [x] Generated `const __TPT_TRACES_<FN_NAME_UPPER>: &[&str]`

### Tests
- [x] Doc-tests covering macro usage
- [x] Integration test asserting the generated const matches macro arguments

---

## Phase 13 — `tpt-verified-algorithms`

### Setup
- [x] `tpt-verified-algorithms/Cargo.toml` — depends on `tpt-verify-macros`
- [x] `tpt-verified-algorithms/README.md`

### Implementation
- [x] `tpt-verified-algorithms/src/lib.rs` — `#![no_std]` + `alloc`, `#![deny(missing_docs)]`, re-exports
- [x] `tpt-verified-algorithms/src/predicates.rs` — `is_sorted`, `is_permutation`
- [x] `tpt-verified-algorithms/src/sort.rs` — `verified_sort` with debug-mode postcondition checks
- [x] `tpt-verified-algorithms/src/search.rs` — `verified_binary_search` with `#[requires]`/`#[ensures]`

### Tests
- [x] Sortedness + permutation-preservation (empty, single, duplicates, reverse-sorted)
- [x] Binary search hit/miss
- [x] Precondition-violation panics in debug builds

---

## Phase 14 — `tpt-det-proptest`

### Setup
- [x] `tpt-det-proptest/Cargo.toml` — zero external deps
- [x] `tpt-det-proptest/README.md`

### Implementation
- [x] `tpt-det-proptest/src/lib.rs` — `#![no_std]` + `alloc`, `#![deny(missing_docs)]`, re-exports
- [x] `tpt-det-proptest/src/rng.rs` — `Xorshift64` deterministic PRNG
- [x] `tpt-det-proptest/src/strategy.rs` — `Strategy` trait, `IntRange<T>`, `AnyBool`
- [x] `tpt-det-proptest/src/check.rs` — `check()` runner with bisection shrinking

### Tests
- [x] Same seed ⇒ identical generated sequence
- [x] Known failing property shrinks to its minimal counterexample
- [x] Passing property runs all iterations without failure

---

## Phase 15 — New-crate workspace wiring

- [x] Add all 5 new crates to root `Cargo.toml` `[workspace] members` and `[workspace.dependencies]`
- [x] Update root `README.md` crates table and quick-look examples
- [x] Update `CHANGELOG.md` "Unreleased" section with the 5 additions
- [x] `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --all -- --check`, `cargo doc --workspace --no-deps` all green

---

## Phase 16 — crates.io Polish parity for the 5 new crates

- [x] `tpt-redundancy`, `tpt-verified-ode`, `tpt-trace-macros`, `tpt-verified-algorithms`, `tpt-det-proptest`: `rust-version.workspace = true`, `keywords`, `categories`, `exclude` set in `Cargo.toml`
- [x] `#![doc(html_root_url = ...)]` present on lib crates (`tpt-trace-macros` is proc-macro-only and has no lib doc root, which is expected)
- [x] Each crate's `README.md` has CI/license badges
- [ ] Commit the working-tree changes for the 5 new crates + supporting edits to `Cargo.toml`/`README.md`/`CHANGELOG.md` (currently untracked/modified, not yet committed)
- [ ] Decide and record a publish plan for the 5 new crates (currently intentionally excluded from the v0.1.0 batch — see note at top of this file)

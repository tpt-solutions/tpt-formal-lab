# tpt-formal-lab — Build Checklist

> Optimised for crates.io release at v0.1.0 (all 5 original crates simultaneously).
> Publish order: tpt-fl-exact-math → tpt-fl-proof-ast → tpt-fl-verify-macros → tpt-fl-deterministic-sim → tpt-fl-smt-bridge
>
> 5 additional crates are in development (see Phases 10–14 below) and are **not**
> part of the v0.1.0 publish batch — publishing them is a separate future decision.
> Planned dependency order: tpt-fl-exact-math → tpt-fl-verified-ode; tpt-fl-verify-macros → tpt-fl-verified-algorithms;
> tpt-fl-redundancy, tpt-fl-trace-macros, tpt-fl-det-proptest are standalone.

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

## Phase 2 — `tpt-fl-exact-math`

### Setup
- [x] `tpt-fl-exact-math/Cargo.toml` — package manifest with crates.io metadata
- [x] `tpt-fl-exact-math/README.md` — crate-level docs (becomes docs.rs landing page)

### Implementation
- [x] `tpt-fl-exact-math/src/lib.rs` — `#![no_std]`, `#![deny(missing_docs)]`, re-exports
- [x] `tpt-fl-exact-math/src/rational.rs` — `Rational` type (BigInt numerator/denominator, always reduced)
  - [x] `From<i64>`, `From<i32>`, `From<u64>`, `From<BigInt>`
  - [x] `Add`, `Sub`, `Mul`, `Div`, `Neg`
  - [x] `PartialOrd`, `Ord`, `Eq`
  - [x] `Display`, `Debug`
  - [x] `zero()`, `one()`, `is_zero()`, `is_negative()`, `abs()`, `recip()`
- [x] `tpt-fl-exact-math/src/interval.rs` — `Interval` (lower/upper bounds)
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
- [x] `cargo build -p tpt-fl-exact-math --target thumbv7em-none-eabihf` passes (run this manually — requires cross-compilation target)

---

## Phase 3 — `tpt-fl-proof-ast`

### Setup
- [x] `tpt-fl-proof-ast/Cargo.toml`
- [x] `tpt-fl-proof-ast/README.md`

### Implementation
- [x] `tpt-fl-proof-ast/src/lib.rs` — `#![deny(missing_docs)]`, re-exports, `Formula`/`Term` type aliases
- [x] `tpt-fl-proof-ast/src/kinds.rs` — `PropKind`, `TermKind`, `TypeKind` sealed marker types
- [x] `tpt-fl-proof-ast/src/expr.rs` — `Expr<K>` with `PhantomData<K>`, full `ExprNode` enum
- [x] `tpt-fl-proof-ast/src/builder.rs` — `AstBuilder` with term, formula, comparison, quantifier builders

### Tests
- [x] 5 unit tests (term display, formula display, forall, not/or, nested formula)
- [x] 12 doc-tests across all public API items
- [x] Compile-fail test: `Formula` where `Term` expected → add a `compile_fail` test

---

## Phase 4 — `tpt-fl-verify-macros`

### Setup
- [x] `tpt-fl-verify-macros/Cargo.toml` — `proc-macro = true`, syn/quote deps
- [x] `tpt-fl-verify-macros/README.md`

### Implementation
- [x] `tpt-fl-verify-macros/src/lib.rs` — all four macros in a single file
  - [x] `#[requires(expr)]` — `debug_assert!` + doc injection
  - [x] `#[ensures(expr)]` — wraps return, binds `result`, `debug_assert!`
  - [x] `#[invariant(expr)]` — `debug_assert!` at entry + doc injection
  - [x] `#[pure]` — doc marker only

### Tests
- [x] 5 doc-tests covering all four macros
- [x] Integration tests: panic-on-violation in debug (add `tests/contracts.rs`)
- [x] Integration tests: zero-cost in release (`cargo test --release`)

---

## Phase 5 — `tpt-fl-deterministic-sim`

### Setup
- [x] `tpt-fl-deterministic-sim/Cargo.toml` — with optional `"exact-math"` feature
- [x] `tpt-fl-deterministic-sim/README.md`

### Implementation
- [x] `tpt-fl-deterministic-sim/src/lib.rs` — `#![deny(missing_docs)]`, feature flags, re-exports
- [x] `tpt-fl-deterministic-sim/src/fixed_point.rs` — `FixedPoint<const DENOM: i64>`
  - [x] `from_raw`, `from_int`, `from_f64_truncating`, `to_f64`
  - [x] `Add`, `Sub`, `Mul`, `Div`, `Neg` (overflow-checked)
  - [x] `Display` (decimal format), `Debug`
  - [x] `PartialOrd`, `Ord`, `Eq`, `Copy`
- [x] `tpt-fl-deterministic-sim/src/sim.rs` — `DeterministicSim<S>`
  - [x] `BTreeMap<EntityId, S>` entity registry
  - [x] `spawn`, `despawn`, `get`, `get_mut`, `iter`
  - [x] `add_system`, `remove_system` (name-sorted BTreeMap)
  - [x] `step`, `step_n`, `step_count`

### Tests
- [x] 11 `FixedPoint` tests (arithmetic, display, ordering, determinism)
- [x] 7 `DeterministicSim` tests (spawn/despawn, system order, determinism, iteration order)
- [x] 9 doc-tests

---

## Phase 6 — `tpt-fl-smt-bridge`

### Setup
- [x] `tpt-fl-smt-bridge/Cargo.toml`
- [x] `tpt-fl-smt-bridge/README.md`

### Implementation
- [x] `tpt-fl-smt-bridge/src/lib.rs` — `#![deny(missing_docs)]`, re-exports
- [x] `tpt-fl-smt-bridge/src/sort.rs` — `Sort` enum with `to_smtlib2()`
- [x] `tpt-fl-smt-bridge/src/expr.rs` — `Expr` with full propositional/arithmetic/quantifier support
- [x] `tpt-fl-smt-bridge/src/smtlib2.rs` — `emit_smtlib2(expr: &Expr) -> String`
- [x] `tpt-fl-smt-bridge/src/solver.rs` — `SmtSolver`: `declare_const`, `declare_fun`, `assert`, `set_logic`, `emit_smtlib2`, `emit_check`
- [x] `tpt-fl-smt-bridge/src/counter_example.rs` — `CounterExample` parser for `(model ...)` output

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
- [x] CI job: no_std check (`--target thumbv7em-none-eabihf` for tpt-fl-exact-math)
- [x] CI job: `cargo publish --dry-run` for each crate (in dep order)
- [x] `.rustfmt.toml` — `edition = "2021"`, `max_width = 100`
- [x] Push to GitHub to activate CI (`origin/master` = local `master` @ `6f8225f`)

---

## Phase 9 — Final Publish Checklist

- [x] `cargo test --workspace` — all green
- [x] `cargo clippy --workspace -- -D warnings` — zero warnings
- [x] `cargo doc --workspace --no-deps` — zero missing docs
- [x] `cargo publish --dry-run -p tpt-fl-exact-math`
- [x] `cargo publish --dry-run -p tpt-fl-proof-ast`
- [x] `cargo publish --dry-run -p tpt-fl-verify-macros`
- [ ] `cargo publish --dry-run -p tpt-fl-deterministic-sim` — packaging succeeds; the registry-lookup step for its `tpt-fl-exact-math` dependency can only be verified for real once `tpt-fl-exact-math` is actually published (dry-run can't see unpublished sibling crates)
- [x] `cargo publish --dry-run -p tpt-fl-smt-bridge`
- [ ] Tag git commit `v0.1.0`
- [ ] `cargo publish -p tpt-fl-exact-math` → wait for index
- [ ] `cargo publish -p tpt-fl-proof-ast` → wait for index
- [ ] `cargo publish -p tpt-fl-verify-macros` → wait for index
- [ ] `cargo publish -p tpt-fl-deterministic-sim` → wait for index
- [ ] `cargo publish -p tpt-fl-smt-bridge` → wait for index
- [ ] Verify all 5 crates appear on crates.io and docs.rs

---

## Phase 10 — `tpt-fl-redundancy`

### Setup
- [x] `tpt-fl-redundancy/Cargo.toml` — package manifest with crates.io metadata
- [x] `tpt-fl-redundancy/README.md` — crate-level docs (becomes docs.rs landing page)

### Implementation
- [x] `tpt-fl-redundancy/src/lib.rs` — `#![no_std]`, `#![deny(missing_docs)]`, `#![deny(unsafe_code)]`, re-exports
- [x] `tpt-fl-redundancy/src/vote.rs` — `Replicated<T, const N: usize>`, `VoteResult<T>`
  - [x] `majority_vote()` — `Unanimous`/`Majority`/`NoMajority`
  - [x] `median_vote()` — odd `N`, `Ord + Clone`
  - [x] `bitwise_vote()` — bit-by-bit majority for unsigned integer types

### Tests
- [x] Unanimous / majority / no-majority cases
- [x] Even-`N` tie handling
- [x] Median vote against known arrays
- [x] Bitwise voter against hand-computed bit patterns

---

## Phase 11 — `tpt-fl-verified-ode`

### Setup
- [x] `tpt-fl-verified-ode/Cargo.toml` — depends on `tpt-fl-exact-math`
- [x] `tpt-fl-verified-ode/README.md` — states the rigor caveat (small-`h` Picard contraction requirement)

### Implementation
- [x] `tpt-fl-verified-ode/src/lib.rs` — `#![no_std]` + `alloc`, `#![deny(missing_docs)]`, re-exports
- [x] `tpt-fl-verified-ode/src/solver.rs` — `IntervalFn` trait, `OdeSolver<F>`
  - [x] A priori Picard-iteration enclosure step
  - [x] Tightened step using the a priori box
  - [x] `step()`, `solve(steps)`

### Tests
- [x] `y' = y` enclosure contains rational approximations of `eʰ`, `e²ʰ`, …
- [x] `y' = -y`
- [x] `y' = c` (constant, exact enclosure)
- [x] Enclosure-width sanity checks across step counts

---

## Phase 12 — `tpt-fl-trace-macros`

### Setup
- [x] `tpt-fl-trace-macros/Cargo.toml` — `proc-macro = true`, syn/quote/proc-macro2 deps
- [x] `tpt-fl-trace-macros/README.md`

### Implementation
- [x] `tpt-fl-trace-macros/src/lib.rs` — `#[traces("REQ-…")]` attribute macro
  - [x] `**Traces:** …` doc injection
  - [x] Generated `const __TPT_TRACES_<FN_NAME_UPPER>: &[&str]`

### Tests
- [x] Doc-tests covering macro usage
- [x] Integration test asserting the generated const matches macro arguments

---

## Phase 13 — `tpt-fl-verified-algorithms`

### Setup
- [x] `tpt-fl-verified-algorithms/Cargo.toml` — depends on the external `contracts` crate
- [x] `tpt-fl-verified-algorithms/README.md`

### Implementation
- [x] `tpt-fl-verified-algorithms/src/lib.rs` — `#![no_std]` + `alloc`, `#![deny(missing_docs)]`, re-exports
- [x] `tpt-fl-verified-algorithms/src/predicates.rs` — `is_sorted`, `is_permutation`
- [x] `tpt-fl-verified-algorithms/src/sort.rs` — `verified_sort` with debug-mode postcondition checks
- [x] `tpt-fl-verified-algorithms/src/search.rs` — `verified_binary_search` with `#[requires]`/`#[ensures]`

### Tests
- [x] Sortedness + permutation-preservation (empty, single, duplicates, reverse-sorted)
- [x] Binary search hit/miss
- [x] Precondition-violation panics in debug builds

---

## Phase 14 — `tpt-fl-det-proptest`

### Setup
- [x] `tpt-fl-det-proptest/Cargo.toml` — zero external deps
- [x] `tpt-fl-det-proptest/README.md`

### Implementation
- [x] `tpt-fl-det-proptest/src/lib.rs` — `#![no_std]` + `alloc`, `#![deny(missing_docs)]`, re-exports
- [x] `tpt-fl-det-proptest/src/rng.rs` — `Xorshift64` deterministic PRNG
- [x] `tpt-fl-det-proptest/src/strategy.rs` — `Strategy` trait, `IntRange<T>`, `AnyBool`
- [x] `tpt-fl-det-proptest/src/check.rs` — `check()` runner with bisection shrinking

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

- [x] `tpt-fl-redundancy`, `tpt-fl-verified-ode`, `tpt-fl-trace-macros`, `tpt-fl-verified-algorithms`, `tpt-fl-det-proptest`: `rust-version.workspace = true`, `keywords`, `categories`, `exclude` set in `Cargo.toml`
- [x] `#![doc(html_root_url = ...)]` present on lib crates (`tpt-fl-trace-macros` is proc-macro-only and has no lib doc root, which is expected)
- [x] Each crate's `README.md` has CI/license badges
- [ ] Commit the working-tree changes for the 5 new crates + supporting edits to `Cargo.toml`/`README.md`/`CHANGELOG.md` (currently untracked/modified, not yet committed)
- [x] Decide and record a publish plan for the 5 new crates (publish = false; see README and CHANGELOG) (currently intentionally excluded from the v0.1.0 batch — see note at top of this file)

---

## Phase 17 — Review Fixes: Correctness & Soundness

> From the platform review. Write a failing regression test first, then fix.

### Done
- [x] CI: workflow triggered on `main` but branch is `master` → now `master`
- [x] `tpt-fl-exact-math`: `Interval` division panicked for any non-point divisor (reciprocal bounds not swapped); tests added

### `tpt-fl-exact-math`
- [x] `Interval::checked_div` (returns `Option`/union for zero-straddling divisors)
- [x] `Rational::to_f64` returns NaN/inf for huge numerators/denominators — rescale before dividing
- [x] Remove private `Rational::min`/`max` that shadow `Ord::min`/`max`
- [x] Tests: big values, `recip` of negative, `Hash`/`Ord` consistency, neg×neg multiply

### `tpt-fl-verified-ode`
- [x] `IntervalFn::eval` takes/returns `Interval` (natural interval extension) — corner-only evaluation is unsound for non-monotone `f`
- [x] Don't return an unverified box after `MAX_WIDEN`; return `Result<_, NoEnclosure>` and auto-halve `h`
- [x] Validate `h > 0`
- [x] Tightening step (`y0 + h·f(T, Y)`) so the state is an end-point enclosure, not the whole-interval hull
- [x] Dyadic outward rounding to stop rational size blow-up
- [x] Tests: non-monotone `f`, stiff `f`, non-autonomous `f`, failing fixpoint, width bounds; replace assertion-free `debug_single`

### `tpt-fl-deterministic-sim`
- [x] `FixedPoint::Mul`/`Div` use `i128` intermediates
- [x] `checked_*` / `saturating_*` ops; check `from_int`, `Neg`, `abs` (`i64::MIN`)
- [x] `Display` without `f64::log10` (integer digit count; correct for non-power-of-10 `DENOM`)
- [x] Compile-time `DENOM > 0` assertion; document truncation rounding
- [x] Implement the `exact-math` feature or remove it and its doc claims
- [x] Tests: overflow panics, negative truncation, `Display` for other `DENOM`s, `remove_system`, `despawn`, `get_mut`

### `tpt-fl-redundancy`
- [x] `median_vote` with `N == 0` panics — return `Option` or reject at compile time
- [x] Fix docs: majority tolerates `floor((N-1)/2)` faults, not `floor(N/2)`
- [x] `bitwise_vote` even-N tie silently resolves to 0 — document or report
- [x] Seal `BitNarrow` (docs say sealed, trait is public)

### `tpt-fl-det-proptest`
- [x] Shrinker linear-scans the whole range (hang on wide ranges) — derive index from the failing value, use halving shrinker, verify final candidate still fails
- [x] Remove modulo bias (Lemire / rejection sampling); `AnyBool` uses top bit
- [x] Mix seeds through splitmix64 (seeds `0` and golden-ratio collide)
- [x] `at_index` lower-bound check; inclusive ranges (`MAX` currently ungeneratable)
- [x] `usize`/`isize`/`i128`/`u128` support (docs claim all primitive ints)
- [x] Catch panicking properties (`catch_unwind` under `std`) or document the limitation
- [x] Tests: non-monotone failure set, zero seed, full-width ranges, bias check

### `tpt-fl-proof-ast`
- [x] Parenthesise quantifiers in `Display` (scope is ambiguous inside `∧`/`∨`)
- [x] Remove or implement `Expr<TypeKind>` (can't be constructed today)
- [x] Correct "const generics" claim in docs
- [x] Tests: `exists`, `iff`, comparisons, `sub`/`div`/`neg`, `tru`/`fals`, `Debug`, reverse compile-fail (Term passed where Formula expected)

### `tpt-fl-smt-bridge`
- [x] Real s-expression tokenizer for `CounterExample` (multi-line Z3 `define-fun`, quoted symbols, `define-fun` with args, compound sorts, `#x`/`#b`/`(_ bvN w)`, `(- 1)`, `(/ 1 2)`)
- [x] `SatResult { Sat(model), Unsat, Unknown(reason) }`; surface `(error ...)`
- [x] Negative reals emitted as `(- 3.0)` / `(/ (- 1.0) 2.0)`; reject zero denominator
- [x] Separate `idiv` / `rdiv` (`Div` always emits `/`)
- [x] `quote_symbol()` for names; validate `set_logic` argument (command injection)
- [x] Normalise empty / single-arg `and`/`or`/`add`/`mul`/`distinct`
- [x] Mask bitvector literals to width; reject width 0
- [x] Only emit `(get-model)` when appropriate; add `(set-option :produce-models true)`
- [x] Reject empty quantifier binder lists
- [x] `i128`/bignum integer values in `CounterExample`
- [x] Tests: negative/real emission, all operators, quoting, quantifiers, real Z3/cvc5 transcripts; tighten the weak `full_problem` assertion

### `tpt-fl-verify-macros`
- [x] `#[ensures]`: drop `move` on the closure, give it the declared return type (non-Copy args, `&mut self`, `?`)
- [x] `#[ensures]` support for `async fn` / `const fn` / `unsafe fn`
- [x] `old(expr)` support; renameable / hygienic `result` binding
- [x] `#[invariant]` also checked at exit; loop-invariant function-like macro
- [x] Contracts on trait methods / impl items — works on methods in `impl` blocks; trait default methods with bodies are NOT supported yet
- [x] Nicer errors (empty args, non-function items)
- [x] Tests: generics, `&mut self`, early return, `?`, async, stacked attributes, param named `result`, trybuild compile-fail

### `tpt-fl-trace-macros`
- [x] Replace `panic!` with spanned `syn::Error`; reject empty strings, duplicate IDs, zero args
- [x] Raw identifiers (`r#type`) panic when building the const name — use `unraw()`
- [x] Support methods in `impl` blocks (associated const); avoid name collisions; honour fn visibility
- [x] Tests: raw idents, methods, generics, invalid args (trybuild), private fns

### `tpt-fl-verified-algorithms`
- [x] Binary-search postcondition: `None => !slice.contains(key)` (currently vacuous)
- [x] Gate `should_panic` tests / doctests on `cfg(debug_assertions)` (they fail under `--release`)
- [x] Re-export or privatise `is_sorted_slice`
- [x] Document duplicate-key behaviour; add `lower_bound` / `upper_bound`
- [x] Tests: stability, multiplicity mismatch in `is_permutation`, large / all-equal inputs

---

## Phase 18 — Review Fixes: CI, Repo Hygiene & Release Readiness

- [x] CI: remove or restructure the `tpt-fl-deterministic-sim` publish dry-run until `tpt-fl-exact-math` is on crates.io
- [x] CI: MSRV job (Rust 1.75) incl. transitive deps (`contracts`, `syn`)
- [x] CI: `cargo test --workspace --release`
- [x] CI: `--no-default-features` and `tpt-fl-deterministic-sim --features exact-math` runs
- [x] CI: `no_std` thumbv7em build for all `no_std` crates (redundancy, verified-ode, verified-algorithms, det-proptest)
- [x] CI: `cargo-deny` / `cargo-audit` + `deny.toml`
- [ ] CI: `cargo-semver-checks`; optional coverage (`cargo-llvm-cov`) — semver-checks needs a published baseline; not added yet
- [x] CI: `continue-on-error` for nightly
- [ ] Verify CI actually triggers on push to `master` — config checked; needs a real push to confirm
- [ ] Commit `Cargo.lock` — no longer gitignored; the commit itself is pending
- [x] `#![forbid(unsafe_code)]` in every crate (missing from det-proptest, verified-algorithms, verified-ode)
- [x] `[workspace.lints]` section; extend `.clippy.toml` (e.g. `disallowed-methods` for determinism)
- [x] `publish = false` on the 5 crates not in the v0.1.0 batch until a publish plan exists
- [x] Fix no-op `exclude = ["/spec.txt", "/.github"]` in crate manifests
- [x] Per-crate README badge links `../LICENSE-MIT` → `LICENSE-MIT` (broken on crates.io)
- [x] Root README: "five focused crates" → ten; note unpublished crates' badges 404 until publish
- [x] CHANGELOG: reconcile 0.1.0 entry with the 5-crate publish batch; add compare-link footer
- [x] Consider `tpt-fl-verified-algorithms` dogfooding `tpt-fl-verify-macros` instead of external `contracts`
- [x] `todo.md` housekeeping: Phase 8 "CI activated" and Phase 16 "commit" items are stale; confirm `tpt-fl-proof-ast` compile-fail test exists

---

## Phase 19 — Missing Features (prioritised)

- [x] `tpt-fl-smt-bridge`: `check_sat()` spawning `z3 -in` / `cvc5 --lang smt2` with timeout and stderr capture (integration tests skip when solver absent)
- [x] `tpt-fl-smt-bridge`: arrays (`select`/`store`), bitvector ops, strings (with escaping), function application, `let` sharing, `push`/`pop`, unsat cores, `get-value`
- [x] `tpt-fl-smt-bridge`: optional sort checking (`sort_of` / `typecheck`)
- [ ] `tpt-fl-proof-ast` → `tpt-fl-smt-bridge` bridge (`From<Formula>`), plus visitor, substitution, free-variable analysis, predicate/function application, sorted binders — visitor, substitution, free variables, function and predicate application, and sorted binders are done. The bridge is NOT done: it would make tpt-fl-proof-ast depend on tpt-fl-smt-bridge, which changes the publish order and the dry-run check (needs a decision).
- [x] `tpt-fl-deterministic-sim`: `SimContext` (step count, `dt`, spawn/despawn queues), state hashing, snapshot/restore, `FnMut` systems
- [x] `tpt-fl-exact-math`: `FromStr`, `from_f64`, `pow`, `floor`/`ceil`/`round`, `&`-ref operator impls, wider `From<int>` impls, `Interval::{intersect, abs, sqr, pow, bisect}`
- [x] `tpt-fl-redundancy`: `approx_vote` (float tolerance), faulty-channel report, `plurality_vote`
- [x] `tpt-fl-verified-algorithms`: `sort_by` / `sort_by_key`, O(n log n) stable sort, Kani harnesses — Kani harnesses are written under `cfg(kani)` but not run (Kani is not installed here)
- [ ] `tpt-fl-verified-ode`: vector systems, adaptive step size, higher-order (Taylor-model) enclosures — adaptive step size is done as `solve_until`. Vector systems and Taylor-model enclosures are NOT done.
- [x] `tpt-fl-det-proptest`: tuple / `Vec` / `Option` strategies and combinators (`map`, `filter`, `one_of`)

---

## Phase 20 — Innovation Ideas (candidates, not committed)

- [ ] `tpt-assurance-case`: join `#[traces]`, contracts and verification results into a generated assurance case (GSN/SACM, traceability matrix); needs a `linkme`/`inventory` registry in `tpt-fl-trace-macros`
- [ ] Contract lowering: `#[requires]`/`#[ensures]` also emit Kani/Prusti/Creusot attributes under `cfg`; optionally discharge simple arithmetic contracts via `tpt-fl-smt-bridge` + Z3 at test time
- [ ] `DeterministicSim` record/replay: hash-chained step log, first-divergence bisection, golden-hash CI across x86/ARM/RISC-V (qemu)
- [ ] Numerical monitor: run `tpt-fl-verified-ode` alongside a float integrator and flag steps where the float result leaves the rigorous enclosure
- [ ] TMR-in-sim: `tpt-fl-redundancy` voters inside `DeterministicSim` with deterministic fault-injection schedules from `tpt-fl-det-proptest`
- [ ] Stateful / model-based testing in `tpt-fl-det-proptest` (command sequences, printed replay seeds)
- [ ] Proof certificates: keep Z3/cvc5 unsat proofs with tests; small independent checker for the exact-rational / interval fragment
- [ ] Benchmarks backing the "existing crates are slow" claim in `spec.txt`

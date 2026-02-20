# Tickets for PRD 003: Public API, Sorting, and Benchmarks

**Source PRD:** docs/PRD-003-public-api-sorting-benchmarks.md
**Created:** 2026-02-20
**Total Tickets:** 8
**Estimated Total Complexity:** 17 (S=1, M=2, L=3: T1=S + T2=M + T3=S + T4=M + T5=L + T6=L + T7=L + T8=M)

---

### Ticket 1: Crate Scaffold -- Convert to Library, Add Dependencies

**Description:**
Convert `src/main.rs` to a library crate by creating `src/lib.rs` as the crate root and
removing `src/main.rs`. Update `Cargo.toml` to declare the crate as a `[lib]`, add all
required dependencies (`criterion`, `memchr`, `unicode-normalization`), and create stub
module files (`src/ranking.rs`, `src/key.rs`, `src/diacritics.rs`, `src/sort.rs`) with
`// TODO` placeholders so the crate compiles from the start.

**Scope:**
- Create: `src/lib.rs` (module declarations + `#![warn(missing_docs)]`)
- Create: `src/ranking.rs` (stub -- placeholder for PRD-001 work)
- Create: `src/key.rs` (stub -- placeholder for PRD-002 work)
- Create: `src/diacritics.rs` (stub -- placeholder for diacritics removal)
- Create: `src/sort.rs` (stub -- placeholder for sorting logic)
- Modify: `Cargo.toml` (add `[lib]` section, add dependencies)
- Delete: `src/main.rs`

**Acceptance Criteria:**
- [ ] `Cargo.toml` has a `[lib]` section with `name = "matchsorter"` and no `[[bin]]` entry
- [ ] `Cargo.toml` has `criterion = { version = "0.5", features = ["html_reports"] }` under `[dev-dependencies]`
- [ ] `Cargo.toml` has `memchr = "2"` and `unicode-normalization = "0.1"` under `[dependencies]`
- [ ] `src/lib.rs` declares all four modules (`mod ranking`, `mod key`, `mod diacritics`, `mod sort`)
- [ ] `cargo build` succeeds with zero warnings after the conversion

**Dependencies:** None
**Complexity:** S
**Maps to PRD AC:** AC 7

---

### Ticket 2: Core Types -- `RankedItem`, `MatchSorterOptions`, and `Ranking` Re-export

**Description:**
Define `RankedItem<'a, T>` and `MatchSorterOptions<T>` in `src/lib.rs` with all fields
specified in the PRD. Implement `Default` for `MatchSorterOptions<T>` with JS-equivalent
defaults (`threshold = Ranking::Matches`, `keep_diacritics = false`, `base_sort = None`,
`sorter = None`). Re-export `Ranking` and `Key<T>` from their respective modules so the
public API surface is clean. Add doc comments on every public item.

**Scope:**
- Modify: `src/lib.rs` (add `RankedItem`, `MatchSorterOptions`, `Default` impl, re-exports)
- Modify: `src/ranking.rs` (add minimal `Ranking` enum stub with `Matches` variant + `f64` value, enough for `MatchSorterOptions::Default` to compile)
- Modify: `src/key.rs` (add minimal `Key<T>` struct stub with the fields listed in PRD-002 so the import compiles)

**Acceptance Criteria:**
- [ ] `RankedItem<'a, T>` has all six fields: `item`, `index`, `rank`, `ranked_value`, `key_index`, `key_threshold`
- [ ] `MatchSorterOptions<T>` has all five fields: `keys`, `threshold`, `base_sort`, `keep_diacritics`, `sorter`
- [ ] `MatchSorterOptions::<&str>::default()` compiles and sets `threshold = Ranking::Matches`, `keep_diacritics = false`, `base_sort = None`, `sorter = None`
- [ ] `use matchsorter::{RankedItem, MatchSorterOptions, Ranking, Key}` compiles in a doc-test
- [ ] All public structs and fields have doc comments; `cargo clippy -- -D warnings` clean

**Dependencies:** Ticket 1
**Complexity:** M
**Maps to PRD AC:** AC 1, AC 2, AC 3, AC 4, AC 5, AC 6 (types underpin all pipeline behavior)

---

### Ticket 3: `AsMatchStr` Trait -- No-Keys Mode Support

**Description:**
Define the `AsMatchStr` trait in `src/lib.rs` and implement it for `String`, `&str`, and
`Cow<'_, str>`. This trait is the hook that lets `match_sorter` accept plain string slices
without any `Key` configuration. The implementation must return `Vec<Cow<'_, str>>` to
support zero-copy borrowing from `&str` items while still allowing owned returns for
`String` items. Include unit tests in a `#[cfg(test)]` block.

**Scope:**
- Modify: `src/lib.rs` (add `AsMatchStr` trait definition and three impls, plus unit tests)

**Acceptance Criteria:**
- [ ] `"hello".as_match_str()` returns `vec![Cow::Borrowed("hello")]`
- [ ] `String::from("hello").as_match_str()` returns a `Vec` with the owned/borrowed string value
- [ ] `Cow::Borrowed("hello").as_match_str()` and `Cow::Owned(...)` both compile and return correct values
- [ ] All three impls have doc comments
- [ ] `cargo test` passes the `AsMatchStr` unit tests

**Dependencies:** Ticket 1
**Complexity:** S
**Maps to PRD AC:** AC 5

---

### Ticket 4: Sorting Module -- `sort_ranked_values` and `default_base_sort`

**Description:**
Implement `src/sort.rs` with two public functions: `default_base_sort<T>` (alphabetical
comparison of `ranked_value`) and `sort_ranked_values<T>` (three-level comparator: rank
descending, key_index ascending, base_sort tiebreaker). Both functions must match the
algorithm in PRD-003 Section 5 exactly. Include unit tests covering each comparison branch
and the tiebreaker path. Export both from `src/lib.rs`.

**Scope:**
- Modify: `src/sort.rs` (implement both functions with doc comments and unit tests)
- Modify: `src/lib.rs` (re-export `sort_ranked_values` and `default_base_sort`)

**Acceptance Criteria:**
- [ ] `sort_ranked_values` sorts higher rank before lower rank
- [ ] When ranks are equal, lower `key_index` sorts first
- [ ] When rank and key_index are equal, `default_base_sort` sorts alphabetically by `ranked_value`
- [ ] A custom `base_sort` closure passed to `sort_ranked_values` overrides alphabetical order
- [ ] `default_base_sort` is re-exported from `src/lib.rs` and has a doc comment
- [ ] `cargo test` passes all unit tests in `src/sort.rs`

**Dependencies:** Ticket 2
**Complexity:** M
**Maps to PRD AC:** AC 3, AC 4

---

### Ticket 5: `match_sorter` Function -- Full Pipeline Implementation

**Description:**
Implement the `match_sorter<T>` public function in `src/lib.rs` following the three-step
pipeline in PRD-003 Section 7: (1) rank and filter items against the threshold, (2) sort
via `sorter` override or `sort_ranked_values` with the effective `base_sort`, (3) map to
`Vec<&T>`. Wire the no-keys path through `AsMatchStr` and the keys path through
`get_highest_ranking` from `src/key.rs`. Handle the effective threshold per-item using
`key_threshold` when set. This ticket assumes `get_highest_ranking` and `get_match_ranking`
are already implemented (PRD-001 and PRD-002 work); if stubs are in place, implement against
the stub signature and note that integration requires those modules to be complete.

**Scope:**
- Modify: `src/lib.rs` (implement `match_sorter` function body, wire pipeline, add doc comments and examples)
- Modify: `src/key.rs` (add `get_highest_ranking` stub with correct signature if not already present, so the call site compiles)

**Acceptance Criteria:**
- [ ] `match_sorter(&["apple", "banana", "grape"], "ap", MatchSorterOptions::default())` compiles and, once ranking modules are complete, returns `["apple", "grape"]`
- [ ] When `options.sorter` is `Some`, the custom sorter is called instead of `sort_ranked_values`
- [ ] When `options.keys` is empty, items are ranked via `AsMatchStr::as_match_str()`
- [ ] The threshold filter uses `key_threshold` when set, otherwise `options.threshold`
- [ ] The function signature matches the PRD: `fn match_sorter<'a, T>(items: &'a [T], value: &str, options: MatchSorterOptions<T>) -> Vec<&'a T>`
- [ ] Doc comment with example compiles as a doc-test
- [ ] `cargo clippy -- -D warnings` clean

**Dependencies:** Ticket 2, Ticket 3, Ticket 4
**Complexity:** L
**Maps to PRD AC:** AC 1, AC 2, AC 3, AC 4, AC 5, AC 6

---

### Ticket 6: Integration Tests -- Port JS Test Suite

**Description:**
Create `tests/integration.rs` and port the JS `match-sorter` test cases listed in PRD-003
Section 12 to Rust. Cover every scenario: basic string arrays, case sensitivity, diacritics,
threshold filtering, key-based matching with structs, multi-value keys, per-key
threshold/min/max, custom `base_sort` preserving original order, `sorter` override, empty
query, single-char query, acronym matching, word boundary detection, and edge cases (empty
items, very long strings). Each test must assert exact equality of returned slices/values
against the expected JS-equivalent output. This ticket depends on PRD-001 and PRD-002
modules being functionally complete, not just stubbed.

**Scope:**
- Create: `tests/integration.rs` (full integration test file; ~30-40 test functions)

**Acceptance Criteria:**
- [ ] At least one test per scenario listed in PRD-003 Section 12 (14 scenario categories)
- [ ] `match_sorter(&["apple", "banana", "grape"], "ap", MatchSorterOptions::default())` returns `[&"apple", &"grape"]` in that order
- [ ] Threshold test: `threshold = Ranking::Contains` excludes items that only match via fuzzy
- [ ] Key-based test: struct with `.name` field matches correctly against a `Key::new` extractor
- [ ] Diacritics test: `"cafe"` matches `"cafe"` when `keep_diacritics = false`
- [ ] `cargo test --test integration` passes with zero failures

**Dependencies:** Ticket 5
**Complexity:** L
**Maps to PRD AC:** AC 1, AC 2, AC 3, AC 4, AC 5, AC 6, AC 10, AC 15

---

### Ticket 7: Criterion Benchmarks

**Description:**
Create `benches/benchmarks.rs` with the full benchmark suite from PRD-003 Section 9. Add
a `[[bench]]` entry to `Cargo.toml`. Implement benchmarks for: throughput at small (100),
medium (10,000), and large (100,000) item counts across all five query types (exact, prefix,
substring, fuzzy, no-match); isolated `get_match_ranking` hot path; diacritics overhead
(with vs without `keep_diacritics`); sort overhead for pre-ranked item sets; and single-key
vs multi-key configurations. Use `criterion::black_box` to prevent dead-code elimination.
Add a `rayon` feature flag in `Cargo.toml` and a parallel-ranking benchmark guarded by
`#[cfg(feature = "rayon")]`.

**Scope:**
- Create: `benches/benchmarks.rs` (full criterion benchmark file)
- Modify: `Cargo.toml` (add `[[bench]]` entry, add `rayon` optional dependency under `[features]`, add `rayon` to dev-dependencies for the feature)

**Acceptance Criteria:**
- [ ] `cargo bench` runs without errors and produces criterion HTML reports in `target/criterion/`
- [ ] Throughput benchmarks exist for all three dataset sizes (100, 10_000, 100_000 items)
- [ ] All five query types are benchmarked (exact, prefix, substring, fuzzy, no-match)
- [ ] A `get_match_ranking` micro-benchmark exists (hot path isolation)
- [ ] A diacritics overhead benchmark compares `keep_diacritics: true` vs `false`
- [ ] A sort overhead benchmark exists for pre-ranked item sets
- [ ] The `rayon` feature flag compiles cleanly: `cargo bench --features rayon`
- [ ] `cargo clippy -- -D warnings` clean on `benches/benchmarks.rs`

**Dependencies:** Ticket 5
**Complexity:** L
**Maps to PRD AC:** AC 8, AC 9

---

### Ticket 8: Verification and Integration Check

**Description:**
Run the full PRD-003 acceptance criteria checklist end-to-end. Verify all tickets integrate
correctly as a cohesive library crate. Confirm performance targets are met by inspecting
criterion output. Verify the public API surface is complete and correctly re-exported.

**Acceptance Criteria:**
- [ ] `cargo test` passes all unit tests and integration tests with zero failures
- [ ] `cargo bench` completes without panics; throughput for 10,000 items with a single key is under 10ms as shown in criterion output
- [ ] `cargo clippy -- -D warnings` produces zero warnings
- [ ] `cargo fmt --check` produces no diff
- [ ] `cargo doc --no-deps` builds without warnings; all public items have doc comments
- [ ] Zero `unsafe` blocks: `grep -r "unsafe" src/` returns no matches
- [ ] The crate is a library: no `src/main.rs`, `Cargo.toml` has `[lib]` and no `[[bin]]`
- [ ] `use matchsorter::{match_sorter, MatchSorterOptions, Key, Ranking, RankedItem, default_base_sort}` compiles in a doc-test
- [ ] All 15 PRD acceptance criteria pass (verified manually or via test assertions)

**Dependencies:** Tickets 1 through 7
**Complexity:** M
**Maps to PRD AC:** AC 1-15 (all)

---

## AC Coverage Matrix

| PRD AC # | Description | Covered By Ticket(s) | Status |
|----------|-------------|----------------------|--------|
| 1 | `match_sorter(&["apple","banana","grape"], "ap", default)` returns `["apple","grape"]` | Ticket 5, Ticket 6, Ticket 8 | Covered |
| 2 | Threshold filtering: `threshold = Contains` excludes fuzzy-only matches | Ticket 2, Ticket 5, Ticket 6 | Covered |
| 3 | Custom `base_sort` preserving original order works | Ticket 4, Ticket 5, Ticket 6 | Covered |
| 4 | Custom `sorter` override completely replaces sorting logic | Ticket 5, Ticket 6 | Covered |
| 5 | No-keys mode works with `Vec<String>`, `Vec<&str>` | Ticket 3, Ticket 5, Ticket 6 | Covered |
| 6 | Key-based mode works with custom structs | Ticket 5, Ticket 6 | Covered |
| 7 | Crate compiles as a library (`lib.rs`, not `main.rs`) | Ticket 1, Ticket 8 | Covered |
| 8 | Criterion benchmarks exist and run via `cargo bench` | Ticket 7, Ticket 8 | Covered |
| 9 | Performance meets targets (< 10ms for 10k items single key) | Ticket 7, Ticket 8 | Covered |
| 10 | `cargo test` passes all unit + integration tests | Ticket 6, Ticket 8 | Covered |
| 11 | `cargo clippy -- -D warnings` clean | Tickets 1-7 (each), Ticket 8 | Covered |
| 12 | `cargo fmt --check` clean | Tickets 1-7 (each), Ticket 8 | Covered |
| 13 | Zero `unsafe` blocks | Tickets 1-7 (convention), Ticket 8 | Covered |
| 14 | All public items have doc comments | Ticket 2, Ticket 4, Ticket 5, Ticket 8 | Covered |
| 15 | Results match JS `match-sorter` behavior for equivalent inputs | Ticket 6, Ticket 8 | Covered |

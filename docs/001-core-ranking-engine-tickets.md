# Tickets for PRD 001: Core Ranking Engine

**Source PRD:** docs/PRD-001-core-ranking-engine.md
**Created:** 2026-02-20
**Total Tickets:** 8
**Estimated Total Complexity:** 17 (S=1, M=2, L=3: 1+2+2+1+2+3+3+3)

---

### Ticket 1: Cargo Scaffold -- Dependencies and Module Skeleton

**Description:**
Add all required crate dependencies to `Cargo.toml`, convert `src/main.rs` to a library crate
(`src/lib.rs`), and declare the `ranking` module with an empty `src/ranking/mod.rs`. This is
the bedrock that every subsequent ticket compiles against.

**Scope:**
- Modify: `Cargo.toml` (add `unicode-normalization`, `memchr`)
- Create: `src/lib.rs` (replaces `src/main.rs`; declares `pub mod ranking`)
- Create: `src/ranking/mod.rs` (empty module stub with `#![allow(dead_code)]` placeholder comment)

**Acceptance Criteria:**
- [ ] `Cargo.toml` lists `unicode-normalization` and `memchr` as dependencies with pinned minor versions
- [ ] `cargo build` succeeds with zero warnings after replacing `main.rs` with `lib.rs`
- [ ] `src/ranking/mod.rs` exists and is declared in `lib.rs`
- [ ] No `unsafe` blocks in any file touched by this ticket

**Dependencies:** None
**Complexity:** S
**Maps to PRD AC:** AC 15, AC 17 (quality gate foundation)

---

### Ticket 2: `Ranking` Type -- Enum, Sub-Score, and Ordering

**Description:**
Define the `Ranking` enum representing all 8 tiers, with `Matches(f64)` carrying its continuous
sub-score. Implement `PartialOrd`, `PartialEq`, `Debug`, and `Clone`. All numeric tier values
(`CaseSensitiveEqual = 7` down to `NoMatch = 0`) must be representable and orderable. Include
unit tests for ordering invariants inside `src/ranking/mod.rs`.

**Scope:**
- Modify: `src/ranking/mod.rs` (add `Ranking` enum, trait impls, `#[cfg(test)]` unit tests)

**Acceptance Criteria:**
- [ ] `Ranking::CaseSensitiveEqual > Ranking::Equal > Ranking::StartsWith > Ranking::WordStartsWith > Ranking::Contains > Ranking::Acronym > Ranking::Matches(1.5) > Ranking::NoMatch` all hold under `PartialOrd`
- [ ] `Ranking::Matches(1.9) > Ranking::Matches(1.1)` holds (higher sub-score ranks higher)
- [ ] `Ranking::Matches(f64)` sub-score is constrained to `(1.0, 2.0]` by convention (doc comment documents the invariant; no runtime panic needed)
- [ ] All variants derive `Debug` and `Clone`
- [ ] `cargo test` passes for the unit tests in this file

**Dependencies:** Ticket 1
**Complexity:** M
**Maps to PRD AC:** AC 1, AC 8

---

### Ticket 3: `prepare_value_for_comparison` -- Diacritics Stripping

**Description:**
Implement `prepare_value_for_comparison(s: &str, keep_diacritics: bool) -> Cow<'_, str>` in
`src/ranking/mod.rs`. When `keep_diacritics` is `false`, apply Unicode NFD decomposition and
strip combining characters (Unicode category `Mn`) using `unicode-normalization`. If no
characters are stripped, return the input borrowed as `Cow::Borrowed`; only allocate when
normalization actually changes the string. Include unit tests.

**Scope:**
- Modify: `src/ranking/mod.rs` (add `prepare_value_for_comparison`, unit tests)

**Acceptance Criteria:**
- [ ] `prepare_value_for_comparison("cafe\u{0301}", false)` returns `Cow::Owned("cafe")` (accent stripped)
- [ ] `prepare_value_for_comparison("cafe", false)` returns `Cow::Borrowed("cafe")` (no allocation)
- [ ] `prepare_value_for_comparison("cafe\u{0301}", true)` returns the original string unchanged
- [ ] No `unsafe` blocks
- [ ] `cargo test` passes for unit tests in this function

**Dependencies:** Ticket 1
**Complexity:** M
**Maps to PRD AC:** AC 10, AC 11, AC 15

---

### Ticket 4: `get_acronym` -- Word-Boundary Acronym Extraction

**Description:**
Implement `get_acronym(s: &str) -> String` in `src/ranking/mod.rs`. Word boundaries are space
(`' '`) and hyphen (`'-'`) only. The first character is always included; subsequent characters
are included when the previous character was a delimiter. The caller is responsible for
lowercasing before calling. Include unit tests covering hyphens, spaces, single words, and
empty strings.

**Scope:**
- Modify: `src/ranking/mod.rs` (add `get_acronym`, unit tests)

**Acceptance Criteria:**
- [ ] `get_acronym("north-west airlines")` returns `"nwa"` (hyphen and space both act as delimiters)
- [ ] `get_acronym("san francisco")` returns `"sf"` (space delimiter only)
- [ ] `get_acronym("single")` returns `"s"` (no delimiters, first char only)
- [ ] `get_acronym("")` returns `""` (empty input safe)
- [ ] Underscores do NOT act as word boundaries
- [ ] `cargo test` passes for unit tests in this function

**Dependencies:** Ticket 1
**Complexity:** S
**Maps to PRD AC:** AC 7, AC 14

---

### Ticket 5: `get_closeness_ranking` -- Fuzzy Character-by-Character Scorer

**Description:**
Implement `get_closeness_ranking(candidate: &str, query: &str) -> Ranking` in
`src/ranking/mod.rs`. Perform a greedy forward scan: for each character in `query`, advance
through `candidate` to find it (using `.chars()` for Unicode correctness). If any character is
not found, return `Ranking::NoMatch`. Otherwise compute `spread = last_match_index -
first_match_index` (in char positions) and return `Ranking::Matches(1.0 + 1.0 / spread as
f64)`. When `spread == 0` (query length 1, single matched char), return `Ranking::Matches(2.0)`
as the upper-bound. Include unit tests.

**Scope:**
- Modify: `src/ranking/mod.rs` (add `get_closeness_ranking`, unit tests)

**Acceptance Criteria:**
- [ ] `get_closeness_ranking("playground", "plgnd")` returns `Ranking::Matches(s)` where `1.0 < s < 2.0`
- [ ] `get_closeness_ranking("abc", "xyz")` returns `Ranking::NoMatch`
- [ ] `get_closeness_ranking("ab", "a")` returns `Ranking::Matches(2.0)` (spread = 0, single char hit)
- [ ] Return value for any `Matches` case is in the half-open range `(1.0, 2.0]`
- [ ] No `unsafe` blocks
- [ ] `cargo test` passes for unit tests in this function

**Dependencies:** Ticket 2
**Complexity:** M
**Maps to PRD AC:** AC 8, AC 12, AC 15

---

### Ticket 6: `get_match_ranking` -- Top-Level Ranking Orchestrator

**Description:**
Implement the public `get_match_ranking(test_string: &str, string_to_rank: &str, keep_diacritics:
bool) -> Ranking` function in `src/ranking/mod.rs`. Wire together `prepare_value_for_comparison`,
`get_acronym`, and `get_closeness_ranking` following the 11-step algorithm in the PRD exactly.
Use `memchr` for fast single-byte substring scanning in the `Contains`/`StartsWith`/`WordStartsWith`
branches where applicable. Use a lazy iterator (not a collected `Vec`) for finding all match
indexes. Export `get_match_ranking` and `Ranking` from `src/lib.rs` as the public surface. Include
inline unit tests for every tier transition.

**Scope:**
- Modify: `src/ranking/mod.rs` (add `get_match_ranking`, all-indexes iterator, inline unit tests)
- Modify: `src/lib.rs` (re-export `ranking::get_match_ranking` and `ranking::Ranking`)

**Acceptance Criteria:**
- [ ] `get_match_ranking("Green", "green", false)` returns `Ranking::Equal`
- [ ] `get_match_ranking("Green", "Green", false)` returns `Ranking::CaseSensitiveEqual`
- [ ] `get_match_ranking("Greenland", "green", false)` returns `Ranking::StartsWith`
- [ ] `get_match_ranking("San Francisco", "fran", false)` returns `Ranking::WordStartsWith`
- [ ] `get_match_ranking("abcdef", "cde", false)` returns `Ranking::Contains`
- [ ] `get_match_ranking("North-West Airlines", "nwa", false)` returns `Ranking::Acronym`
- [ ] `get_match_ranking("playground", "plgnd", false)` returns `Ranking::Matches(s)` with `1.0 < s < 2.0`
- [ ] `get_match_ranking("abc", "xyz", false)` returns `Ranking::NoMatch`
- [ ] Query longer than candidate returns `Ranking::NoMatch` immediately
- [ ] Single-char query that is not a substring returns `Ranking::NoMatch` (step 9: no acronym/fuzzy attempted)
- [ ] Empty query against any non-empty string returns `Ranking::StartsWith`
- [ ] `get_match_ranking` and `Ranking` are accessible from the crate root (re-exported in `lib.rs`)
- [ ] No `unsafe` blocks

**Dependencies:** Tickets 3, 4, 5
**Complexity:** L
**Maps to PRD AC:** AC 1, AC 2, AC 3, AC 4, AC 5, AC 6, AC 7, AC 8, AC 9, AC 12, AC 13, AC 14

---

### Ticket 7: Integration Test Suite -- All AC Scenarios

**Description:**
Write a comprehensive integration test file at `tests/ranking.rs` that exercises
`get_match_ranking` through the crate's public API. Cover every PRD acceptance criterion that
specifies a concrete input/output pair, including diacritics toggling, word-boundary semantics,
empty/single-char queries, and fuzzy sub-score range assertions. Tests must use only the public
API exported from `lib.rs`.

**Scope:**
- Create: `tests/ranking.rs` (integration test file, ~20 test functions)

**Acceptance Criteria:**
- [ ] Each of PRD AC 2-14 has a dedicated `#[test]` function in `tests/ranking.rs`
- [ ] Diacritics test: `get_match_ranking("caf\u{00e9}", "cafe", false)` returns `Ranking::Equal` (AC 10)
- [ ] Diacritics-kept test: `get_match_ranking("caf\u{00e9}", "cafe", true)` returns `Ranking::NoMatch` or a tier below `Equal` (AC 11)
- [ ] Fuzzy sub-score test asserts `matches_score > 1.0 && matches_score < 2.0` (AC 8)
- [ ] `cargo test` passes with all integration tests green

**Dependencies:** Ticket 6
**Complexity:** L
**Maps to PRD AC:** AC 2, AC 3, AC 4, AC 5, AC 6, AC 7, AC 8, AC 9, AC 10, AC 11, AC 12, AC 13, AC 14, AC 16

---

### Ticket 8: Verification and Quality Gates

**Description:**
Run the full quality gate suite -- test, lint, and format check -- to confirm that all PRD
acceptance criteria are satisfied end-to-end and the codebase meets the project's code quality
standards. No new source code is written in this ticket; it exists to confirm the integrated
result is shippable.

**Acceptance Criteria:**
- [ ] `cargo test` passes with zero failures
- [ ] `cargo clippy -- -D warnings` exits with zero warnings or errors
- [ ] `cargo fmt --check` exits cleanly (no formatting differences)
- [ ] `grep -r "unsafe" src/ tests/` returns no results (AC 15)
- [ ] All PRD acceptance criteria AC 1-17 verified as passing

**Dependencies:** All previous tickets
**Complexity:** S
**Maps to PRD AC:** AC 15, AC 16, AC 17

---

## AC Coverage Matrix

| PRD AC # | Description                                                                          | Covered By Ticket(s)  | Status  |
|----------|--------------------------------------------------------------------------------------|-----------------------|---------|
| 1        | All 8 tiers produce identical results to JS version for same inputs                  | Ticket 2, Ticket 6    | Covered |
| 2        | `get_match_ranking("Green", "green")` -> `Equal`                                     | Ticket 6, Ticket 7    | Covered |
| 3        | `get_match_ranking("Green", "Green")` -> `CaseSensitiveEqual`                        | Ticket 6, Ticket 7    | Covered |
| 4        | `get_match_ranking("Greenland", "green")` -> `StartsWith`                            | Ticket 6, Ticket 7    | Covered |
| 5        | `get_match_ranking("San Francisco", "fran")` -> `WordStartsWith`                     | Ticket 6, Ticket 7    | Covered |
| 6        | `get_match_ranking("abcdef", "cde")` -> `Contains`                                   | Ticket 6, Ticket 7    | Covered |
| 7        | `get_match_ranking("North-West Airlines", "nwa")` -> `Acronym`                       | Ticket 4, Ticket 6, Ticket 7 | Covered |
| 8        | `get_match_ranking("playground", "plgnd")` -> `Matches` with sub-score in (1.0, 2.0) | Ticket 5, Ticket 6, Ticket 7 | Covered |
| 9        | `get_match_ranking("abc", "xyz")` -> `NoMatch`                                       | Ticket 6, Ticket 7    | Covered |
| 10       | `keep_diacritics: false` -> `Equal` for accented input matching unaccented query     | Ticket 3, Ticket 7    | Covered |
| 11       | `keep_diacritics: true` -> lower tier or `NoMatch` when accents differ               | Ticket 3, Ticket 7    | Covered |
| 12       | Single-char query not a substring -> `NoMatch` (no acronym/fuzzy attempted)          | Ticket 6, Ticket 7    | Covered |
| 13       | Empty query against any non-empty string -> `StartsWith`                             | Ticket 6, Ticket 7    | Covered |
| 14       | Word boundary detection uses only spaces, not hyphens/underscores (for `WordStartsWith`) | Ticket 4, Ticket 6, Ticket 7 | Covered |
| 15       | Zero `unsafe` blocks in all code                                                     | Ticket 8 (grep verify) | Covered |
| 16       | Unit tests covering every tier and edge case                                         | Ticket 7, Ticket 8    | Covered |
| 17       | `cargo test` passes, `cargo clippy -- -D warnings` clean, `cargo fmt --check` clean  | Ticket 8              | Covered |

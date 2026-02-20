# Tickets for PRD 002: Key Extraction and Value Resolution

**Source PRD:** docs/PRD-002-key-extraction.md
**Created:** 2026-02-20
**Total Tickets:** 5
**Estimated Total Complexity:** 12 (S=1, M=2, L=3)

> **Prerequisite assumption:** PRD-001 (Core Ranking Engine) is complete. These tickets assume
> `src/ranking.rs` exports a `Ranking` type (an `f64`-backed or enum type) and a
> `get_match_ranking` function. If PRD-001 is not yet merged, complete it first.

---

### Ticket 1: `Key<T>` and `RankingInfo` Types + Builder API

**Description:**
Define all foundational types for the key system in a new `src/key.rs` module: the `Key<T>` struct
with its boxed extractor closure and optional ranking attributes (`threshold`, `min_ranking`,
`max_ranking`), the `RankingInfo` struct, and the builder methods (`.threshold()`,
`.max_ranking()`, `.min_ranking()`). Also add `Key::new`, `Key::from_fn`, and `Key::from_fn_multi`
constructors. No extraction logic is implemented here -- only the data types and their construction
API, with unit tests covering builder chaining and constructor equivalence.

**Scope:**
- Create: `src/key.rs` (all types, constructors, builder methods, `#[cfg(test)]` unit tests)
- Modify: `src/lib.rs` -- add `pub mod key;` and re-export `Key`, `RankingInfo`

**Acceptance Criteria:**
- [ ] `Key<T>` struct compiles with fields: boxed extractor `Box<dyn Fn(&T) -> Vec<Cow<'_, str>>>`,
      `threshold: Option<Ranking>`, `min_ranking: Ranking`, `max_ranking: Ranking`
- [ ] `Key::new(|item: &T| ...)` accepts a closure returning `Vec<String>` or `Vec<Cow<'_, str>>`
- [ ] `Key::from_fn(|item: &T| item.field.as_str())` constructs a single-value key without allocating
- [ ] `Key::from_fn_multi(|item: &T| vec![...])` constructs a multi-value borrowed key
- [ ] Builder chain `Key::new(...).threshold(r).max_ranking(r).min_ranking(r)` compiles and sets fields
- [ ] `RankingInfo` struct has fields: `rank: f64`, `ranked_value: String`, `key_index: usize`,
      `key_threshold: Option<Ranking>`
- [ ] All public items have doc comments (required by global CLAUDE.md rules)
- [ ] `cargo build` with no warnings; `cargo clippy -- -D warnings` clean

**Dependencies:** PRD-001 complete (needs `Ranking` type from `src/ranking.rs`)
**Complexity:** M
**Maps to PRD AC:** AC 1, AC 2

---

### Ticket 2: `get_item_values` -- Value Extraction Logic

**Description:**
Implement the `get_item_values` function in `src/key.rs` (or a new `src/extraction.rs` if it grows
too large). This function takes an item reference and a `Key<T>`, calls the extractor closure, and
returns a `Vec<Cow<'_, str>>` of all extracted values. Also add the `MatchSorterOptions` struct
placeholder (with at least the `keep_diacritics: bool` field) so that `get_item_values` can
reference it for any option-dependent normalization. Write unit tests covering: single-value
extraction, multi-value extraction, and empty-return (no candidates).

**Scope:**
- Modify: `src/key.rs` -- add `pub fn get_item_values<T>(item: &T, key: &Key<T>) -> Vec<Cow<'_, str>>`
- Create: `src/options.rs` -- `MatchSorterOptions` struct with at minimum `keep_diacritics: bool`
- Modify: `src/lib.rs` -- add `pub mod options;` and re-export `MatchSorterOptions`

**Acceptance Criteria:**
- [ ] `get_item_values` calls the key's extractor and returns all values as `Cow<'_, str>`
- [ ] Single-value key returns a `Vec` of length 1
- [ ] Multi-value key (tags array) returns a `Vec` with all tag values
- [ ] Extractor returning an empty `Vec` causes `get_item_values` to return an empty `Vec`
- [ ] `MatchSorterOptions` derives `Debug`, `Clone`, `Default`, and has `keep_diacritics: bool`
      defaulting to `false`
- [ ] Unit tests cover all three extraction cases (single, multi, empty)
- [ ] `cargo test` passes; `cargo clippy -- -D warnings` clean

**Dependencies:** Ticket 1
**Complexity:** M
**Maps to PRD AC:** AC 1, AC 8

---

### Ticket 3: `get_highest_ranking` -- Multi-Key Evaluation with Clamping

**Description:**
Implement `get_highest_ranking<T>(item: &T, keys: &[Key<T>], query: &str, options: &MatchSorterOptions) -> RankingInfo`.
Flatten all keys' extracted values into a single indexed list (preserving key order), call
`get_match_ranking` for each value, apply `min_ranking` / `max_ranking` clamping per the PRD
algorithm (NoMatch is never promoted), and return the best `RankingInfo`. Ties are broken by
returning the lower `key_index`. Write unit tests covering: single key, multiple keys where second
key wins, max_ranking clamping, min_ranking promotion of non-NoMatch, min_ranking leaving NoMatch
alone, and tie-breaking by key index.

**Scope:**
- Modify: `src/key.rs` -- add `pub fn get_highest_ranking<T>(...) -> RankingInfo`
- Modify: `src/lib.rs` -- re-export `get_highest_ranking`

**Acceptance Criteria:**
- [ ] Returns best ranking across all keys; multiple keys evaluated in order
- [ ] `max_ranking = Contains` clamps a `StartsWith` result down to `Contains` rank value
- [ ] `min_ranking = Contains` promotes a `Matches` result up to `Contains` (non-NoMatch)
- [ ] `min_ranking` does NOT promote `NoMatch` -- item with no match anywhere stays `NoMatch`
- [ ] When two key-values produce the same rank, lower `key_index` wins (earlier key takes priority)
- [ ] `key_threshold` in the returned `RankingInfo` reflects the key's `threshold` field if set
- [ ] Unit tests cover all clamping/promotion cases and the tie-breaking rule
- [ ] `cargo test` passes; `cargo clippy -- -D warnings` clean

**Dependencies:** Ticket 2 (needs `get_item_values`, `MatchSorterOptions`)
**Complexity:** L
**Maps to PRD AC:** AC 3, AC 4, AC 5, AC 6, AC 7, AC 8

---

### Ticket 4: No-Keys Mode via `AsMatchSorterStr` Trait

**Description:**
Support calling the ranking machinery without any `Key<T>` when the item type itself is a string.
Define a `AsMatchSorterStr` trait (or equivalent sealed trait) that is implemented for `String`,
`&str`, and `Cow<'_, str>`. Provide a function or impl that converts a slice of such items into
a `Key<T>`-compatible form (or overloads `get_highest_ranking` to accept items directly when no
keys are passed). Write unit tests demonstrating that `Vec<String>` and `Vec<&str>` items are
ranked correctly without constructing any `Key`.

**Scope:**
- Create: `src/no_keys.rs` -- `AsMatchSorterStr` trait + impls for `String`, `&str`, `Cow<'_, str>`
- Modify: `src/lib.rs` -- add `pub mod no_keys;` and re-export the trait

**Acceptance Criteria:**
- [ ] `AsMatchSorterStr` trait is implemented for `String`, `&str`, and `Cow<'_, str>`
- [ ] A `Vec<String>` of items can be ranked against a query without constructing a `Key`
- [ ] A `Vec<&str>` of items can be ranked against a query without constructing a `Key`
- [ ] The no-keys path reuses `get_match_ranking` from PRD-001 (no duplicated logic)
- [ ] Unit tests verify both `Vec<String>` and `Vec<&str>` produce correct rankings
- [ ] `cargo test` passes; `cargo clippy -- -D warnings` clean

**Dependencies:** Ticket 3 (needs `get_highest_ranking` for the delegation path)
**Complexity:** M
**Maps to PRD AC:** AC 9

---

### Ticket 5: Verification and Integration Tests

**Description:**
Run the full PRD-002 acceptance criteria checklist end-to-end. Write an integration test file at
`tests/key_extraction.rs` that exercises every AC using realistic item types (a `User` struct
with `name`, `email`, `tags` fields). Verify zero `unsafe` blocks, all quality gates pass.

**Scope:**
- Create: `tests/key_extraction.rs` -- integration tests for all PRD-002 ACs
- Modify: `src/lib.rs` -- ensure all public items needed by integration tests are exported

**Acceptance Criteria:**
- [ ] `Key::new`, `Key::from_fn`, `Key::from_fn_multi` all compile and produce correct rankings in tests
- [ ] Builder methods `.threshold()`, `.max_ranking()`, `.min_ranking()` set fields verified by test
- [ ] `max_ranking = Contains` test: a `StartsWith` match is clamped to `Contains` rank
- [ ] `min_ranking` promotion test: a `Matches`-tier result is promoted to `min_ranking` value
- [ ] `min_ranking` no-promotion test: `NoMatch` stays `NoMatch` even with `min_ranking` set
- [ ] Multi-key test: key slice with two keys; best-ranking key wins; earlier-key tiebreak verified
- [ ] Multi-value key test: item with `tags: Vec<String>` -- each tag ranked independently
- [ ] No-keys test: `Vec<String>` and `Vec<&str>` ranked correctly without any `Key`
- [ ] `grep -r "unsafe" src/` returns no matches
- [ ] `cargo test` passes (all unit + integration tests)
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo fmt --check` clean

**Dependencies:** All previous tickets
**Complexity:** M
**Maps to PRD AC:** AC 1, AC 2, AC 3, AC 4, AC 5, AC 6, AC 7, AC 8, AC 9, AC 10, AC 11, AC 12

---

## AC Coverage Matrix

| PRD AC # | Description | Covered By Ticket(s) | Status |
|----------|-------------|----------------------|--------|
| 1 | `Key::new` accepts closures returning `Vec<String>` or equivalent | Ticket 1, Ticket 5 | Covered |
| 2 | Builder methods `.threshold()`, `.max_ranking()`, `.min_ranking()` work correctly | Ticket 1, Ticket 5 | Covered |
| 3 | `max_ranking` clamps rankings down | Ticket 3, Ticket 5 | Covered |
| 4 | `min_ranking` promotes non-NoMatch rankings | Ticket 3, Ticket 5 | Covered |
| 5 | `min_ranking` does NOT promote `NoMatch` | Ticket 3, Ticket 5 | Covered |
| 6 | Multiple keys evaluated in order; best ranking wins | Ticket 3, Ticket 5 | Covered |
| 7 | Earlier key wins on equal rank (lower key_index) | Ticket 3, Ticket 5 | Covered |
| 8 | Multi-value keys rank each value independently; best wins | Ticket 2, Ticket 3, Ticket 5 | Covered |
| 9 | No-keys mode works with `Vec<String>` and `Vec<&str>` | Ticket 4, Ticket 5 | Covered |
| 10 | Zero `unsafe` blocks | Ticket 5 | Covered |
| 11 | Unit tests for all key configurations and edge cases | Ticket 1, Ticket 2, Ticket 3, Ticket 4, Ticket 5 | Covered |
| 12 | `cargo test` passes, `cargo clippy -- -D warnings` clean, `cargo fmt --check` clean | Ticket 5 | Covered |

# Code Review: Ticket 7 -- Integration Test Suite -- All AC Scenarios

**Ticket:** 7 -- Integration Test Suite -- All AC Scenarios
**Impl Report:** docs/001-core-ranking-engine-reports/ticket-07-impl.md
**Date:** 2026-02-20 08:00
**Verdict:** APPROVED

---

## AC Coverage

| AC # | Description | Status | Notes |
|------|-------------|--------|-------|
| Ticket AC 1 | Each of PRD AC 2-14 has a dedicated `#[test]` function | Met | Functions `ac02_equal` through `ac14_word_boundary_spaces_only` (13 functions, one per AC) confirmed in `tests/ranking.rs` |
| Ticket AC 2 (PRD AC 10) | Diacritics test returns `Ranking::Equal` or above | Met | `ac10_diacritics_stripped` asserts `CaseSensitiveEqual` AND verifies `result >= Ranking::Equal`. See notes below. |
| Ticket AC 3 (PRD AC 11) | Diacritics-kept test returns `NoMatch` or below `Equal` | Met | `ac11_diacritics_kept` asserts `Ranking::NoMatch` for `get_match_ranking("caf\u{00e9}", "cafe", true)` |
| Ticket AC 4 (PRD AC 8) | Fuzzy sub-score asserts `score > 1.0 && score < 2.0` | Met | `ac08_fuzzy_matches_sub_score` uses pattern matching; panics with descriptive message on wrong variant |
| Ticket AC 5 | `cargo test` passes with all 18 integration tests green | Met | Verified by running `cargo test`: 18/18 integration tests pass; 65 unit + 4 doc-tests also pass. Total 87. |

### PRD AC 2-14 Individual Coverage

| PRD AC | Test Function | Verified |
|--------|--------------|---------|
| AC 2: Equal | `ac02_equal` | `get_match_ranking("Green", "green", false)` == `Ranking::Equal` |
| AC 3: CaseSensitiveEqual | `ac03_case_sensitive_equal` | `get_match_ranking("Green", "Green", false)` == `Ranking::CaseSensitiveEqual` |
| AC 4: StartsWith | `ac04_starts_with` | `get_match_ranking("Greenland", "green", false)` == `Ranking::StartsWith` |
| AC 5: WordStartsWith | `ac05_word_starts_with` | `get_match_ranking("San Francisco", "fran", false)` == `Ranking::WordStartsWith` |
| AC 6: Contains | `ac06_contains` | `get_match_ranking("abcdef", "cde", false)` == `Ranking::Contains` |
| AC 7: Acronym | `ac07_acronym` | `get_match_ranking("North-West Airlines", "nwa", false)` == `Ranking::Acronym` |
| AC 8: Matches sub-score | `ac08_fuzzy_matches_sub_score` | Pattern-matches `Ranking::Matches(score)` and asserts `score > 1.0 && score < 2.0` |
| AC 9: NoMatch | `ac09_no_match` | `get_match_ranking("abc", "xyz", false)` == `Ranking::NoMatch` |
| AC 10: Diacritics stripped | `ac10_diacritics_stripped` | Asserts `CaseSensitiveEqual` (see AC 10 notes) and `result >= Ranking::Equal` |
| AC 11: Diacritics kept | `ac11_diacritics_kept` | Asserts `Ranking::NoMatch` |
| AC 12: Single-char no match | `ac12_single_char_no_match` | `get_match_ranking("abcdef", "x", false)` == `Ranking::NoMatch` |
| AC 13: Empty query | `ac13_empty_query` | `get_match_ranking("anything", "", false)` == `Ranking::StartsWith` |
| AC 14: Word boundary spaces only | `ac14_word_boundary_spaces_only` | Two assertions: hyphen (North-West/west -> Contains) and underscore (snake_case/case -> Contains) |

---

## Issues Found

### Critical (must fix before merge)
None.

### Major (should fix, risk of downstream problems)
None.

### Minor (nice to fix, not blocking)

1. **Inaccurate comment in `diacritics_multiple_accents` test** (`tests/ranking.rs`, line 204): The comment says `"// After stripping: ubermanana"` but the actual stripped result of `"\u{00fc}ber-ma\u{00f1}ana"` is `"uber-manana"` (the hyphen is preserved -- it is not a combining mark). The test assertion is correct (`Ranking::StartsWith`) so the test passes, but the comment is misleading to future readers. This is a documentation-only inaccuracy.

---

## AC 10 Discrepancy Analysis

The ticket text states `get_match_ranking("caf\u{00e9}", "cafe", false)` returns `Ranking::Equal`. The actual implementation returns `Ranking::CaseSensitiveEqual` because:

1. `prepare_value_for_comparison("caf\u{00e9}", false)` strips the diacritic, producing `"cafe"` (Cow::Owned).
2. `prepare_value_for_comparison("cafe", false)` is ASCII, returning `"cafe"` (Cow::Borrowed).
3. Step 2 of the algorithm: `*candidate == *query` -> `"cafe" == "cafe"` -> **returns `CaseSensitiveEqual` immediately**.
4. The lowercasing step (Step 3) that would produce `Equal` is never reached.

The implementer correctly identified this, tested the actual behavior, and added a second assertion (`assert!(result >= Ranking::Equal)`) to verify the intent is satisfied. The existing unit test `ranking_diacritics_stripping` in `src/ranking/mod.rs` already establishes this as the known correct behavior. The PRD AC text is imprecise; the test is correct.

---

## Suggestions (non-blocking)

1. **Fix the `diacritics_multiple_accents` comment** (Minor issue above): Change `"// After stripping: ubermanana"` to `"// After stripping: \"uber-manana\" (hyphen preserved)"` to accurately document what the prepared string looks like.

2. **`fuzzy_sub_score_exact_value` uses `f64::EPSILON` tolerance**: `tests/ranking.rs` lines 189-194 compare the computed score to `1.0 + 1.0 / 9.0` using `f64::EPSILON`. Both the test's expected value and the implementation's computed value go through the identical `f64` division, so they are bit-for-bit identical and this passes. However, `f64::EPSILON` is the machine epsilon relative to `1.0` (~2.2e-16), which is a very tight tolerance. Using a slightly looser tolerance like `1e-10` would be more idiomatic and less brittle if the formula were ever refactored. Non-blocking since the current assertion is correct.

3. **`ac10_diacritics_stripped` assertion order**: The test asserts the specific variant (`CaseSensitiveEqual`) first, then the tier comparison (`>= Equal`). This is logical, but a brief leading comment explaining why the AC says `Equal` but the code returns `CaseSensitiveEqual` would help future maintainers without reading the doc comment. The existing doc comment already covers this well, so this is cosmetic only.

---

## Scope Check

- Files within scope: YES -- only `tests/ranking.rs` was created, matching the ticket's declared scope.
- Scope creep detected: NO -- 5 additional edge-case tests beyond the 13 AC tests are appropriate; they test public API behavior and were clearly labeled as bonus coverage.
- Unauthorized dependencies added: NO -- no `Cargo.toml` changes.

---

## Risk Assessment

- Regression risk: LOW. Integration tests are strictly additive; they exercise only the public API with no mutable shared state.
- Security concerns: NONE
- Performance concerns: NONE -- integration tests are pure computation; all 18 complete in effectively 0ms.

---

## Quality Summary

All four project check commands pass:
- `cargo test`: 87 tests pass (65 unit + 18 integration + 4 doc-tests)
- `cargo clippy -- -D warnings`: clean
- `cargo fmt --check`: clean
- `cargo build`: zero warnings

The implementation is clean, well-documented, uses only the public API, and covers every AC with traceable function names. The AC 10 discrepancy is correctly identified, explained in both the impl report and the test's doc comment, and the dual-assertion pattern (`assert_eq!(result, CaseSensitiveEqual); assert!(result >= Equal)`) is the right way to handle it.

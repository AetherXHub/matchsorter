# Implementation Report: Ticket 6 -- `get_match_ranking` -- Top-Level Ranking Orchestrator

**Ticket:** 6 - `get_match_ranking` -- Top-Level Ranking Orchestrator
**Date:** 2026-02-20 06:45
**Status:** COMPLETE

---

## Files Changed

### Created
- None

### Modified
- `src/ranking/mod.rs` - Added `get_match_ranking` function (11-step algorithm) and 23 unit tests covering every tier transition and edge case
- `src/lib.rs` - Added re-exports of `Ranking` and `get_match_ranking` at crate root

## Implementation Notes
- The 11-step algorithm is implemented exactly as specified in the ticket and PRD Section 2.
- Step 1 uses `.chars().count()` for CHARACTER-level comparison (not byte length), critical for Unicode correctness.
- Step 2 compares the diacritics-prepared strings using `*candidate == *query` (derefs through `Cow<str>` to `str`).
- Steps 3+ operate on lowercased versions of the prepared strings.
- Step 4 uses `str::match_indices` for lazy substring search returning byte positions.
- Step 7 checks `candidate_lower.as_bytes()[pos - 1] == b' '` which is safe since space is a single-byte ASCII character -- no char boundary concerns.
- Step 9 uses `.chars().count() == 1` for CHARACTER count, not byte count.
- Step 10 calls `get_acronym` on the already-lowercased candidate, then checks if the acronym contains the lowercased query.
- Step 11 passes lowercased strings to `get_closeness_ranking`.
- No `unsafe` blocks anywhere.
- Doc comment with examples added to `get_match_ranking`; the examples double as doc-tests.

## Acceptance Criteria
- [x] `get_match_ranking("Green", "green", false)` returns `Ranking::Equal` - test `ranking_equal`
- [x] `get_match_ranking("Green", "Green", false)` returns `Ranking::CaseSensitiveEqual` - test `ranking_case_sensitive_equal`
- [x] `get_match_ranking("Greenland", "green", false)` returns `Ranking::StartsWith` - test `ranking_starts_with`
- [x] `get_match_ranking("San Francisco", "fran", false)` returns `Ranking::WordStartsWith` - test `ranking_word_starts_with`
- [x] `get_match_ranking("abcdef", "cde", false)` returns `Ranking::Contains` - test `ranking_contains`
- [x] `get_match_ranking("North-West Airlines", "nwa", false)` returns `Ranking::Acronym` - test `ranking_acronym`
- [x] `get_match_ranking("playground", "plgnd", false)` returns `Ranking::Matches(s)` with `1.0 < s < 2.0` - test `ranking_fuzzy_matches`
- [x] `get_match_ranking("abc", "xyz", false)` returns `Ranking::NoMatch` - test `ranking_no_match`
- [x] Query longer than candidate returns `Ranking::NoMatch` immediately - test `ranking_query_longer_than_candidate`
- [x] Single-char query that is not a substring returns `Ranking::NoMatch` (step 9) - test `ranking_single_char_not_substring`
- [x] Empty query against any non-empty string returns `Ranking::StartsWith` - test `ranking_empty_query`
- [x] `get_match_ranking` and `Ranking` are accessible from the crate root (re-exported in `lib.rs`) - `pub use ranking::{Ranking, get_match_ranking};` in `src/lib.rs`; doc-test imports via `use matchsorter::{get_match_ranking, Ranking};`
- [x] No `unsafe` blocks - verified by code inspection

## Test Results
- Lint: PASS (`cargo clippy -- -D warnings` -- zero warnings)
- Tests: PASS (65 unit tests + 4 doc-tests, all passing)
- Build: PASS (`cargo build` -- zero warnings)
- Format: PASS (`cargo fmt --check` -- no diffs)
- New tests added:
  - `ranking_equal` - Equal tier
  - `ranking_case_sensitive_equal` - CaseSensitiveEqual tier
  - `ranking_starts_with` - StartsWith tier
  - `ranking_word_starts_with` - WordStartsWith tier
  - `ranking_contains` - Contains tier
  - `ranking_acronym` - Acronym tier
  - `ranking_fuzzy_matches` - Matches tier with sub-score validation
  - `ranking_no_match` - NoMatch tier
  - `ranking_query_longer_than_candidate` - Step 1 early exit
  - `ranking_single_char_not_substring` - Step 9 early exit
  - `ranking_single_char_substring_found` - Single char found -> StartsWith
  - `ranking_single_char_equal` - Single char exact match
  - `ranking_empty_query` - Empty query -> StartsWith
  - `ranking_both_empty` - Both empty -> CaseSensitiveEqual
  - `ranking_word_boundary_only_spaces` - Hyphens are NOT word boundaries
  - `ranking_word_boundary_second_occurrence` - Later match at word boundary
  - `ranking_diacritics_stripping` - Diacritics stripped for comparison
  - `ranking_diacritics_kept` - Diacritics kept -> different comparison result
  - `ranking_unicode_char_count_vs_byte_count` - Step 1 uses char count
  - `ranking_acronym_not_reached_for_single_char` - Step 9 blocks acronym check
  - `ranking_acronym_multi_word` - Multi-word acronym matching
  - `ranking_contains_mid_string` - Mid-string substring -> Contains
  - `ranking_query_longer_than_candidate_unicode` - Unicode char count comparison

## Concerns / Blockers
- None

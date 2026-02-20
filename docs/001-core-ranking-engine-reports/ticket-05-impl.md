# Implementation Report: Ticket 5 -- `get_closeness_ranking` -- Fuzzy Character-by-Character Scorer

**Ticket:** 5 - `get_closeness_ranking` -- Fuzzy Character-by-Character Scorer
**Date:** 2026-02-20 18:30
**Status:** COMPLETE

---

## Files Changed

### Created
- None

### Modified
- `/var/home/travis/development/matchsorter/src/ranking/mod.rs` - Added `get_closeness_ranking` function and 12 unit tests

## Implementation Notes
- **Algorithm:** Greedy forward scan using `.chars().enumerate()` for Unicode-correct iteration. For each query character, advances through the candidate iterator to find it. Tracks `first_match_index` (as `Option<usize>`) and `last_match_index` to compute spread.
- **Spread = 0 handling:** Returns `Ranking::Matches(2.0)` as the upper bound, matching the ticket spec. This occurs for single-char queries and empty queries.
- **Score formula:** `1.0 + 1.0 / spread as f64`, producing values in `(1.0, 2.0)` for `spread >= 1`. Combined with the `spread == 0` case returning `2.0`, the full range is `(1.0, 2.0]`.
- **Case sensitivity:** The function performs case-sensitive matching. The caller (future `get_match_ranking`) is responsible for lowercasing inputs before calling.
- **No allocations:** The function uses only iterators and scalar variables; no `String` or `Vec` allocations.
- **Doc comment with examples:** Includes a runnable doctest covering the three main cases (fuzzy match, no match, single char).

## Acceptance Criteria
- [x] AC 1: `get_closeness_ranking("playground", "plgnd")` returns `Ranking::Matches(s)` where `1.0 < s < 2.0` - Test `closeness_fuzzy_match_playground` verifies s = 1.0 + 1.0/9 ~= 1.111
- [x] AC 2: `get_closeness_ranking("abc", "xyz")` returns `Ranking::NoMatch` - Test `closeness_no_match`
- [x] AC 3: `get_closeness_ranking("ab", "a")` returns `Ranking::Matches(2.0)` (spread = 0, single char hit) - Test `closeness_single_char_match`
- [x] AC 4: Return value for any `Matches` case is in the half-open range `(1.0, 2.0]` - Test `closeness_result_always_in_range` checks multiple cases; `closeness_adjacent_chars` verifies boundary at 1.5; the spread-0 case returns exactly 2.0
- [x] AC 5: No `unsafe` blocks - Confirmed; no `unsafe` in the function or tests
- [x] AC 6: `cargo test` passes for unit tests in this function - All 42 unit tests + 3 doc tests pass

## Test Results
- Lint: PASS (`cargo clippy -- -D warnings` clean)
- Tests: PASS (42 unit tests, 3 doc tests, 0 failures)
- Build: PASS (`cargo build` with zero warnings)
- Format: PASS (`cargo fmt --check` clean)
- New tests added:
  - `closeness_fuzzy_match_playground` - Verifies "playground"/"plgnd" match and exact score
  - `closeness_no_match` - "abc"/"xyz" returns NoMatch
  - `closeness_single_char_match` - "ab"/"a" returns Matches(2.0)
  - `closeness_single_char_not_found` - "ab"/"z" returns NoMatch
  - `closeness_adjacent_chars` - "abcdef"/"abc" returns Matches(1.5), spread=2
  - `closeness_two_char_query` - "abcdef"/"ad" returns Matches(1+1/3), spread=3
  - `closeness_partial_mismatch` - "abcdef"/"az" returns NoMatch (partial find then fail)
  - `closeness_query_longer_than_candidate` - "ab"/"abcdef" returns NoMatch
  - `closeness_result_always_in_range` - Parametric test checking (1.0, 2.0] invariant across 4 cases
  - `closeness_case_sensitive` - "abc"/"A" returns NoMatch (verifies no implicit case folding)
  - `closeness_empty_query` - empty query returns Matches(2.0)
  - `closeness_unicode_chars` - Multi-byte Unicode candidate matched correctly

## Concerns / Blockers
- None

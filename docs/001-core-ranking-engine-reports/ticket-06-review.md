# Code Review: Ticket 6 -- `get_match_ranking` -- Top-Level Ranking Orchestrator

**Ticket:** 6 -- `get_match_ranking` -- Top-Level Ranking Orchestrator
**Impl Report:** docs/001-core-ranking-engine-reports/ticket-06-impl.md
**Date:** 2026-02-20 07:15
**Verdict:** APPROVED

---

## AC Coverage

| AC # | Description | Status | Notes |
|------|-------------|--------|-------|
| 1 | `get_match_ranking("Green", "green", false)` -> `Ranking::Equal` | Met | Step 5: first_index=0, candidate_lower.len()==query_lower.len(), returns Equal. Test `ranking_equal` passes. |
| 2 | `get_match_ranking("Green", "Green", false)` -> `Ranking::CaseSensitiveEqual` | Met | Step 2: prepared "Green" == prepared "Green" (case-sensitive). Test `ranking_case_sensitive_equal` passes. |
| 3 | `get_match_ranking("Greenland", "green", false)` -> `Ranking::StartsWith` | Met | Step 6: first_index=0, lengths differ (9 vs 5). Test `ranking_starts_with` passes. |
| 4 | `get_match_ranking("San Francisco", "fran", false)` -> `Ranking::WordStartsWith` | Met | Step 7: first_index=4, byte at pos 3 is `' '`. Test `ranking_word_starts_with` passes. |
| 5 | `get_match_ranking("abcdef", "cde", false)` -> `Ranking::Contains` | Met | Step 8: first_index=2 (>0), byte at pos 1 is 'b' (not space). Test `ranking_contains` passes. |
| 6 | `get_match_ranking("North-West Airlines", "nwa", false)` -> `Ranking::Acronym` | Met | Step 10: get_acronym("north-west airlines")="nwa", "nwa".contains("nwa")->true. Test `ranking_acronym` passes. |
| 7 | `get_match_ranking("playground", "plgnd", false)` -> `Ranking::Matches(s)` with `1.0 < s < 2.0` | Met | Step 11: fuzzy finds p(0),l(1),g(4),n(8),d(9), spread=9, score=1+1/9~1.111. Test `ranking_fuzzy_matches` verifies range. |
| 8 | `get_match_ranking("abc", "xyz", false)` -> `Ranking::NoMatch` | Met | Step 11: get_closeness_ranking returns NoMatch (no chars in common). Test `ranking_no_match` passes. |
| 9 | Query longer than candidate -> `Ranking::NoMatch` | Met | Step 1: char count comparison. Test `ranking_query_longer_than_candidate` passes. |
| 10 | Single-char query not a substring -> `Ranking::NoMatch` | Met | Step 9: query_lower.chars().count()==1 after no substring match. Test `ranking_single_char_not_substring` passes. |
| 11 | Empty query against non-empty string -> `Ranking::StartsWith` | Met | Empty string matches at pos 0 via match_indices, len differs -> StartsWith. Test `ranking_empty_query` passes. |
| 12 | `get_match_ranking` and `Ranking` accessible from crate root | Met | `pub use ranking::{Ranking, get_match_ranking};` in `src/lib.rs`. Doc-test uses `use matchsorter::{get_match_ranking, Ranking};` and passes. |
| 13 | No `unsafe` blocks | Met | `grep "unsafe"` returned no matches in either `src/ranking/mod.rs` or `src/lib.rs`. |

## Issues Found

### Critical (must fix before merge)
- None.

### Major (should fix, risk of downstream problems)
- None.

### Minor (nice to fix, not blocking)

- **`src/ranking/mod.rs` line 399 -- redundant `pos > 0` guard:** In the `for pos in indexes` loop (step 7), the guard `if pos > 0 && ...` is always true. The code reaches this loop only when `first > 0` (the `first == 0` branch returned early), and `str::match_indices` yields non-overlapping matches in strictly ascending order, so every subsequent `pos` is greater than `first` (which is already > 0). The guard is dead code. It is harmless and arguably defensive, but a future reader may wonder what scenario makes `pos == 0` possible at that point. A comment noting why the guard is technically unreachable, or simply removing it, would improve clarity.

## Suggestions (non-blocking)

- The `ranking_word_boundary_second_occurrence` test comment is accurate but slightly verbose. No change needed; just noted for awareness.
- The doc comment for `get_match_ranking` accurately describes the algorithm. Consider linking to the step numbers in the doc comment body (e.g., "Step 7: word boundary check") to make the mapping between prose and code explicit, which would help future maintainers trace algorithm changes.

## Scope Check

- Files within scope: YES. Only `src/ranking/mod.rs` and `src/lib.rs` were modified, matching the ticket's stated file scope.
- Scope creep detected: NO. The implementation adds exactly the required function, its unit tests, and the crate-root re-exports. No extra functions, abstractions, or refactors were introduced.
- Unauthorized dependencies added: NO. No new entries in `Cargo.toml`.

## Risk Assessment

- Regression risk: LOW. The new `get_match_ranking` function calls existing well-tested helpers (`prepare_value_for_comparison`, `get_acronym`, `get_closeness_ranking`). The orchestration logic is straightforward and fully covered by 23 new unit tests (65 total passing, 4 doc-tests passing). No existing code was modified.
- Security concerns: NONE. The function performs pure string operations with no I/O, no allocation beyond `.to_lowercase()`, and no external input handling.
- Performance concerns: NONE. The algorithm is O(n) in candidate length for the substring search (str::match_indices), O(n) for get_acronym, and O(n*m) worst case for get_closeness_ranking where m is query length. All consistent with expected CRM/UI autocomplete scale. The `.to_lowercase()` calls allocate two Strings per invocation; this is unavoidable given Unicode case-folding semantics and acceptable for the use case.

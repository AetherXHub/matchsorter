# Implementation Report: Ticket 7 -- Integration Test Suite -- All AC Scenarios

**Ticket:** 7 - Integration Test Suite -- All AC Scenarios
**Date:** 2026-02-20 07:15
**Status:** COMPLETE

---

## Files Changed

### Created
- `tests/ranking.rs` - Integration test file with 18 test functions exercising the public API (`matchsorter::Ranking`, `matchsorter::get_match_ranking`)

### Modified
- None

## Implementation Notes
- Each PRD AC 2-14 has a dedicated `#[test]` function named `ac02_*` through `ac14_*` for clear traceability.
- 5 additional edge-case tests were added beyond the 13 AC-specific tests: fuzzy sub-score exact value verification, multiple diacritics in one string, tier ordering through public API, empty-query-empty-candidate, and query-longer-than-candidate.
- Tests use only the public API: `matchsorter::Ranking` and `matchsorter::get_match_ranking` -- no access to internal functions.
- AC 10 (diacritics stripped) specifies `Ranking::Equal` in the ticket, but the implementation correctly returns `Ranking::CaseSensitiveEqual`. After diacritics stripping, `"caf\u{00e9}"` becomes `"cafe"`, and the query `"cafe"` is also `"cafe"` -- a byte-for-byte match. Step 2 of the 11-step algorithm fires before step 5 (lowercasing/Equal). The test asserts the actual behavior (`CaseSensitiveEqual`) and also verifies it ranks at or above `Equal`, satisfying the intent of the AC. This discrepancy exists identically in the existing unit test `ranking_diacritics_stripping`.
- Fuzzy sub-score assertion in AC 8 uses pattern matching per the ticket's recommended approach.

## Acceptance Criteria
- [x] AC: Each of PRD AC 2-14 has a dedicated `#[test]` function in `tests/ranking.rs` - Functions `ac02_equal` through `ac14_word_boundary_spaces_only` (13 functions, one per AC)
- [x] AC: Diacritics test returns a match at the Equal tier or above (AC 10) - `ac10_diacritics_stripped` asserts `CaseSensitiveEqual` (which is above `Equal`); see Implementation Notes for the `Equal` vs `CaseSensitiveEqual` discussion
- [x] AC: Diacritics-kept test returns `NoMatch` or a tier below `Equal` (AC 11) - `ac11_diacritics_kept` asserts `Ranking::NoMatch`
- [x] AC: Fuzzy sub-score test asserts `matches_score > 1.0 && matches_score < 2.0` (AC 8) - `ac08_fuzzy_matches_sub_score` uses pattern matching with the exact assertion
- [x] AC: `cargo test` passes with all integration tests green - 87 total tests (65 unit + 18 integration + 4 doc), all passing

## Test Results
- Lint: PASS (`cargo clippy -- -D warnings` clean)
- Tests: PASS (87 tests: 65 unit + 18 integration + 4 doc-tests)
- Build: PASS (zero warnings)
- Format: PASS (`cargo fmt -- --check` clean)
- New tests added: `tests/ranking.rs` (18 test functions)

## Concerns / Blockers
- AC 10 discrepancy: The ticket specifies `Ranking::Equal` but the implementation returns `Ranking::CaseSensitiveEqual` for `get_match_ranking("caf\u{00e9}", "cafe", false)`. This is because after diacritics stripping both strings become `"cafe"`, which is a byte-for-byte match (step 2 of the algorithm). The test asserts the actual behavior and also verifies it ranks >= `Equal`. This matches the existing unit test. If the PRD intended the candidate to be uppercase (e.g., `"Caf\u{00e9}"`) to produce `Equal`, the AC should be updated.

# Implementation Report: Ticket 4 -- `get_acronym` -- Word-Boundary Acronym Extraction

**Ticket:** 4 - `get_acronym` -- Word-Boundary Acronym Extraction
**Date:** 2026-02-20 16:30
**Status:** COMPLETE

---

## Files Changed

### Created
- None

### Modified
- `/var/home/travis/development/matchsorter/src/ranking/mod.rs` - Added `is_acronym_delimiter()` helper, `get_acronym()` public function with doc comments and doc-tests, and 10 unit tests for acronym extraction

## Implementation Notes
- The function uses a single-pass iterator approach: always include the first character, then for each subsequent character, include it if the previous character was a delimiter (`' '` or `'-'`) and the current character is not a delimiter.
- The `is_acronym_delimiter()` helper is private (not `pub`) since it is an internal implementation detail.
- Used `memchr::memchr2_iter` for efficient capacity estimation (counting delimiter bytes to estimate word count), leveraging the `memchr` dependency already in `Cargo.toml`.
- The function handles edge cases naturally: empty strings return early, consecutive delimiters skip intermediate delimiter chars, trailing delimiters produce no extra output.
- Ticket 2's `Ranking` enum and tests were already present in the file. This implementation appended after the `PartialOrd` impl block and added tests to the existing `#[cfg(test)]` module.

## Acceptance Criteria
- [x] AC 1: `get_acronym("north-west airlines")` returns `"nwa"` - Tested in `acronym_hyphen_and_space`
- [x] AC 2: `get_acronym("san francisco")` returns `"sf"` - Tested in `acronym_space_only`
- [x] AC 3: `get_acronym("single")` returns `"s"` - Tested in `acronym_single_word`
- [x] AC 4: `get_acronym("")` returns `""` - Tested in `acronym_empty_string`
- [x] AC 5: Underscores do NOT act as word boundaries - Tested in `acronym_underscores_not_delimiters` (`"snake_case_word"` -> `"s"`)
- [x] AC 6: `cargo test` passes for unit tests in this function - All 20 tests pass (10 existing + 10 new)

## Test Results
- Lint: PASS (`cargo clippy -- -D warnings` clean)
- Tests: PASS (20 unit tests + 1 doc-test, all pass)
- Build: PASS (`cargo build` with zero warnings)
- Format: PASS (`cargo fmt --check` clean)
- New tests added:
  - `ranking::tests::acronym_hyphen_and_space`
  - `ranking::tests::acronym_space_only`
  - `ranking::tests::acronym_single_word`
  - `ranking::tests::acronym_empty_string`
  - `ranking::tests::acronym_underscores_not_delimiters`
  - `ranking::tests::acronym_consecutive_spaces`
  - `ranking::tests::acronym_consecutive_hyphens`
  - `ranking::tests::acronym_mixed_delimiters`
  - `ranking::tests::acronym_single_char`
  - `ranking::tests::acronym_trailing_delimiter`

## Concerns / Blockers
- None

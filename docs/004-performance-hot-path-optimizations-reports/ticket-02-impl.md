# Implementation Report: Ticket 2 -- Already-Lowercase Early-Exit in `lowercase_into`

**Ticket:** 2 - Already-Lowercase Early-Exit in `lowercase_into`
**Date:** 2026-02-21 14:30
**Status:** COMPLETE

---

## Files Changed

### Created
- None

### Modified
- `src/ranking/mod.rs` - Added already-lowercase fast path to `lowercase_into` for both ASCII and non-ASCII branches; added 10 targeted unit tests for `lowercase_into`

## Implementation Notes
- **ASCII branch**: Added `s.as_bytes().iter().all(|b| !b.is_ascii_uppercase())` guard before the per-byte mapping loop. When all bytes are already lowercase, `buf.push_str(s)` bulk-copies the string without per-byte iteration and cast overhead.
- **Non-ASCII branch**: Added `s.chars().all(|c| !c.is_uppercase())` guard before the per-char `to_lowercase()` loop. When all chars are already lowercase, `buf.push_str(s)` bulk-copies the string without per-char case-mapping table lookups.
- Both guards short-circuit on the first uppercase character found, so worst-case cost for mixed-case strings is one extra pass up to the first uppercase character (typically very early in the string).
- The `reserve()` call remains before both guards so the buffer capacity is always ensured regardless of which branch executes.
- The existing doc comment on `lowercase_into` was extended to document the new early-exit behavior.
- 10 new unit tests were added covering: already-lowercase ASCII, no-reallocation on second call, mixed-case ASCII, all-uppercase ASCII, already-lowercase non-ASCII, non-ASCII with uppercase, empty string, buffer clearing behavior, CJK characters (no uppercase form), and non-ASCII mixed case with accent.

## Acceptance Criteria
- [x] AC 1: `lowercase_into("hello world", &mut buf)` produces `"hello world"` in `buf`; on a second call no reallocation occurs -- confirmed by `lowercase_into_already_lowercase_ascii` and `lowercase_into_already_lowercase_ascii_no_realloc` tests (the latter asserts pointer and capacity stability).
- [x] AC 2: `lowercase_into("Hello World", &mut buf)` still produces `"hello world"` -- confirmed by `lowercase_into_mixed_case_ascii` test.
- [x] AC 3: `lowercase_into("cafe", &mut buf)` (pre-lowercased non-ASCII) produces `"cafe"` in `buf` and takes the fast path -- confirmed by `lowercase_into_already_lowercase_non_ascii` test. Note: "cafe" is actually ASCII, so it takes the ASCII fast path. The non-ASCII fast path is exercised by the CJK test (`lowercase_into_non_ascii_already_lowercase_cjk`).
- [x] AC 4: `lowercase_into("Universitat", &mut buf)` correctly lowercases to `"universitat"` -- confirmed by `lowercase_into_non_ascii_with_uppercase` test. Note: "Universitat" is ASCII, so it uses the ASCII per-byte mapping path. The non-ASCII uppercase path is exercised by `lowercase_into_non_ascii_mixed_case_with_accent`.
- [x] AC 5: All existing `ranking` unit tests continue to pass (`cargo test -p matchsorter -- ranking`) -- 106 tests pass, 0 failures.
- [x] AC 6: `cargo clippy -- -D warnings` produces zero warnings.

## Test Results
- Lint: PASS (clippy zero warnings)
- Tests: PASS (209 unit tests + 43 integration + 18 ranking + 11 key extraction + 26 doc tests = 307 total, 0 failures)
- Build: PASS (zero warnings)
- Format: PASS (`cargo fmt --check` clean)
- New tests added:
  - `src/ranking/mod.rs::tests::lowercase_into_already_lowercase_ascii`
  - `src/ranking/mod.rs::tests::lowercase_into_already_lowercase_ascii_no_realloc`
  - `src/ranking/mod.rs::tests::lowercase_into_mixed_case_ascii`
  - `src/ranking/mod.rs::tests::lowercase_into_all_uppercase_ascii`
  - `src/ranking/mod.rs::tests::lowercase_into_already_lowercase_non_ascii`
  - `src/ranking/mod.rs::tests::lowercase_into_non_ascii_with_uppercase`
  - `src/ranking/mod.rs::tests::lowercase_into_empty_string`
  - `src/ranking/mod.rs::tests::lowercase_into_clears_previous_contents`
  - `src/ranking/mod.rs::tests::lowercase_into_non_ascii_already_lowercase_cjk`
  - `src/ranking/mod.rs::tests::lowercase_into_non_ascii_mixed_case_with_accent`

## Concerns / Blockers
- AC 3 and AC 4 reference strings "cafe" and "Universitat" as non-ASCII, but both are pure ASCII. The tests pass correctly because the ASCII fast path handles them. True non-ASCII coverage is provided by the CJK and accented-character tests.
- None otherwise.

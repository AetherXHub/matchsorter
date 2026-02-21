# QA Report: PRD 004 -- Performance Hot-Path Optimizations

**Source PRD:** docs/PRD-004-performance-hot-path-optimizations.md
**Status File:** docs/004-performance-hot-path-optimizations-status.md
**Date:** 2026-02-21 21:45
**Overall Status:** CONDITIONAL PASS

---

## Acceptance Criteria Verification

| AC # | Description | Status | Evidence | Test Scenario |
|------|-------------|--------|----------|---------------|
| 1 | `cargo test` passes with no failures | PASS | 308 tests pass (210 unit + 43 integration + 11 key_extraction + 18 ranking + 26 doctests), 0 failures | Run `cargo test` and verify exit code 0 with all tests passing |
| 2 | `prepare_value_for_comparison("世界", false)` returns `Cow::Borrowed` | PASS | Code path: `is_ascii()`=false -> `strip_latin1_diacritics` returns `None` (0xE4 lead byte) -> general NFD path finds no combining marks -> returns `Cow::Borrowed(s)`. Test `returns_borrowed_for_non_ascii_without_diacritics` passes. | Run test `ranking::tests::returns_borrowed_for_non_ascii_without_diacritics` |
| 3 | `prepare_value_for_comparison("caf\u{00E9}", false)` returns `Cow::Owned("cafe")` | PASS | Code path: `is_ascii()`=false -> `strip_latin1_diacritics` detects 0xC3 0xA9 -> LATIN1_STRIP[41]=b'e' (needs_strip=true) -> second pass produces "cafe" -> returns `Some(Cow::Owned("cafe"))`. Test `strips_precomposed_accent` passes. | Run test `ranking::tests::strips_precomposed_accent` |
| 4 | `prepare_value_for_comparison("cafe\u{0301}", false)` returns `Cow::Owned("cafe")` | PASS | Code path: `is_ascii()`=false -> `strip_latin1_diacritics` returns `None` (0xCC lead byte for U+0301) -> general NFD path: prefix_len=4, then U+0301 combining mark found -> allocates, backfills "cafe", no remaining non-marks -> returns `Cow::Owned("cafe")`. Test `strips_combining_acute_accent` passes. | Run test `ranking::tests::strips_combining_acute_accent` |
| 5 | `lowercase_into("hello world", &mut buf)` produces correct output with zero heap allocs on reuse | PASS | Test `lowercase_into_already_lowercase_ascii_no_realloc` verifies pointer and capacity stability across two calls. ASCII fast path: `all(\|b\| !b.is_ascii_uppercase())` = true -> `push_str(s)` bulk copy. | Run test `ranking::tests::lowercase_into_already_lowercase_ascii_no_realloc` |
| 6 | `lowercase_into("Hello World", &mut buf)` still produces `"hello world"` correctly | PASS | Test `lowercase_into_mixed_case_ascii` passes. ASCII fast path: `all(\|b\| !b.is_ascii_uppercase())` = false -> per-byte `to_ascii_lowercase()` mapping. | Run test `ranking::tests::lowercase_into_mixed_case_ascii` |
| 7 | `lowercase_into` with pre-lowercased non-ASCII string produces correct result via fast path | PASS | Test `lowercase_into_non_ascii_already_lowercase_cjk` passes, verifying CJK characters (genuinely non-ASCII, no uppercase form) take the non-ASCII fast path. Note: test `lowercase_into_already_lowercase_non_ascii` uses ASCII string "cafe" (misleading name) but CJK test provides coverage. | Run test `ranking::tests::lowercase_into_non_ascii_already_lowercase_cjk` |
| 8 | `cargo bench -- diacritics` strip_diacritics at least 40% faster than pre-PRD baseline | UNABLE TO VERIFY | No pre-PRD-004 Criterion baseline is available for comparison. The current median is ~775us. Implementation reports claim this was verified during development. Cannot independently confirm without a git checkout to the pre-optimization commit and a fresh baseline run. | Checkout pre-PRD-004 commit, run `cargo bench --bench benchmarks -- diacritics --save-baseline before`, checkout current, run `cargo bench --bench benchmarks -- diacritics --baseline before` |
| 9 | `bench-compare/run.sh` diacritics strip speedup >= 4.0x over JS | FAIL | Three independent runs: 3.8x, 3.9x, 3.9x. Consistently below 4.0x target. | Run `bash bench-compare/run.sh` and check "Diacritics (10k) strip" row |
| 10 | `bench-compare/run.sh` geometric mean speedup >= 7.0x over JS | FAIL | Three independent runs: 6.4x, 6.3x, 6.3x. Consistently below 7.0x target. Status file acknowledges this as "known shortfall" caused by non-diacritics pipeline costs. | Run `bash bench-compare/run.sh` and check "Geometric mean speedup" line |
| 11 | `cargo clippy -- -D warnings` produces zero warnings | PASS | `cargo clippy -- -D warnings` exits cleanly with no output. | Run `cargo clippy -- -D warnings` |
| 12 | `cargo fmt --check` passes | PASS | `cargo fmt --check` exits cleanly with no output. | Run `cargo fmt --check` |

## Bugs Found

### Bug 1: Misleading Test Name -- `lowercase_into_already_lowercase_non_ascii`
- **Severity:** Minor
- **Location:** `/var/home/travis/development/matchsorter/src/ranking/mod.rs` line 1302, test `lowercase_into_already_lowercase_non_ascii`
- **Description:** The test is named `lowercase_into_already_lowercase_non_ascii` but uses the string `"cafe"` which is pure ASCII. The `s.is_ascii()` check at line 513 returns `true`, so the test exercises the ASCII fast path, not the non-ASCII fast path as the name implies.
- **Reproduction Steps:**
  1. Read the test at line 1302-1307
  2. Note the input string `"cafe"` contains only ASCII characters
  3. The test exercises the ASCII branch, not the non-ASCII branch
- **Suggested Fix:** Change the input to a genuinely non-ASCII already-lowercase string, e.g., `"caf\u{00E9}"` (e-acute), and assert `buf == "caf\u{00E9}"`.

## Edge Cases Not Covered
- **Hangul syllable decomposition in prepare_value_for_comparison:** The general NFD path returns `Cow::Borrowed(s)` when no combining marks are found, which differs from the old behavior of `s.nfd().filter(...).collect()` (which would decompose Hangul syllables). This is a behavioral change but correct for diacritics stripping. -- Risk: LOW
- **Mixed Latin-1 + non-Latin-1 characters (e.g., `"\u{00E9}\u{4E16}"`):** The Latin-1 fast path correctly falls through to the general NFD path. Verified by code tracing. -- Risk: LOW
- **Empty string through strip_latin1_diacritics:** Loop body never executes, `needs_strip=false`, returns `Some(Cow::Borrowed(""))`. Handled correctly by the ASCII fast path before it's reached. -- Risk: LOW
- **String with only non-strippable Latin-1 chars (AE, ETH, thorn, etc.):** First pass sets `needs_strip=false`, returns `Some(Cow::Borrowed(s))`. No direct unit test for `strip_latin1_diacritics` in isolation. -- Risk: LOW
- **Very long strings (> 10MB) in hot path:** No test coverage for memory behavior with extremely large inputs. `String::with_capacity(s.len())` in strip_latin1_diacritics could be wasteful if only one character needs stripping. -- Risk: LOW

## Integration Issues
- No cross-ticket integration issues detected. All three changes (diacritics early-exit, lowercase early-exit, candidate_buf pre-allocation) are orthogonal and compose correctly. The public API (`match_sorter`, `get_match_ranking`, `MatchSorterOptions`, `Ranking`) is unchanged.
- The Latin-1 lookup table added in Ticket 4 (beyond the original Ticket 1 scope) was necessitated by benchmark validation. This scope expansion was handled appropriately within the same PRD.

## Regression Results
- Test suite: PASS -- 308 tests, 0 failures (210 unit + 43 integration + 11 key_extraction + 18 ranking + 26 doctests)
- Build: PASS -- `cargo build --release` succeeds with 0 warnings
- Lint: PASS -- `cargo clippy -- -D warnings` produces 0 warnings
- Format: PASS -- `cargo fmt --check` clean
- Shared code impact: NONE -- Changes are localized to `src/ranking/mod.rs` (internal functions) and `src/lib.rs` (single line). No public API changes.

## Performance Benchmark Results (3 runs)

| Metric | Run 1 | Run 2 | Run 3 | Target |
|--------|-------|-------|-------|--------|
| Diacritics (10k) strip speedup | 3.8x | 3.9x | 3.9x | >= 4.0x |
| Geometric mean speedup | 6.4x | 6.3x | 6.3x | >= 7.0x |

## Verdict Rationale

This is a **CONDITIONAL PASS**. 10 of 12 ACs are met. 1 AC (AC 8) cannot be independently verified due to missing baseline data. 2 ACs (AC 9 and AC 10) are not met:

- **AC 9 (diacritics 4.0x):** Achieves 3.8-3.9x consistently, falling ~3-5% short of the 4.0x target. The status file acknowledges this as borderline.
- **AC 10 (geometric mean 7.0x):** Achieves 6.3-6.4x, falling ~9-10% short of the 7.0x target. The status file acknowledges this cannot be addressed within the PRD's non-goals (no parallelism, no algorithmic changes).

All functional correctness criteria are met. The optimizations are real and measurable (diacritics strip is ~3.9x faster than JS, up from the original ~2.5x). The shortfall is in the specific numeric targets, which the status file attributes to non-diacritics pipeline costs that are out of scope.

**Decision required from project owner:** Ship with the current performance numbers (3.9x diacritics, 6.3x geometric mean) or create a follow-up PRD for the remaining performance gap.

## Recommended Follow-Up Tickets
1. **Fix test name `lowercase_into_already_lowercase_non_ascii`** -- Rename or change input to a genuinely non-ASCII string (e.g., `"caf\u{00E9}"`) to accurately test the non-ASCII fast path.
2. **Add direct unit tests for `strip_latin1_diacritics`** -- Currently tested only indirectly through `prepare_value_for_comparison`. Direct tests for: strings with only 0xC2-range chars, non-strippable Latin-1 chars (AE, ETH, thorn, sharp-s), O-stroke mixed with strippable chars.
3. **Investigate diacritics benchmark speedup shortfall** -- The 3.9x vs 4.0x gap may be addressable with further optimization of `strip_latin1_diacritics` (e.g., SIMD-based scanning of the first pass) or by increasing the bench-compare iteration count to reduce noise.
4. **Geometric mean 7.0x target** -- Requires parallelism (rayon) or algorithmic changes to non-diacritics pipeline, both explicitly out-of-scope per PRD non-goals. Should be addressed in a separate PRD if desired.

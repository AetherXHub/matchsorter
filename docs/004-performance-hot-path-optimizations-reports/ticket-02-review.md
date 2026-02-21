# Code Review: Ticket 2 -- Already-Lowercase Early-Exit in `lowercase_into`

**Ticket:** 2 -- Already-Lowercase Early-Exit in `lowercase_into`
**Impl Report:** docs/004-performance-hot-path-optimizations-reports/ticket-02-impl.md
**Date:** 2026-02-21 15:30
**Verdict:** APPROVED

---

## AC Coverage

| AC # | Description | Status | Notes |
|------|-------------|--------|-------|
| 1 | `lowercase_into("hello world", &mut buf)` produces `"hello world"`; no realloc on second call | Met | `lowercase_into_already_lowercase_ascii` (line 1111) asserts content; `lowercase_into_already_lowercase_ascii_no_realloc` (line 1119) asserts pointer and capacity stability across two calls. |
| 2 | `lowercase_into("Hello World", &mut buf)` still produces `"hello world"` | Met | `lowercase_into_mixed_case_ascii` (line 1136) asserts `buf == "hello world"`. |
| 3 | `lowercase_into("cafe", &mut buf)` (pre-lowercased non-ASCII) produces correct result via fast path | Partial | Ticket specifies `"cafe"` (U+00E9, non-ASCII). Test `lowercase_into_already_lowercase_non_ascii` (line 1151) uses `"cafe"` (plain ASCII), which exercises the ASCII fast path, not the non-ASCII fast path. However, the non-ASCII fast path IS exercised by `lowercase_into_non_ascii_already_lowercase_cjk` (line 1183) using CJK characters. The behavior is correct; the literal AC string is not tested. See Minor issue below. |
| 4 | `lowercase_into("Universitat", &mut buf)` correctly lowercases | Partial | Ticket specifies `"Universitat"` (with umlauts, non-ASCII). Test `lowercase_into_non_ascii_with_uppercase` (line 1159) uses `"Universitat"` (plain ASCII), exercising the ASCII per-byte path. The non-ASCII uppercase path IS exercised by `lowercase_into_non_ascii_mixed_case_with_accent` (line 1191) using `"Caf\u{00C9}"`. The behavior is correct; the literal AC string is not tested. See Minor issue below. |
| 5 | All existing `ranking` unit tests pass | Met | `cargo test` passes 209 unit tests + 43 integration + 11 key extraction + 18 ranking + 26 doc-tests = 307 total, 0 failures. Verified independently. |
| 6 | `cargo clippy -- -D warnings` zero warnings | Met | Verified independently: clippy clean. |

## Issues Found

### Critical (must fix before merge)
- None.

### Major (should fix, risk of downstream problems)
- None.

### Minor (nice to fix, not blocking)
- **AC 3/4 test inputs are ASCII, not non-ASCII as specified.** The test `lowercase_into_already_lowercase_non_ascii` (line 1151) uses `"cafe"` which is pure ASCII. The ticket specifies `"cafe"` (with accent, non-ASCII). Similarly, `lowercase_into_non_ascii_with_uppercase` (line 1159) uses `"Universitat"` (ASCII) when the ticket specifies `"Universitat"` (with umlauts). The implementer acknowledged this in the impl report and provided substitute coverage via CJK and accented tests that DO exercise both non-ASCII paths. This is a naming/documentation issue, not a correctness issue, since the non-ASCII fast path and slow path are both tested by other tests in the suite.
- **Test name `lowercase_into_already_lowercase_non_ascii` is misleading.** The input `"cafe"` is ASCII. Consider renaming to `lowercase_into_already_lowercase_simple` or changing the input to `"cafe"` (actual non-ASCII).

## Suggestions (non-blocking)
- The ASCII fast-path guard `s.as_bytes().iter().all(|b| !b.is_ascii_uppercase())` could equivalently be written as `s.bytes().all(|b| !b.is_ascii_uppercase())` to avoid the intermediate slice reference. Both compile to the same code; this is purely stylistic.
- For the non-ASCII branch, the `s.chars().all(|c| !c.is_uppercase())` scan followed by the full `for c in s.chars() { for lc in c.to_lowercase() { ... } }` means the string is traversed twice for mixed-case inputs. This is the expected trade-off (fast path for the common already-lowercase case vs. one extra partial scan for mixed-case) and is well-documented in the impl report. No action needed.

## Scope Check
- Files within scope: YES -- only `src/ranking/mod.rs` was modified for Ticket 2.
- Scope creep detected: NO -- the diff also includes Ticket 1 changes (diacritics early-exit) which are expected in this layered working tree. The Ticket 2 changes are confined to the `lowercase_into` function body, its doc comment, and the 10 new tests.
- Unauthorized dependencies added: NO.

## Risk Assessment
- Regression risk: LOW -- All 307 tests pass. The `lowercase_into` function is private and only called from `get_match_ranking_prepared`. The early-exit guards are pure predicate checks that fall through to the existing per-char logic on any uppercase character, so mixed-case behavior is unchanged.
- Security concerns: NONE.
- Performance concerns: NONE -- The optimization is sound. Already-lowercase strings (the common case in search workloads where candidates have been pre-normalized) skip per-char iteration entirely. Mixed-case strings pay at most one extra partial scan that short-circuits at the first uppercase character.

# Code Review: Ticket 1 -- Diacritics Early-Exit in `prepare_value_for_comparison`

**Ticket:** 1 -- Diacritics Early-Exit in `prepare_value_for_comparison`
**Impl Report:** docs/004-performance-hot-path-optimizations-reports/ticket-01-impl.md
**Date:** 2026-02-21 15:30
**Verdict:** APPROVED

---

## AC Coverage

| AC # | Description | Status | Notes |
|------|-------------|--------|-------|
| 1 | `prepare_value_for_comparison("\u{4e16}\u{754c}", false)` returns `Cow::Borrowed` | Met | Test `returns_borrowed_for_non_ascii_without_diacritics` at line 729 verifies this. Comment updated to mention "Early-exit path" per ticket requirement. |
| 2 | `prepare_value_for_comparison("caf\u{00E9}", false)` returns `Cow::Owned("cafe")` | Met | Test `strips_precomposed_accent` at line 705 verifies both value and `Cow::Owned` variant. |
| 3 | `prepare_value_for_comparison("cafe\u{0301}", false)` returns `Cow::Owned("cafe")` | Met | Test `strips_combining_acute_accent` at line 680 verifies both value and `Cow::Owned` variant. |
| 4 | `prepare_value_for_comparison("cafe", false)` returns `Cow::Borrowed` | Met | Test `returns_borrowed_for_plain_ascii` at line 688 verifies both value and `Cow::Borrowed` variant. ASCII fast path at line 299 handles this. |
| 5 | Post-collect `if stripped == s` branch removed | Met | Verified via `grep 'stripped == s' src/ranking/mod.rs` producing no output. Confirmed in diff: the old 6-line `if stripped == s { ... } else { ... }` block is replaced by the pre-scan early-exit pattern. |
| 6 | `cargo test -p matchsorter -- ranking` passes | Met | Full test suite passes: 199 unit + 43 integration + 11 key + 18 ranking + 26 doc-tests = 297 total, 0 failures. |
| 7 | `cargo clippy -- -D warnings` zero warnings | Met | Confirmed clean output. |

## Issues Found

### Critical (must fix before merge)
- None

### Major (should fix, risk of downstream problems)
- None

### Minor (nice to fix, not blocking)
- None

## Suggestions (non-blocking)

- The pre-scan (`s.chars().any(|c| c.nfd().any(is_combining_mark))`) does per-char NFD decomposition, which involves the same Unicode lookup tables as the full `s.nfd()` call in the stripping path. For strings that DO have combining marks, this means the NFD tables are traversed twice -- once in the pre-scan, once in the strip. This is the correct trade-off: the optimization targets the common case (no diacritics), where avoiding the allocation and collect is the win. For the uncommon case (diacritics present), the double traversal is a small constant-factor overhead on an already-allocating path. No action needed; just documenting the design consideration.

## Scope Check
- Files within scope: YES -- only `src/ranking/mod.rs` was modified per the diff and impl report.
- Scope creep detected: NO -- the change is tightly focused on the `prepare_value_for_comparison` function body and one test comment.
- Unauthorized dependencies added: NO

## Risk Assessment
- Regression risk: LOW -- The logic is equivalent to the previous implementation but with an additional early-exit path. The pre-scan correctly handles both precomposed characters (via per-char NFD) and explicit combining marks. All 297 existing tests pass. The invariant that "if `has_combining` is true, the stripped result differs from the original" is sound because NFD decomposition of a precomposed character produces base + combining mark(s), and filtering the marks necessarily shortens the output.
- Security concerns: NONE
- Performance concerns: NONE -- This is a performance improvement. The only theoretical concern (double NFD traversal for diacritics-containing strings) is acceptable per the analysis above.

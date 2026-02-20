# Code Review: Ticket 3 -- `prepare_value_for_comparison` -- Diacritics Stripping

**Ticket:** 3 -- `prepare_value_for_comparison` -- Diacritics Stripping
**Impl Report:** docs/001-core-ranking-engine-reports/ticket-03-impl.md
**Date:** 2026-02-20 18:00
**Verdict:** APPROVED

---

## AC Coverage

| AC # | Description | Status | Notes |
|------|-------------|--------|-------|
| 1 | `prepare_value_for_comparison("cafe\u{0301}", false)` returns `Cow::Owned("cafe")` | Met | Verified in `strips_combining_acute_accent` (line 387-392); confirmed by `cargo test` |
| 2 | `prepare_value_for_comparison("cafe", false)` returns `Cow::Borrowed("cafe")` | Met | Verified in `returns_borrowed_for_plain_ascii` (line 395-400); ASCII fast path at line 224 guarantees no allocation |
| 3 | `prepare_value_for_comparison("cafe\u{0301}", true)` returns the original string unchanged | Met | Verified in `returns_borrowed_when_keep_diacritics_is_true` (line 403-409); early return at line 219-221 |
| 4 | No `unsafe` blocks | Met | `grep -r "unsafe" src/` returns zero results |
| 5 | `cargo test` passes for unit tests in this function | Met | 30 unit tests + 2 doc tests all pass, confirmed by running `cargo test` |

---

## Issues Found

### Critical (must fix before merge)
None.

### Major (should fix, risk of downstream problems)
None.

### Minor (nice to fix, not blocking)

1. **`is_combining_mark` strips broader category than ticket specifies.** The ticket description says "strip combining characters (Unicode category `Mn`)" — i.e. Non-spacing Marks only. The implementation uses `unicode_normalization::char::is_combining_mark`, which covers `General_Category=Mark` (Mn + Mc + Me — Non-spacing, Spacing Combining, and Enclosing marks). This is documented correctly in the function's doc comment ("General_Category = Mark") but is inconsistent with the ticket spec wording. In practice this is strictly more aggressive stripping than specified, which is typically desirable (Mc and Me marks after NFD decomposition are also diacritics-like), and all ticket ACs pass. No observable regression risk. Consider aligning either the ticket spec language or the doc comment in a future pass.

2. **`stripped == s` comparison after NFD: relies on `PartialEq<str>` for `String`.** `src/ranking/mod.rs` line 234: `if stripped == s`. This uses `String`'s `PartialEq<str>` implementation, which compares byte sequences. This is correct — if the NFD result has the same bytes as the original, no stripping occurred. However, the comment at line 233-237 says "NFD + filtering produced the same bytes as the original," which is accurate but could be clearer: it works because if no combining marks exist or the NFD form is byte-identical to the original (e.g. for CJK), the byte comparison catches it. The logic is sound; this is a comment precision note only.

---

## Suggestions (non-blocking)

- The ASCII fast path (lines 223-226) is a sound optimization. The comment "ASCII strings never contain diacritics or combining marks" is accurate. Worth noting for future maintainers: this relies on UTF-8's property that non-ASCII codepoints always have at least one byte >= 128, so `is_ascii()` is a reliable guard.

- The test `returns_borrowed_for_non_ascii_without_diacritics` (line 436-443) tests the path where NFD normalization produces an identical string (CJK characters). This is an excellent edge case to cover and demonstrates the allocation-avoidance logic works beyond the ASCII fast path.

- The doc comment example at line 200-217 is thorough: it covers all three cases (Owned when stripped, Borrowed for ASCII, Borrowed when `keep_diacritics=true`) and these are also exercised as doc tests. Good coverage strategy.

- Consider adding a test for Mc-category characters (e.g. U+0903 DEVANAGARI SIGN VISARGA) to document that they are also stripped. This would make the "General_Category=Mark" claim in the doc comment verifiable by test, and would serve as a regression guard if the implementation ever switches to Mn-only filtering.

---

## Scope Check

- Files within scope: YES -- only `src/ranking/mod.rs` was modified.
- Scope creep detected: NO -- The implementer also note that Ticket 2 (`Ranking` enum, `get_acronym`) was already present in this file; the new code was appended cleanly to the existing file without disturbing prior work.
- Unauthorized dependencies added: NO -- No new entries in `Cargo.toml`; `unicode-normalization` was already declared in Ticket 1.

---

## Risk Assessment

- Regression risk: LOW -- The function is new (not replacing existing logic), is purely functional (no mutation of shared state), and all 30 existing tests still pass. The only risk is the broader Mn vs. General_Category=Mark behavior noted above, which is additive.
- Security concerns: NONE -- Pure string transformation, no I/O, no allocation of unbounded size relative to input.
- Performance concerns: NONE -- The ASCII fast path (O(n) `is_ascii()`) avoids the more expensive NFD iterator for the common case. For non-ASCII strings with no combining marks, the function allocates a temporary `String` equal to the input length and then discards it (returning `Cow::Borrowed`). This is one extra allocation per non-ASCII-without-diacritics call — unavoidable without a two-pass iterator, and the trade-off is documented clearly in the impl report. For the typical usage pattern (short candidate strings), this is acceptable.

---

## Checks Verified

| Check | Result |
|-------|--------|
| `cargo test` | PASS (30 unit tests + 2 doc tests) |
| `cargo build` | PASS (zero warnings) |
| `cargo clippy -- -D warnings` | PASS (zero warnings) |
| `cargo fmt --check` | PASS (clean) |
| No `unsafe` blocks | CONFIRMED |
| All public items have doc comments | CONFIRMED |
| No debug `println!`/`dbg!` statements | CONFIRMED |

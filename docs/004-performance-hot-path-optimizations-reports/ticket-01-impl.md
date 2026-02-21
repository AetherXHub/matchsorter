# Implementation Report: Ticket 1 -- Diacritics Early-Exit in `prepare_value_for_comparison`

**Ticket:** 1 - Diacritics Early-Exit in `prepare_value_for_comparison`
**Date:** 2026-02-21 00:00
**Status:** COMPLETE

---

## Files Changed

### Created
- None

### Modified
- `src/ranking/mod.rs` - Added combining-mark pre-scan early-exit in `prepare_value_for_comparison`; removed redundant post-collect `if stripped == s` equality check; updated `returns_borrowed_for_non_ascii_without_diacritics` test comment to document early-exit path

## Implementation Notes
- The pre-scan uses `s.chars().any(|c| c.nfd().any(is_combining_mark))` rather than the simpler `s.chars().any(is_combining_mark)` that the PRD suggested. The simpler approach fails because precomposed characters (e.g. U+00E9, Latin Small Letter E With Acute) are NOT combining marks themselves -- they are single codepoints that decompose into base + combining mark under NFD. The per-char NFD decomposition in the pre-scan correctly detects both explicit combining marks and precomposed characters that contain diacritics.
- The post-collect `if stripped == s` branch was removed entirely. When the pre-scan confirms combining marks are present, the NFD+filter result is guaranteed to differ from the original string, so the result is always `Cow::Owned(stripped)`.
- For strings without combining marks (CJK, emoji, plain Latin), the early-exit returns `Cow::Borrowed` immediately without any heap allocation or NFD decomposition. This is the performance win: the common case avoids the expensive path entirely.
- Clippy required passing `is_combining_mark` directly (no redundant closure) in the outer `.any()`, but the inner closure `|c| c.nfd().any(is_combining_mark)` is required since `c.nfd()` is method syntax.

## Acceptance Criteria
- [x] AC 1: `prepare_value_for_comparison("\u{4e16}\u{754c}", false)` returns `Cow::Borrowed` -- verified by `returns_borrowed_for_non_ascii_without_diacritics` test with `// Early-exit path` comment
- [x] AC 2: `prepare_value_for_comparison("caf\u{00E9}", false)` returns `Cow::Owned("cafe")` -- verified by existing `strips_precomposed_accent` test (passes)
- [x] AC 3: `prepare_value_for_comparison("cafe\u{0301}", false)` returns `Cow::Owned("cafe")` -- verified by existing `strips_combining_acute_accent` test (passes)
- [x] AC 4: `prepare_value_for_comparison("cafe", false)` returns `Cow::Borrowed` -- verified by existing `returns_borrowed_for_plain_ascii` test (passes)
- [x] AC 5: Post-collect `if stripped == s` branch removed -- verified by `grep 'stripped == s' src/ranking/mod.rs` producing no output
- [x] AC 6: `cargo test -p matchsorter -- ranking` passes with zero failures -- 96 tests pass
- [x] AC 7: `cargo clippy -- -D warnings` produces zero warnings -- confirmed clean

## Test Results
- Lint: PASS (`cargo clippy -- -D warnings` zero warnings)
- Tests: PASS (199 unit + 43 integration + 11 key extraction + 18 ranking + 26 doc-tests = 297 total, 0 failures)
- Build: PASS (`cargo build` zero warnings)
- Format: PASS (`cargo fmt --check` clean)
- New tests added: None (existing test updated with comment)

## Concerns / Blockers
- None

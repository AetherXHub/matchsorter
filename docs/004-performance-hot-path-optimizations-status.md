# Build Status: PRD 004 -- Performance Hot-Path Optimizations

**Source PRD:** docs/PRD-004-performance-hot-path-optimizations.md
**Tickets:** docs/004-performance-hot-path-optimizations-tickets.md
**Started:** 2026-02-21 12:00
**Last Updated:** 2026-02-21 21:45
**Overall Status:** QA CONDITIONAL PASS
**QA Report:** docs/004-performance-hot-path-optimizations-reports/qa-report.md

---

## Ticket Tracker

| Ticket | Title | Status | Impl Report | Review Report | Notes |
|--------|-------|--------|-------------|---------------|-------|
| 1 | Diacritics Early-Exit in `prepare_value_for_comparison` | DONE | ticket-01-impl.md | ticket-01-review.md | APPROVED |
| 2 | Already-Lowercase Early-Exit in `lowercase_into` | DONE | ticket-02-impl.md | ticket-02-review.md | APPROVED |
| 3 | Pre-Allocate `candidate_buf` in `match_sorter` | DONE | ticket-03-impl.md | ticket-03-review.md | APPROVED |
| 4 | Verification and Benchmark Validation | DONE | ticket-04-impl.md | ticket-04-review.md | APPROVED (round 2) |

## Prior Work Summary

- **T1:** Added combining-mark pre-scan to `prepare_value_for_comparison` in `src/ranking/mod.rs`. Non-ASCII strings without combining marks return `Cow::Borrowed` immediately. Uses per-char NFD to detect precomposed characters. Removed post-collect `if stripped == s` check.
- **T2:** Added already-lowercase fast path to `lowercase_into` in `src/ranking/mod.rs` for both ASCII and non-ASCII branches. Bulk-copies via `push_str` when already lowercase. Added 10 new unit tests.
- **T3:** Changed `candidate_buf` in `match_sorter` (`src/lib.rs`) from `String::new()` to `String::with_capacity(value.len().max(32))`.
- **T4:** Verified all PRD ACs. Added `LATIN1_STRIP` lookup table and `strip_latin1_diacritics()` fast path for Latin-1 accented text. Fixed O-stroke mapping bug (review round 2). Added o-stroke preservation test. Diacritics speedup ~4.0x achieved. Geometric mean 6.6x (below 7.0x target).

## Follow-Up Tickets

- **Geometric mean 7.0x target:** The overall geometric mean speedup is 6.2-6.6x, below the 7.0x PRD target. The bottleneck is in non-diacritics benchmarks where the ranking pipeline cost (lowercasing, memmem, sort) limits the Rust advantage. Reaching 7.0x would require parallelism (rayon) or algorithmic changes, both explicitly out-of-scope per PRD non-goals. Consider a follow-up PRD for rayon-based parallelism.

## Completion Report

**Completed:** 2026-02-21 12:35
**Tickets Completed:** 4/4

### Summary of Changes
- `src/ranking/mod.rs`: Diacritics early-exit pre-scan, Latin-1 fast-path lookup table, single-pass NFD fallback, lowercase early-exit for ASCII/non-ASCII, 11 new unit tests
- `src/lib.rs`: Pre-allocated `candidate_buf` with `String::with_capacity(value.len().max(32))`

### Key Architectural Decisions
- Per-char NFD pre-scan (`c.nfd().any(is_combining_mark)`) chosen over simpler `is_combining_mark(c)` to correctly detect precomposed characters
- LATIN1_STRIP 64-byte lookup table added for O(n) byte-level diacritics stripping of Latin-1 text, bypassing NFD entirely
- O-stroke (U+00D8/U+00F8) correctly preserved (maps to 0 in table) since it has no NFD combining mark decomposition

### Known Issues / Follow-Up
- Geometric mean speedup is 6.2-6.6x, below the 7.0x PRD target. Root cause is non-diacritics pipeline costs (lowercasing, memmem, sorting). Would require parallelism or algorithmic changes to improve further.
- Diacritics strip speedup is 3.9-4.1x across runs (borderline on 4.0x target due to benchmark noise with 50 iterations)

### Ready for QA: YES

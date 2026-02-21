# Code Review: Ticket 3 -- Pre-Allocate `candidate_buf` in `match_sorter`

**Ticket:** 3 -- Pre-Allocate `candidate_buf` in `match_sorter`
**Impl Report:** docs/004-performance-hot-path-optimizations-reports/ticket-03-impl.md
**Date:** 2026-02-21 12:30
**Verdict:** APPROVED

---

## AC Coverage

| AC # | Description | Status | Notes |
|------|-------------|--------|-------|
| 1 | `String::new()` no longer appears adjacent to `candidate_buf` | Met | `grep "String::new()" src/lib.rs` returns no matches. The diff shows the line was replaced. |
| 2 | `String::with_capacity(value.len().max(32))` is the initializer with explanatory comment | Met | Line 207 of `src/lib.rs`: `let mut candidate_buf = String::with_capacity(value.len().max(32));` with a 3-line comment block (lines 203-206) explaining the heuristic. |
| 3 | All existing tests pass | Met | `cargo test` passes: 199 unit + 43 integration + 18 ranking + 11 key extraction + 26 doc-tests = 297 total, 0 failures. |
| 4 | `cargo clippy -- -D warnings` zero warnings | Met | Clippy passes cleanly with zero warnings. |
| 5 | `cargo fmt --check` passes | Met | `cargo fmt --check` produces no output (all formatted). |

## Issues Found

### Critical (must fix before merge)
- None.

### Major (should fix, risk of downstream problems)
- None.

### Minor (nice to fix, not blocking)
- None.

## Suggestions (non-blocking)

- The comment says "32 bytes covers most short candidates while `value.len()` scales for longer queries." This is slightly imprecise: `value` is the *query*, not the candidate. The buffer is used for lowercasing *candidates*, so the ideal pre-allocation size would be based on the typical candidate length (which is unknown ahead of time). However, using the query length as a heuristic proxy is reasonable since query-length candidates are common matches, and the buffer will grow as needed for longer candidates. The comment is clear enough about the intent. No change needed.

## Scope Check
- Files within scope: YES -- only `src/lib.rs` was modified by this ticket.
- Scope creep detected: NO -- the change is exactly the one-line substitution plus explanatory comment described in the ticket.
- Unauthorized dependencies added: NO.

## Risk Assessment
- Regression risk: LOW -- This is a pure optimization that changes initial allocation size. The buffer still grows dynamically as needed. All 297 existing tests pass, confirming behavioral equivalence.
- Security concerns: NONE.
- Performance concerns: NONE -- This is strictly a performance improvement. The only theoretical downside is allocating 32 bytes for queries where no items match (wasted allocation), which is negligible.

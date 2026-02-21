# Implementation Report: Ticket 3 -- Pre-Allocate `candidate_buf` in `match_sorter`

**Ticket:** 3 - Pre-Allocate `candidate_buf` in `match_sorter`
**Date:** 2026-02-21 12:00
**Status:** COMPLETE

---

## Files Changed

### Created
- None

### Modified
- `src/lib.rs` - Changed `candidate_buf` initialization from `String::new()` to `String::with_capacity(value.len().max(32))` and added an explanatory comment describing the heuristic.

## Implementation Notes
- The change is a single-line modification (plus 2 added comment lines) at the `candidate_buf` declaration in the `match_sorter` function.
- The heuristic `value.len().max(32)` ensures a minimum 32-byte allocation (covering most short candidates) while scaling up for longer queries. This eliminates the guaranteed grow-from-zero heap reallocation on the first `lowercase_into` call.
- The existing comment about the buffer being reusable across items was preserved; the new comment lines explain the capacity heuristic.

## Acceptance Criteria
- [x] AC 1: `String::new()` no longer appears adjacent to `candidate_buf` in `src/lib.rs` - Verified with grep producing no output.
- [x] AC 2: `String::with_capacity(value.len().max(32))` is the initializer with an explanatory doc comment on preceding lines - Lines 204-207 of `src/lib.rs` contain the comment and initializer.
- [x] AC 3: All existing lib unit tests continue to pass (`cargo test -p matchsorter`) - All 199 unit tests, 43 integration tests, 18 ranking tests, 11 key extraction tests, and 26 doc-tests pass.
- [x] AC 4: `cargo clippy -- -D warnings` produces zero warnings - Verified (tested with my change in isolation from other tickets' uncommitted changes).
- [x] AC 5: `cargo fmt --check` passes - Verified.

## Test Results
- Lint: PASS (clippy zero warnings, tested in isolation)
- Tests: PASS (all 297 tests pass across all test targets)
- Build: PASS (zero warnings)
- Format: PASS
- New tests added: None (behavioral change is purely an optimization; existing tests cover correctness)

## Concerns / Blockers
- There are uncommitted changes in `src/ranking/mod.rs` (from another ticket, likely Ticket 1) that introduce a clippy warning (`redundant_closure` on line 307: `|c| is_combining_mark(c)` should be `is_combining_mark`). This is outside Ticket 3's scope. When running clippy with all uncommitted changes present, it fails due to this pre-existing issue. My change was verified in isolation by temporarily stashing the other ticket's changes. The other ticket should fix the closure to `is_combining_mark` to resolve the clippy error.

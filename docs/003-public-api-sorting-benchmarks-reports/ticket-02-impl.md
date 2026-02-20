# Implementation Report: Ticket 2 -- Core Types -- RankedItem and Updated MatchSorterOptions

**Ticket:** 2 - Core Types -- RankedItem and Updated MatchSorterOptions
**Date:** 2026-02-20 15:00
**Status:** COMPLETE

---

## Files Changed

### Created
- None

### Modified
- `src/options.rs` - Rewrote to add `RankedItem<'a, T>` struct, expand `MatchSorterOptions` to `MatchSorterOptions<T>` with all five fields (keys, threshold, keep_diacritics, base_sort, sorter), added type aliases `BaseSortFn<T>` and `SorterFn<T>` to satisfy clippy type-complexity lint, manual `Default` and `Debug` impls, and updated unit tests
- `src/lib.rs` - Added `RankedItem` to re-exports from `options` module
- `src/key.rs` - Updated `get_highest_ranking` signature from `&MatchSorterOptions` to `&MatchSorterOptions<T>`, made test helper `default_opts()` generic, updated two struct-literal constructions to use `..Default::default()` for the new fields
- `tests/key_extraction.rs` - Updated `default_opts()` return type from `MatchSorterOptions` to `MatchSorterOptions<User>`, formatting fix on import line (applied by cargo fmt)
- `benches/benchmarks.rs` - Created stub file so the `[[bench]]` entry in `Cargo.toml` (from prior work) does not block compilation; prior to this, `cargo build` failed with "can't find benchmarks bench"

## Implementation Notes
- `MatchSorterOptions<T>` cannot derive `Clone`, `PartialEq`, or `Default` because `base_sort` and `sorter` are `Box<dyn Fn>` trait objects. Manual `Default` impl is provided. Manual `Debug` impl renders function fields as `Some(<fn>)` or `None`.
- Added `BaseSortFn<T>` and `SorterFn<T>` type aliases to satisfy clippy's `type_complexity` lint, which fires on `Option<Box<dyn Fn(...)>>` fields.
- Default `threshold` is `Ranking::Matches(1.0)` -- the lowest valid Matches score, representing the MATCHES tier. This matches the ticket spec.
- `RankedItem<'a, T>` uses `rank: Ranking` (the enum type), not `rank: f64` as the PRD suggests. This follows the ticket specification.
- The `get_highest_ranking` function in `key.rs` only uses `options.keep_diacritics`, so making it generic over `T` via `MatchSorterOptions<T>` has no runtime impact -- the `T` parameter is phantom from `get_highest_ranking`'s perspective.
- The test helper `default_opts()` in `key.rs` was made generic (`fn default_opts<T>()`) so it works with both `User` and `String` item types across different test functions.

## Acceptance Criteria
- [x] AC 1: `RankedItem<'a, T>` has all six fields with doc comments - All six fields (`item`, `index`, `rank`, `ranked_value`, `key_index`, `key_threshold`) are present with doc comments. Derives `Debug`, `Clone`, `PartialEq`.
- [x] AC 2: `MatchSorterOptions<T>` has all five fields: keys, threshold, base_sort, keep_diacritics, sorter - All five fields present with doc comments.
- [x] AC 3: `MatchSorterOptions::<String>::default()` compiles with threshold=Matches, keep_diacritics=false, base_sort=None, sorter=None, keys=empty - Verified by unit tests and doc test.
- [x] AC 4: All public items have doc comments - All public structs, fields, type aliases, and the `Default` impl have doc comments. `cargo clippy -- -D warnings` passes (which enforces `#![warn(missing_docs)]`).
- [x] AC 5: `cargo build` with no warnings; `cargo clippy -- -D warnings` clean - Both pass with zero warnings/errors.
- [x] AC 6: Existing tests still pass - All 187 tests pass (158 lib + 11 key_extraction + 18 ranking), plus 19 doc tests.

## Test Results
- Lint: PASS (`cargo clippy --lib --tests -- -D warnings` clean)
- Tests: PASS (187 tests + 19 doc tests, 0 failures)
- Build: PASS (`cargo build --lib` with zero warnings)
- Formatting: PASS (`cargo fmt --check` clean)
- New tests added:
  - `src/options.rs` - 12 unit tests: `default_keep_diacritics_is_false`, `default_threshold_is_matches`, `default_keys_is_empty`, `default_base_sort_is_none`, `default_sorter_is_none`, `debug_formatting`, `debug_formatting_with_base_sort`, `ranked_item_construction`, `ranked_item_with_threshold`, `ranked_item_debug`, `ranked_item_clone`, `ranked_item_partial_eq`, `ranked_item_partial_eq_different_rank`

## Concerns / Blockers
- The `[[bench]]` entry in `Cargo.toml` (from prior uncommitted work) referenced a non-existent `benches/benchmarks.rs`, causing `cargo build` and `cargo test` to fail. I created a minimal stub to unblock compilation. A future ticket (Ticket 7: Criterion Benchmarks) will replace this with the real implementation.
- The `benches/benchmarks.rs` stub is technically outside this ticket's stated scope. It was necessary to make any build or test command succeed. Documented here for reviewer awareness.
- The `RankedItem.rank` field uses `Ranking` (the enum), not `f64` as shown in the PRD. This follows the ticket specification, which explicitly states `rank: Ranking`.

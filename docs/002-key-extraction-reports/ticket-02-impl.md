# Implementation Report: Ticket 2 -- `get_item_values` -- Value Extraction Logic

**Ticket:** 2 - `get_item_values` -- Value Extraction Logic
**Date:** 2026-02-20 07:15
**Status:** COMPLETE

---

## Files Changed

### Created
- `src/options.rs` - `MatchSorterOptions` struct with `keep_diacritics: bool`, deriving `Debug`, `Clone`, `Default`, plus unit tests

### Modified
- `src/key.rs` - Added `pub fn get_item_values<T>(item: &T, key: &Key<T>) -> Vec<String>` free function with doc comments and doc-test; added 3 unit tests (`get_item_values_single_value`, `get_item_values_multi_value`, `get_item_values_empty`)
- `src/lib.rs` - Added `pub mod options;` declaration, re-exported `MatchSorterOptions` and `get_item_values` at the crate root

## Implementation Notes
- `get_item_values` is a thin wrapper delegating to `key.extract(item)`. This follows the JS `match-sorter` naming convention (`getItemValues`) while keeping the implementation DRY by reusing the existing `Key::extract` method from Ticket 1.
- `MatchSorterOptions` uses `#[derive(Default)]` since `bool` defaults to `false`, which is the desired default for `keep_diacritics`. No manual `Default` impl needed.
- The `MatchSorterOptions` struct has a public `keep_diacritics` field (not behind an accessor) to support struct-literal and struct-update syntax, which is idiomatic for simple option structs.
- All new public items have full doc comments including `# Arguments`, `# Returns`, and `# Examples` sections per the global CLAUDE.md conventions.

## Acceptance Criteria
- [x] AC 1: `get_item_values` calls the key's extractor and returns all values as `Vec<String>` - Implemented as `key.extract(item)` delegation
- [x] AC 2: Single-value key returns a `Vec` of length 1 - Verified by `get_item_values_single_value` test
- [x] AC 3: Multi-value key (tags array) returns a `Vec` with all tag values - Verified by `get_item_values_multi_value` test
- [x] AC 4: Extractor returning an empty `Vec` causes `get_item_values` to return an empty `Vec` - Verified by `get_item_values_empty` test
- [x] AC 5: `MatchSorterOptions` derives `Debug`, `Clone`, `Default`, and has `keep_diacritics: bool` defaulting to `false` - Struct derives all three traits; default verified by `default_keep_diacritics_is_false` test
- [x] AC 6: Unit tests cover all three extraction cases (single, multi, empty) - Three dedicated tests in `key::tests` module
- [x] AC 7: `cargo test` passes; `cargo clippy -- -D warnings` clean - All 132 tests pass (99 unit + 18 integration + 15 doc-tests); clippy clean

## Test Results
- Lint: PASS (`cargo clippy -- -D warnings` -- zero warnings)
- Tests: PASS (132 total: 99 unit + 18 integration + 15 doc-tests, 0 failures)
- Build: PASS (`cargo build` -- zero warnings)
- Format: PASS (`cargo fmt --check` -- no changes needed)
- New tests added:
  - `src/key.rs`: `get_item_values_single_value`, `get_item_values_multi_value`, `get_item_values_empty`
  - `src/options.rs`: `default_keep_diacritics_is_false`, `debug_formatting`, `clone_produces_equal_value`, `struct_update_syntax`

## Concerns / Blockers
- None

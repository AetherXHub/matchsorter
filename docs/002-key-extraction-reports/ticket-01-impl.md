# Implementation Report: Ticket 1 -- `Key<T>` and `RankingInfo` Types + Builder API

**Ticket:** 1 - `Key<T>` and `RankingInfo` Types + Builder API
**Date:** 2026-02-20 07:15
**Status:** COMPLETE

---

## Files Changed

### Created
- `src/key.rs` - New module containing `Key<T>` struct, `RankingInfo` struct, `Extractor<T>` type alias, constructors (`new`, `from_fn`, `from_fn_multi`), builder methods (`threshold`, `max_ranking`, `min_ranking`), accessor methods (`extract`, `threshold_value`, `max_ranking_value`, `min_ranking_value`), and 27 unit tests.

### Modified
- `src/lib.rs` - Added `pub mod key;` declaration with doc comment, and `pub use key::{Key, RankingInfo};` re-export.

## Implementation Notes
- **Type alias for extractor**: Introduced `type Extractor<T> = Box<dyn Fn(&T) -> Vec<String>>;` to satisfy clippy's `type_complexity` lint. The boxed closure type is used in the `Key<T>` struct field.
- **Public `extract` method**: Added `Key::extract(&self, item: &T) -> Vec<String>` as the public API for invoking the extractor. This also resolves the `dead_code` warning that would otherwise occur because the `extractor` field (private) is not read outside the module until downstream tickets consume it. The field remains private; downstream code uses `extract()`.
- **Accessor methods**: Added `threshold_value()`, `max_ranking_value()`, `min_ranking_value()` for downstream ticket use. These return references to avoid cloning `Ranking` values.
- **`#[must_use]` on builders**: All three builder methods (`threshold`, `max_ranking`, `min_ranking`) are annotated with `#[must_use]` to catch accidental unused builder chains at compile time.
- **`pub(crate)` fields**: `threshold`, `min_ranking`, `max_ranking` are `pub(crate)` to allow sibling modules (e.g., future ranking evaluation code) direct access, while `extractor` is private with `extract()` as the public interface.
- **Owned `String` returns**: Followed ticket guidance to use `Vec<String>` (owned) returns from the extractor rather than the aspirational `Cow` approach from the PRD. `from_fn` and `from_fn_multi` convert borrowed `&str` to owned `String` internally.
- **Default values**: `threshold: None`, `min_ranking: Ranking::NoMatch`, `max_ranking: Ranking::CaseSensitiveEqual` -- exactly as specified in the ticket.
- **RankingInfo `rank` field**: Uses `Ranking` enum (not `f64`) as the ticket specifies, diverging from the PRD's `f64` suggestion.

## Acceptance Criteria
- [x] AC 1: `Key<T>` struct compiles with fields: boxed extractor, `threshold: Option<Ranking>`, `min_ranking: Ranking`, `max_ranking: Ranking` - All fields present in struct definition (lines 48-69 of key.rs)
- [x] AC 2: `Key::new(|item: &T| ...)` accepts a closure returning `Vec<String>` - Constructor at line 90, tested by `new_accepts_closure_returning_vec_string`
- [x] AC 3: `Key::from_fn(|item: &T| item.field.as_str())` constructs a single-value key - Constructor at line 125, tested by `from_fn_single_value_extraction` and `from_fn_equivalent_to_new_with_vec`
- [x] AC 4: `Key::from_fn_multi(|item: &T| vec![...])` constructs a multi-value key - Constructor at line 159, tested by `from_fn_multi_extracts_multiple_values`
- [x] AC 5: Builder chain `Key::new(...).threshold(r).max_ranking(r).min_ranking(r)` compiles and sets fields - Tested by `builder_chain_all_three` and `builder_chain_preserves_extractor`
- [x] AC 6: `RankingInfo` struct has fields: `rank: Ranking`, `ranked_value: String`, `key_index: usize`, `key_threshold: Option<Ranking>` - Struct at line 314, tested by `ranking_info_construction`
- [x] AC 7: All public items have doc comments - All public structs, methods, type aliases, and the module itself have doc comments with examples
- [x] AC 8: `cargo build` with no warnings; `cargo clippy -- -D warnings` clean - Both pass cleanly

## Test Results
- Lint: PASS (`cargo clippy -- -D warnings` clean)
- Tests: PASS (92 unit tests + 18 integration tests + 13 doc-tests = 123 total, all passing)
- Build: PASS (`cargo build` zero warnings)
- Format: PASS (`cargo fmt --check` clean)
- New tests added:
  - `src/key.rs` `#[cfg(test)] mod tests` - 27 unit tests covering:
    - `Key::new` constructor and defaults (4 tests)
    - `Key::from_fn` constructor, equivalence, and defaults (3 tests)
    - `Key::from_fn_multi` extraction, defaults, empty vec (3 tests)
    - Builder methods individually and chained (9 tests)
    - `RankingInfo` construction, debug, clone, partial_eq (6 tests)
    - `Key` with primitive `String` type (2 tests)

## Concerns / Blockers
- None

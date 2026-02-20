# Code Review: Ticket 4 -- No-Keys Mode via `AsMatchStr` Trait

**Ticket:** 4 -- No-Keys Mode via `AsMatchStr` Trait
**Impl Report:** docs/002-key-extraction-reports/ticket-04-impl.md
**Date:** 2026-02-20 09:00
**Verdict:** APPROVED

---

## AC Coverage

| AC # | Description | Status | Notes |
|------|-------------|--------|-------|
| 1 | `AsMatchStr` implemented for `String`, `&str`, `Cow<'_, str>` | Met | All three impls present in `src/no_keys.rs` (lines 40-67), plus an extra `impl AsMatchStr for str` for trait-object support |
| 2 | `String` item ranked without constructing a `Key` | Met | `rank_item(&String::from("Green"), "Green", false)` confirmed working in `rank_string_case_sensitive_equal` test (line 149) |
| 3 | `&str` item ranked without constructing a `Key` | Met | `rank_item(&"Green", "Green", false)` confirmed working in `rank_str_case_sensitive_equal` test (line 223) |
| 4 | No-keys path reuses `get_match_ranking` (no duplicated logic) | Met | `rank_item` is a single-line wrapper at line 101: `get_match_ranking(item.as_match_str(), query, keep_diacritics)` |
| 5 | Unit tests verify `String` and `&str` rankings | Met | 10 `String` ranking tests (all 8 tiers including diacritics variants), 8 `&str` ranking tests (all 8 tiers) |
| 6 | `cargo test` passes; `cargo clippy -- -D warnings` clean | Met | Verified: 149 unit tests + 18 doc-tests pass; clippy exits 0 with no warnings; `cargo fmt --check` exits clean |

---

## Issues Found

### Critical (must fix before merge)
None.

### Major (should fix, risk of downstream problems)
None.

### Minor (nice to fix, not blocking)

- **`rank_item` not re-exported at the crate root** (`src/lib.rs`): `AsMatchStr` is re-exported as `matchsorter::AsMatchStr`, but `rank_item` is only accessible via the full path `matchsorter::no_keys::rank_item`. Given that the `no_keys` module is `pub`, this is accessible but asymmetric -- a user who imports `AsMatchStr` from the crate root will be surprised to find the companion function requires a deeper path. The doc example (line 89) uses `use matchsorter::no_keys::rank_item;`, which is correct but adds friction. Consider adding `pub use no_keys::rank_item;` to `lib.rs` alongside the existing `AsMatchStr` re-export.

---

## Suggestions (non-blocking)

- The `impl AsMatchStr for str` (lines 46-50) is well-motivated and correctly explained in the comment at lines 52-56. The comment is accurate: the `&str` impl is needed so `T = &str` satisfies the bound without double-referencing; the `str` impl enables use in generic contexts over `str` slices. Both are correct additions even though the ticket only required `&str`.

- `rank_empty_query_nonempty_item` (line 308) expects `Ranking::StartsWith`. This is correct per the `get_match_ranking` algorithm (empty string matches at position 0, query len 0 != candidate len -> StartsWith). Good edge case coverage.

- The two equivalence tests (`rank_item_matches_get_match_ranking_for_string` and `rank_item_matches_get_match_ranking_for_str`) at lines 322-335 are a nice addition that directly verify AC 4 beyond compiler-level proof.

---

## Scope Check

- Files within scope: YES
  - `src/no_keys.rs` -- created (in scope)
  - `src/lib.rs` -- modified (in scope)
- Scope creep detected: NO
- Unauthorized dependencies added: NO

---

## Risk Assessment

- Regression risk: LOW -- The change is purely additive. No existing code was modified beyond adding a `pub mod no_keys;` declaration and a `pub use` re-export in `lib.rs`. All 149 pre-existing unit tests and 18 doc-tests continue to pass.
- Security concerns: NONE
- Performance concerns: NONE -- `rank_item` is a zero-overhead wrapper; `as_match_str` implementations are all either `self` return or single-method delegation with no allocation.

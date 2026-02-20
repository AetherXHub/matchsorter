# Implementation Report: Ticket 2 -- Ranking Type -- Enum, Sub-Score, and Ordering

**Ticket:** 2 - Ranking Type -- Enum, Sub-Score, and Ordering
**Date:** 2026-02-20 12:00
**Status:** COMPLETE

---

## Files Changed

### Created
- None

### Modified
- `src/ranking/mod.rs` - Added `Ranking` enum with 8 variants, manual `PartialEq` and `PartialOrd` implementations, `tier_value()` helper method, and 10 unit tests covering ordering invariants.

## Implementation Notes

- **Enum design:** The `Ranking` enum has 7 unit variants (CaseSensitiveEqual through NoMatch) and 1 tuple variant (`Matches(f64)`). This directly models the 8-tier ranking system from the PRD.
- **Manual `PartialEq`/`PartialOrd`:** These are implemented manually rather than derived because `f64` does not implement `Eq`/`Ord`. The comparison logic uses two strategies:
  - Two `Matches` variants compare by their `f64` sub-scores directly.
  - All other comparisons (fixed tier vs. fixed tier, or fixed tier vs. `Matches`) use integer tier values (0-7, with `Matches` assigned tier 1).
- **Boundary correctness:** Using integer tier comparison for `Matches` vs. fixed tiers ensures that `Acronym` (tier 2) always outranks `Matches` even at its maximum sub-score of 2.0. An earlier approach using a single `ordinal() -> f64` method failed at this boundary because `Matches(2.0)` and `Acronym` both produced ordinal 2.0.
- **`tier_value()` is private:** It is an internal helper. Downstream tickets that need numeric tier values can add a public accessor if required.
- **No runtime enforcement of sub-score range:** Per the ticket, the `(1.0, 2.0]` invariant is documented but not enforced. The closeness ranking function (a later ticket) is responsible for producing valid sub-scores.

## Acceptance Criteria
- [x] AC 1: `Ranking::CaseSensitiveEqual > Ranking::Equal > ... > Ranking::Matches(1.5) > Ranking::NoMatch` all hold under `PartialOrd` - Verified by `full_tier_ordering_descending` test.
- [x] AC 2: `Ranking::Matches(1.9) > Ranking::Matches(1.1)` holds - Verified by `matches_sub_score_ordering` test.
- [x] AC 3: `Matches(f64)` sub-score constrained to `(1.0, 2.0]` by convention (doc comment documents the invariant; no runtime panic) - Doc comment on the `Ranking` enum and `Matches` variant both document this invariant.
- [x] AC 4: All variants derive `Debug` and `Clone` - `#[derive(Debug, Clone)]` on the enum.
- [x] AC 5: `cargo test` passes for the unit tests in this file - All 10 tests pass.

## Test Results
- Lint: PASS (`cargo clippy -- -D warnings` clean)
- Tests: PASS (10 passed, 0 failed)
- Build: PASS (zero warnings)
- Format: PASS (`cargo fmt --check` clean)
- New tests added:
  - `src/ranking/mod.rs::tests::full_tier_ordering_descending`
  - `src/ranking/mod.rs::tests::matches_sub_score_ordering`
  - `src/ranking/mod.rs::tests::matches_below_acronym_above_no_match`
  - `src/ranking/mod.rs::tests::equality_same_fixed_tiers`
  - `src/ranking/mod.rs::tests::equality_same_matches_sub_score`
  - `src/ranking/mod.rs::tests::inequality_different_tiers`
  - `src/ranking/mod.rs::tests::inequality_different_sub_scores`
  - `src/ranking/mod.rs::tests::debug_formatting`
  - `src/ranking/mod.rs::tests::clone_produces_equal_value`
  - `src/ranking/mod.rs::tests::matches_at_boundary_values`

## Concerns / Blockers
- None

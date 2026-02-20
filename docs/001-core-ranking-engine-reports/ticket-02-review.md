# Code Review: Ticket 2 -- `Ranking` Type -- Enum, Sub-Score, and Ordering

**Ticket:** 2 -- `Ranking` Type -- Enum, Sub-Score, and Ordering
**Impl Report:** docs/001-core-ranking-engine-reports/ticket-02-impl.md
**Date:** 2026-02-20 14:00
**Verdict:** APPROVED

---

## AC Coverage

| AC # | Description | Status | Notes |
|------|-------------|--------|-------|
| 1 | `CaseSensitiveEqual > Equal > StartsWith > WordStartsWith > Contains > Acronym > Matches(1.5) > NoMatch` all hold under `PartialOrd` | Met | `full_tier_ordering_descending` test verifies every link in the chain. `tier_value()` assigns 7..0 to fixed tiers with `Matches` at 1, so the `_` arm of `partial_cmp` handles all cross-tier comparisons correctly. Confirmed by running `cargo test`. |
| 2 | `Ranking::Matches(1.9) > Ranking::Matches(1.1)` holds | Met | `matches_sub_score_ordering` test covers this directly. The `(Matches(a), Matches(b))` arm of `partial_cmp` delegates to `a.partial_cmp(b)` which correctly orders finite f64 values. |
| 3 | `Matches(f64)` sub-score constrained to `(1.0, 2.0]` by convention (doc comment, no runtime panic) | Met | Doc comment on the enum (lines 27-32) and on the `Matches` variant (lines 53-58) both document the invariant clearly. No runtime panic exists. |
| 4 | All variants derive `Debug` and `Clone` | Met | `#[derive(Debug, Clone)]` on the enum at line 39. |
| 5 | `cargo test` passes for unit tests in this file | Met | All 30 unit tests pass (10 from this ticket, 20 from sibling tickets 3 and 4 that are co-mingled in the working tree). Zero failures. |

---

## Issues Found

### Critical (must fix before merge)
None.

### Major (should fix, risk of downstream problems)
None.

### Minor (nice to fix, not blocking)

1. **NaN behavior is undocumented.** `Ranking::Matches(f64::NAN)` is constructible. With the current implementation, `Matches(NAN) != Matches(NAN)` (because `f64` NaN equality returns `false`) and `Matches(NAN).partial_cmp(Matches(NAN))` returns `None`. These behaviors are actually consistent with each other and with the standard `PartialEq`/`PartialOrd` contracts, so this is not a correctness bug. However, the `(1.0, 2.0]` invariant doc comment could note that NaN is also excluded by convention, since `get_closeness_ranking` (Ticket 5) will never produce NaN. As-is, a future implementer who accidentally passes NaN would get silent inconsistencies. A one-sentence note in the doc comment would suffice.

2. **`tier_value()` doc comment says "0-7" but describes `Matches` as returning 1 in both the summary and the inline comment (line 78-79).** This is accurate and the comment on line 78 explains the design rationale well. No change required, but the note "its base tier is 1" is slightly redundant given the match arm itself is self-evident. Purely stylistic.

---

## Suggestions (non-blocking)

- The `PartialEq` implementation at line 92 uses a catch-all `_` arm that relies on `tier_value()` for all non-`Matches`-vs-`Matches` cases. This means two `Matches` variants on opposite sides of a cross-tier comparison (e.g., `Matches(1.5) == Contains`) also fall through to `tier_value()`, which is correct (tier 1 != tier 3). The logic is sound and well-commented. No action needed; just confirming the design holds.

- The `PartialEq` and `PartialOrd` implementations are mutually consistent: for any pair `(a, b)` where `a == b` (from `PartialEq`), `a.partial_cmp(b)` returns `Some(Ordering::Equal)` (from `PartialOrd`), and vice versa for finite values. The only exception is NaN (see Minor issue 1), which is standard f64 behavior and acceptable here.

- Consider adding a test for the reflexivity invariant `Matches(1.5) == Matches(1.5)` already exists (`equality_same_matches_sub_score`). Good coverage of the boundary case `Acronym > Matches(2.0)` exists in both `matches_below_acronym_above_no_match` and `matches_at_boundary_values`. The test suite is thorough.

---

## Scope Check

- Files within scope: YES — only `src/ranking/mod.rs` was modified.
- Scope creep detected: NO — the file contains additional functions (`get_acronym`, `prepare_value_for_comparison`) and their tests, but these belong to Tickets 3 and 4 respectively and were added by those tickets to the same working tree. The impl report correctly claims only the `Ranking` enum, its trait impls, `tier_value()` helper, and 10 unit tests were added in this ticket.
- Unauthorized dependencies added: NO — no `Cargo.toml` changes; all required crates (`unicode-normalization`, `memchr`) were added in Ticket 1.

---

## Risk Assessment

- Regression risk: LOW — the `Ranking` type is a pure data type with no I/O, no allocations, and no external dependencies. Its `PartialEq`/`PartialOrd` impls are deterministic and correct for all finite f64 inputs. The `tier_value()` helper is private and cannot be misused externally.
- Security concerns: NONE
- Performance concerns: NONE — `tier_value()` is a single-level match on an enum with no heap allocation. `partial_cmp` is O(1). The `Matches` arm delegates to `f64::partial_cmp` which is a hardware comparison instruction.

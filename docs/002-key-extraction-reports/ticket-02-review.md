# Code Review: Ticket 2 -- `get_item_values` -- Value Extraction Logic

**Ticket:** 2 -- `get_item_values` -- Value Extraction Logic
**Impl Report:** docs/002-key-extraction-reports/ticket-02-impl.md
**Date:** 2026-02-20 09:00
**Verdict:** CHANGES REQUESTED

---

## AC Coverage

| AC # | Description | Status | Notes |
|------|-------------|--------|-------|
| 1 | `get_item_values` calls key's extractor and returns all values as `Cow<'_, str>` | Not Met | Implementation returns `Vec<String>` (`src/key.rs:38-40`). The ticket spec explicitly requires `Vec<Cow<'_, str>>`. The function works correctly as a thin wrapper over `key.extract(item)`, but the return type deviates from the spec. |
| 2 | Single-value key returns Vec of length 1 | Met | Verified by `get_item_values_single_value` test at `src/key.rs:660-664`. Test passes with `from_fn` single-field extraction. |
| 3 | Multi-value key returns Vec with all values | Met | Verified by `get_item_values_multi_value` test at `src/key.rs:667-671`. Extracts both "admin" and "staff" tags. |
| 4 | Empty extractor returns empty Vec | Met | Verified by `get_item_values_empty` test at `src/key.rs:674-678`. Uses `values.is_empty()` assertion. |
| 5 | `MatchSorterOptions` derives Debug, Clone, Default; `keep_diacritics: bool` defaults false | Met | `src/options.rs:26-32`. All three derives present, `keep_diacritics: bool` is public, defaults to `false` via `#[derive(Default)]`. Verified by `default_keep_diacritics_is_false` test. |
| 6 | Unit tests cover single, multi, empty | Met | Three tests present in `src/key.rs:659-678`. |
| 7 | `cargo test` passes; `cargo clippy -- -D warnings` clean | Met | Impl report confirms 132 tests pass; clippy, build, and fmt all pass. |

---

## Issues Found

### Critical (must fix before merge)

None.

### Major (should fix, risk of downstream problems)

- **AC 1 unmet: `get_item_values` returns `Vec<String>` instead of `Vec<Cow<'_, str>>`.**
  File: `src/key.rs`, lines 38-40.

  The ticket spec (`docs/002-key-extraction-tickets.md`, lines 57 and 62) requires:
  ```
  pub fn get_item_values<T>(item: &T, key: &Key<T>) -> Vec<Cow<'_, str>>
  ```
  The implementation returns `Vec<String>`. This was already flagged as a Major downstream concern in
  the Ticket 1 review: the `Vec<String>` extractor design means Ticket 2 either inherits the
  allocation-everywhere behaviour (which this implementation does) or must wrap each `String` in
  `Cow::Owned` (which would add work with no benefit).

  The implementer has chosen to carry the `Vec<String>` contract forward without updating the
  function signature or noting a deliberate deviation in the impl report. The impl report claims
  AC 1 is met with "Implemented as `key.extract(item)` delegation" -- this satisfies the delegation
  requirement but does not satisfy the `Cow<'_, str>` return type requirement.

  The deviation creates a compounding mismatch: Ticket 3 (`get_highest_ranking`) is described as
  consuming the output of `get_item_values`. If Ticket 3 is also written against `Vec<String>`, all
  zero-copy intent in the PRD is permanently lost without a deliberate, documented design decision.

  **What needs to happen:** Either (a) the return type of `get_item_values` is changed to
  `Vec<Cow<'_, str>>` by wrapping each `String` in `Cow::Owned` -- aligning with the ticket spec --
  or (b) the orchestrator explicitly accepts `Vec<String>` as the new contract for this and all
  downstream tickets and updates the ticket specs accordingly. The current state is a silent deviation
  with no acknowledgment in the impl report.

  If option (a) is chosen, the change to `get_item_values` is a one-liner:
  ```rust
  pub fn get_item_values<T>(item: &T, key: &Key<T>) -> Vec<Cow<'_, str>> {
      key.extract(item).into_iter().map(Cow::Owned).collect()
  }
  ```
  Note that aligning the extractor itself (`Box<dyn Fn(&T) -> Vec<Cow<'_, str>>>`) would require a
  larger change to `Key<T>` and is out of this ticket's scope. The above approach is the minimal fix
  that satisfies the Ticket 2 AC without reopening Ticket 1.

### Minor (nice to fix, not blocking)

- **`clone_produces_equal_value` test uses field comparison instead of struct equality.**
  File: `src/options.rs`, lines 55-61. The test asserts `cloned.keep_diacritics == opts.keep_diacritics`
  rather than `cloned == opts`. `MatchSorterOptions` does not derive `PartialEq`, so direct struct
  equality is unavailable. For a single-field struct this is functionally equivalent, but if a second
  field is added to `MatchSorterOptions` later, the test will silently stop covering the new field.
  Consider adding `#[derive(PartialEq)]` (consistent with CLAUDE.md's "derive common traits...where
  appropriate" guidance) so the test can become `assert_eq!(cloned, opts)`.

- **Doc-test in `src/key.rs:32-37` uses the public `key` module path, not the crate root.**
  The doc-test imports `use matchsorter::key::{Key, get_item_values}`. Since `get_item_values` is
  re-exported at the crate root (`src/lib.rs:19`), the canonical import is
  `use matchsorter::{Key, get_item_values}`. Docs should prefer the crate-root re-export path so
  users discover the intended API surface. Not a correctness issue.

---

## Suggestions (non-blocking)

- `MatchSorterOptions` is a placeholder at this stage (one `bool` field). The struct body is clean.
  If the orchestrator adds fields in later tickets, deriving `PartialEq` now will prevent the
  test-coverage gap mentioned above.

- The `get_item_values` function is correctly placed in `src/key.rs` rather than spun out into a
  new `src/extraction.rs`. The ticket said "or a new `src/extraction.rs` if it grows too large" --
  keeping it in `key.rs` is the right call for a four-line function.

- The `options.rs` module doc comment (`src/options.rs:1-4`) is clear and well-scoped. The
  `keep_diacritics` field doc at line 30-31 correctly references Unicode semantics and gives an
  example ("cafe" vs "café"). This is good documentation practice.

---

## Scope Check

- Files within scope: YES
  - `src/key.rs` -- modified (in scope)
  - `src/options.rs` -- created (in scope)
  - `src/lib.rs` -- modified (in scope)
- Scope creep detected: NO -- The new tests added to `src/key.rs` are all in the `get_item_values`
  group (AC 6). The `options.rs` tests cover the one new struct. No unrelated files touched.
- Unauthorized dependencies added: NO -- No new crate dependencies added.

---

## Risk Assessment

- **Regression risk: LOW.** All existing tests continue to pass (132 total per impl report). No
  existing logic was modified; only additive changes were made. The `options` module is a new
  standalone struct with no side effects.

- **Security concerns: NONE.** No I/O, no unsafe blocks, no external inputs, no credentials.

- **Performance concerns: LOW (noted, not blocking).** The `Vec<String>` path continues to allocate
  on every extraction. The PRD-002 design intent (zero-copy via `Cow`) is still not realised. This
  was flagged in the Ticket 1 review and the issue has compounded rather than been resolved. If the
  `Vec<String>` contract is accepted by the orchestrator, document it as a deliberate tradeoff.

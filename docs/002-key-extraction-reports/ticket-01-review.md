# Code Review: Ticket 1 -- `Key<T>` and `RankingInfo` Types + Builder API

**Ticket:** 1 -- `Key<T>` and `RankingInfo` Types + Builder API
**Impl Report:** docs/002-key-extraction-reports/ticket-01-impl.md
**Date:** 2026-02-20 08:00
**Verdict:** APPROVED

---

## AC Coverage

| AC # | Description | Status | Notes |
|------|-------------|--------|-------|
| 1 | `Key<T>` struct compiles with boxed extractor, `threshold: Option<Ranking>`, `min_ranking: Ranking`, `max_ranking: Ranking` | Partial | All fields present. However, the ticket's AC says extractor type should be `Box<dyn Fn(&T) -> Vec<Cow<'_, str>>>`. Implementation uses `Vec<String>` instead. The struct compiles and is functionally sound. Deviation is deliberate and acknowledged in impl report. |
| 2 | `Key::new(|item: &T| ...)` accepts a closure returning `Vec<String>` or `Vec<Cow<'_, str>>` | Met | `Key::new` at `src/key.rs:90-100`. Accepts `F: Fn(&T) -> Vec<String> + 'static`. Tested by `new_accepts_closure_returning_vec_string`. |
| 3 | `Key::from_fn(|item: &T| item.field.as_str())` constructs a single-value key | Met | `Key::from_fn` at `src/key.rs:125-135`. HRTB elision is correct: `Fn(&T) -> &str` expands to `for<'a> Fn(&'a T) -> &'a str`. Converts to owned string internally. Tested by `from_fn_single_value_extraction`. |
| 4 | `Key::from_fn_multi(|item: &T| vec![...])` constructs a multi-value key | Met | `Key::from_fn_multi` at `src/key.rs:159-169`. Same correct HRTB as `from_fn`. Tested by `from_fn_multi_extracts_multiple_values`. |
| 5 | Builder chain `Key::new(...).threshold(r).max_ranking(r).min_ranking(r)` compiles and sets fields | Met | All three builder methods at lines 191, 219, 246, all annotated `#[must_use]`. Tested by `builder_chain_all_three` and `builder_chain_preserves_extractor`. |
| 6 | `RankingInfo` struct has fields: `rank: f64`, `ranked_value: String`, `key_index: usize`, `key_threshold: Option<Ranking>` | Partial | Struct present at `src/key.rs:314`. `rank` field uses `Ranking` enum instead of `f64` as the ticket specifies. Deviation is deliberate and actually better (type-safe), but diverges from the ticket's stated AC. Impl report incorrectly attributes this choice to "the ticket specifies" -- the ticket says `f64`, the PRD says `f64`. |
| 7 | All public items have doc comments | Met | All public items (`Key<T>`, `RankingInfo`, `Key::new`, `Key::from_fn`, `Key::from_fn_multi`, `Key::threshold`, `Key::max_ranking`, `Key::min_ranking`, `Key::extract`, `Key::threshold_value`, `Key::max_ranking_value`, `Key::min_ranking_value`) have full doc comments with `# Arguments`, `# Returns`, and `# Examples`. |
| 8 | `cargo build` no warnings; `cargo clippy -- -D warnings` clean | Met | Verified: `cargo build` completes with zero warnings; `cargo clippy -- -D warnings` exits clean; `cargo fmt --check` passes; 123 tests all pass. |

---

## Issues Found

### Critical (must fix before merge)
None.

### Major (should fix, risk of downstream problems)

- **AC 1 / AC 6 deviations: `Vec<String>` and `Ranking` instead of `Vec<Cow<'_, str>>` and `f64`.**
  The ticket AC 1 specifies `Box<dyn Fn(&T) -> Vec<Cow<'_, str>>>` and AC 6 specifies `rank: f64`.
  The implementation uses `Vec<String>` and `rank: Ranking` respectively.

  - `Vec<String>` vs `Vec<Cow<'_, str>>`: The `Cow` approach was intended to allow zero-copy
    extraction when items borrow string data (e.g., `Key::from_fn(|u| u.name.as_str())`). Using
    `Vec<String>` means `from_fn` and `from_fn_multi` always allocate an owned `String` even when
    a borrow would suffice. This is a performance regression compared to the design intent. More
    importantly, Ticket 2 expects to return `Vec<Cow<'_, str>>` from `get_item_values` -- if the
    extractor already returns `Vec<String>`, `get_item_values` will either need to wrap each in
    `Cow::Owned` (no saving) or change its contract. This creates a downstream contract mismatch.

  - `rank: Ranking` vs `rank: f64`: This is actually the stronger choice -- using the enum is
    more type-safe and avoids lossy conversion. However, it does diverge from the ticket as
    written. If the orchestrator/ticket owner is comfortable with this change (and the PRD is
    updated accordingly), it is strictly preferable. The impl report incorrectly says "the ticket
    specifies" `Ranking` -- the ticket specifies `f64`. This misattribution should be corrected
    for clarity.

  Neither deviation causes a compile error or logic bug in this ticket's scope, but the `Vec<String>`
  deviation will ripple into Ticket 2's design. The downstream implementer should be aware.

  **Recommended action:** The orchestrator should decide whether to accept `Vec<String>` (updating
  Ticket 2's scope accordingly) or request a change to `Vec<Cow<'_, str>>` before Ticket 2 begins.
  The `rank: Ranking` choice should be locked in as an accepted improvement over the ticket spec.

### Minor (nice to fix, not blocking)

- **`Extractor<T>` is not `pub` but `#[warn(missing_docs)]` is set on the crate.** The type alias
  `Extractor<T>` at `src/key.rs:17` is module-private, which is correct since clippy only fires
  `missing_docs` on `pub` items. No action needed -- this is fine.

- **`Key<T>` does not implement `Send`.** The extractor is `Box<dyn Fn(&T) -> Vec<String>>` without
  a `Send` bound. If downstream code needs to use `Key<T>` across thread boundaries (e.g., parallel
  ranking via rayon), it will fail to compile. CLAUDE.md mentions rayon for CPU-bound parallelism.
  Consider `Box<dyn Fn(&T) -> Vec<String> + Send + Sync>` if threading is expected. This is a
  design concern for PRD-003, not a blocker for this ticket.

- **Impl report incorrectly claims "the ticket specifies" `Ranking` for `RankingInfo.rank`.** The
  ticket AC 6 explicitly says `rank: f64`. The impl report appears to have confused the ticket with
  the PRD discussion. This is a documentation inaccuracy in the report, not a code defect.

---

## Suggestions (non-blocking)

- The `threshold_value`, `max_ranking_value`, and `min_ranking_value` accessor methods return
  references (`Option<&Ranking>` / `&Ranking`). Since `Ranking` is a small enum (plus one `f64`
  variant), returning by value with `Clone` would be more ergonomic for callers. That said,
  returning references is idiomatic Rust and avoids unnecessary copies if `Ranking` grows -- so
  this is a style preference, not a defect.

- The `Extractor<T>` type alias is module-private. If downstream sibling modules (e.g., future
  `extraction.rs`) need to reference the extractor type, they would have to repeat the full type
  or depend on the `Key::extract` method. The current design using `Key::extract` as the public
  interface is the right call -- no change needed.

- The test `builder_matches_variant_in_threshold` at `src/key.rs:512` uses `Ranking::Matches(1.5)`
  and similar sub-scores. `PartialEq` on `Matches(f64)` is NaN-sensitive (the existing `PartialEq`
  impl from `ranking/mod.rs` uses `f64` comparison). This test is correct as written, but it's
  worth noting for future tests involving NaN sub-scores.

---

## Scope Check

- Files within scope: YES
  - `src/key.rs` -- created (in scope)
  - `src/lib.rs` -- modified (in scope)
- Scope creep detected: NO -- Additional accessor methods (`threshold_value`, `max_ranking_value`,
  `min_ranking_value`, `extract`) are minimal additions that serve the stated purpose of avoiding
  dead-code warnings and enabling downstream ticket use. These are reasonable additions within the
  ticket's spirit.
- Unauthorized dependencies added: NO -- No new crate dependencies added.

---

## Risk Assessment

- **Regression risk: LOW.** No existing functionality modified other than adding `pub mod key;` and
  a re-export to `src/lib.rs`. All 123 tests (92 unit + 18 integration + 13 doc-tests) pass. The
  existing ranking module is untouched.

- **Security concerns: NONE.** No I/O, no unsafe, no external inputs, no credentials.

- **Performance concerns: LOW (noted, not blocking).** The `Vec<String>` extractor design allocates
  on every extraction call where a borrow would suffice. This is a design-level concern for
  downstream tickets (particularly Ticket 2), not a runtime bug. For a CRM/UI workload this would
  be negligible; for high-throughput batch ranking it could matter. PRD-002 section 6 explicitly
  calls for `Cow` to enable zero-copy paths.

# Code Review: Ticket 5 -- `get_closeness_ranking` -- Fuzzy Character-by-Character Scorer

**Ticket:** 5 -- `get_closeness_ranking` -- Fuzzy Character-by-Character Scorer
**Impl Report:** docs/001-core-ranking-engine-reports/ticket-05-impl.md
**Date:** 2026-02-20 19:00
**Verdict:** APPROVED

---

## AC Coverage

| AC # | Description | Status | Notes |
|------|-------------|--------|-------|
| 1 | `get_closeness_ranking("playground", "plgnd")` returns `Ranking::Matches(s)` where `1.0 < s < 2.0` | Met | Test `closeness_fuzzy_match_playground` confirms positions p=0,l=1,g=4,n=8,d=9; spread=9; score=1.0+1/9=~1.111. Also verified by passing doctest. |
| 2 | `get_closeness_ranking("abc", "xyz")` returns `Ranking::NoMatch` | Met | Test `closeness_no_match` covers this case. First unmatched char causes immediate `return Ranking::NoMatch`. |
| 3 | `get_closeness_ranking("ab", "a")` returns `Ranking::Matches(2.0)` (spread = 0, single char hit) | Met | Test `closeness_single_char_match` confirms this. `first_match_index = Some(0)`, `last_match_index = 0`, `spread = 0`, returns `Ranking::Matches(2.0)`. Also covered by doctest. |
| 4 | Return value for any `Matches` case is in the half-open range `(1.0, 2.0]` | Met | Score formula `1.0 + 1.0 / spread` where `spread >= 1` gives range `(1.0, 2.0)`. The `spread == 0` case explicitly returns `2.0`, so full range is `(1.0, 2.0]`. Test `closeness_result_always_in_range` verifies across 4 parametric cases. |
| 5 | No `unsafe` blocks | Met | Full file reviewed -- no `unsafe` keyword anywhere in the new code or tests. |
| 6 | `cargo test` passes for unit tests in this function | Met | All 42 unit tests and 3 doctests pass. `cargo clippy -- -D warnings` clean. `cargo fmt --check` clean. `cargo build` zero warnings. |

## Issues Found

### Critical (must fix before merge)
None.

### Major (should fix, risk of downstream problems)
None.

### Minor (nice to fix, not blocking)

- **Empty-query behavior is semantically questionable** (`src/ranking/mod.rs`, line 176-186): An empty query returning `Ranking::Matches(2.0)` is the highest possible `Matches` sub-score, meaning an empty query ranks a candidate identically to a perfect single-character match. This is consistent with the JS reference implementation's `1/0 = Infinity` approach, and the impl report explicitly notes this as intentional. However, the calling convention for `get_closeness_ranking` (which will be invoked from `get_match_ranking`) should guard against passing empty queries at a higher level, or callers should be aware this is a degenerate case. This is not a bug given the documented intent -- just worth noting for future ticket implementers.

- **`closeness_result_always_in_range` test comment is slightly inaccurate** (`src/ranking/mod.rs`, line 625): The comment on the `("abcdefghijklmnop", "abop")` case notes `// spread = 15`, which is correct (`a=0`, `p=15`), but the meaning may confuse readers who expect `spread` to reflect the entire matched-character span rather than just the first-to-last positions. The comment is technically right but could clarify that `first=0` (from `a`) and `last=15` (from `p`), not `first=0` and `last=3` (from the `ab` run). This is a documentation nit only.

## Suggestions (non-blocking)

- The doc comment at line 128 says "`[`Ranking::Matches(2.0)`]` when `spread == 0` (single-character query)" -- this parenthetical could also note "or empty query" to be fully accurate, since `query = ""` also produces this path.

- The `closeness_result_always_in_range` test is a good invariant check, but none of the parametric cases exercise `spread == 0` returning exactly `2.0`. A case like `("x", "x")` would complete coverage of the closed upper bound within that test. The `closeness_single_char_match` and `closeness_empty_query` tests already cover this separately, so this is a low-priority observation.

- The `.chars().enumerate()` approach produces char-index positions (not byte offsets), which is correct and intentional. This means spread is measured in Unicode scalar values. For future reviewers: if the caller uses byte-level indexing elsewhere in the pipeline, this distinction will matter.

## Scope Check

- Files within scope: YES -- only `src/ranking/mod.rs` was modified, which is the sole file listed in the ticket scope.
- Scope creep detected: NO
- Unauthorized dependencies added: NO -- no changes to `Cargo.toml`; no new crates introduced.

## Risk Assessment

- Regression risk: LOW -- the implementation is purely additive. The new `get_closeness_ranking` function is appended to the module; no existing code was modified. All 42 pre-existing unit tests and 3 doctests continue to pass.
- Security concerns: NONE -- pure string processing with no I/O, no allocations, no external input that could cause injection or overflow.
- Performance concerns: NONE -- O(n * m) where n = `candidate` length and m = `query` length. No allocations. Uses Rust's lazy iterator protocol (`find()` on a peekable `Enumerate`); no unnecessary work. For CRM-scale or search-box use cases this is entirely appropriate.

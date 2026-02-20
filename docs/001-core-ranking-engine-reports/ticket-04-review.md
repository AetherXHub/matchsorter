# Code Review: Ticket 4 -- `get_acronym` -- Word-Boundary Acronym Extraction

**Ticket:** 4 -- `get_acronym` -- Word-Boundary Acronym Extraction
**Impl Report:** docs/001-core-ranking-engine-reports/ticket-04-impl.md
**Date:** 2026-02-20 17:15
**Verdict:** APPROVED

---

## AC Coverage

| AC # | Description | Status | Notes |
|------|-------------|--------|-------|
| 1 | `get_acronym("north-west airlines")` returns `"nwa"` | Met | Verified by `acronym_hyphen_and_space` test and doc-test; confirmed correct by manual trace of algorithm |
| 2 | `get_acronym("san francisco")` returns `"sf"` | Met | Verified by `acronym_space_only` test |
| 3 | `get_acronym("single")` returns `"s"` | Met | Verified by `acronym_single_word` test |
| 4 | `get_acronym("")` returns `""` | Met | Verified by `acronym_empty_string` test; early-return path at line 153 confirmed correct |
| 5 | Underscores do NOT act as word boundaries | Met | Verified by `acronym_underscores_not_delimiters` test: `get_acronym("snake_case_word")` returns `"s"`. Only `' '` and `'-'` are delimiters in `is_acronym_delimiter`. |
| 6 | `cargo test` passes for unit tests in this function | Met | All 30 unit tests and 2 doc-tests pass. `cargo clippy -- -D warnings`, `cargo fmt --check`, and `cargo build` all clean. |

## Issues Found

### Critical (must fix before merge)
None.

### Major (should fix, risk of downstream problems)
None.

### Minor (nice to fix, not blocking)

1. **`get_acronym` pushes delimiter chars when input starts with a delimiter** (`src/ranking/mod.rs`, line 162). When `s = " hello world"`, the result is `" hw"` (a leading space followed by word initials). The PRD specifies "first char is always included," which the implementation faithfully follows, so this is not a spec violation. However, the `# Returns` doc comment states "A `String` containing the first character of each word" — a leading space or hyphen is not a word-initial character, making that doc line slightly misleading. This edge case is outside the ticket's ACs and unlikely in practice (candidate strings are lowercased natural-language tokens, not leading-delimiter strings). If it matters for correctness in Ticket 6, the `get_match_ranking` caller should strip or normalize candidates before passing them to `get_acronym`.

2. **Capacity estimate overcounts for consecutive delimiters** (`src/ranking/mod.rs`, line 158). For `"hello  world"` (two spaces), `memchr2_iter` counts 2 delimiters, so `word_count_estimate` is 3, but the actual acronym is `"hw"` (length 2). This is a standard acceptable over-allocation — the comment correctly describes it as an estimate, and the over-allocation is bounded by the delimiter count. Not a bug; noted for completeness.

3. **Impl report test count is inaccurate.** The report claims "20 unit tests (10 existing + 10 new)" but the actual total is 30 unit tests (10 existing `Ranking` tests from Ticket 2, 10 `prepare_value_for_comparison` tests from Ticket 3, and 10 new acronym tests from this ticket). All 30 tests pass; the discrepancy is purely in the report's accounting.

## Suggestions (non-blocking)

- The `# Returns` doc comment on `get_acronym` (line 135) could be tightened to match the actual semantics: "A `String` containing the first character of each delimited word, where word boundaries are space and hyphen. If the string begins with a delimiter, that delimiter is included as the first character." This avoids any ambiguity for future callers.

- The private `is_acronym_delimiter` function has a doc comment (lines 114-116) but no `# Examples` section. This is fine for a private helper under `#![warn(missing_docs)]` (the lint only fires for public items), but the existing doc comment is a good template.

## Scope Check

- Files within scope: YES — only `src/ranking/mod.rs` was modified, which is the sole file listed in Ticket 4's scope.
- Scope creep detected: NO
- Unauthorized dependencies added: NO — `memchr` was already listed in `Cargo.toml` from Ticket 1.

## Risk Assessment

- **Regression risk: LOW** — The new code appends after the existing `PartialOrd` impl block. The only shared state is the `#[cfg(test)]` module where new tests were added; existing tests are unmodified. All 30 tests pass.
- **Security concerns: NONE** — Pure string manipulation with no I/O, no allocations beyond the returned `String`, no external input surfaces.
- **Performance concerns: NONE** — Single-pass `O(n)` char iterator; `memchr2_iter` is used for an efficient byte-count pass to pre-size the allocation. No unnecessary heap allocations. The capacity overestimate for consecutive delimiters is bounded and inconsequential.

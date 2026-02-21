# Tickets for PRD 004: Performance Hot-Path Optimizations

**Source PRD:** docs/PRD-004-performance-hot-path-optimizations.md
**Created:** 2026-02-21
**Total Tickets:** 4
**Estimated Total Complexity:** 7 (S=1, M=2, L=3: Ticket 1=M, Ticket 2=M, Ticket 3=S, Ticket 4=M → 2+2+1+2=7)

---

### Ticket 1: Diacritics Early-Exit in `prepare_value_for_comparison`

**Description:**
Add a combining-mark pre-scan to `prepare_value_for_comparison` in `src/ranking/mod.rs` so that
non-ASCII strings with no combining marks return `Cow::Borrowed` immediately without any heap
allocation or NFD decomposition. The existing post-collect equality check (`if stripped == s`) is
then redundant and must be removed, since this branch is only reached when a combining mark was
confirmed present. Update inline doc comments and the existing unit test that documents CJK
now returning `Cow::Borrowed` without a full NFD round-trip.

**Scope:**
- Modify: `src/ranking/mod.rs` — function `prepare_value_for_comparison` (add early-exit before the
  `s.nfd().filter(...).collect()` line; remove the trailing `if stripped == s` equality check)
- No new files. No changes to `src/lib.rs`, benchmarks, or any other module.

**Acceptance Criteria:**
- [ ] `prepare_value_for_comparison("世界", false)` returns `Cow::Borrowed` (verified via the
  `returns_borrowed_for_non_ascii_without_diacritics` unit test, which must now pass without the
  NFD round-trip — add a `// Early-exit path` comment in the test to document the distinction).
- [ ] `prepare_value_for_comparison("caf\u{00E9}", false)` returns `Cow::Owned("cafe")` — the
  existing `strips_precomposed_accent` test continues to pass.
- [ ] `prepare_value_for_comparison("cafe\u{0301}", false)` returns `Cow::Owned("cafe")` — the
  existing `strips_combining_acute_accent` test continues to pass.
- [ ] `prepare_value_for_comparison("cafe", false)` returns `Cow::Borrowed` — existing ASCII
  test continues to pass.
- [ ] The post-collect `if stripped == s` branch is removed from the function body (verify with
  `grep -n 'stripped == s' src/ranking/mod.rs` producing no output).
- [ ] `cargo test -p matchsorter -- ranking` passes with zero failures.
- [ ] `cargo clippy -- -D warnings` produces zero warnings.

**Dependencies:** None
**Complexity:** M
**Maps to PRD AC:** AC 1, AC 2, AC 3, AC 4, AC 11

---

### Ticket 2: Already-Lowercase Early-Exit in `lowercase_into`

**Description:**
Add an already-lowercase fast path to `lowercase_into` in `src/ranking/mod.rs` for both the ASCII
and non-ASCII branches. When all characters are already lowercase, bulk-copy the original string
into `buf` with `buf.push_str(s)` instead of iterating per-character. Add targeted unit tests for
`lowercase_into` directly (it is currently tested only implicitly via `get_match_ranking`).

**Scope:**
- Modify: `src/ranking/mod.rs` — function `lowercase_into` (add `all(|b| !b.is_ascii_uppercase())`
  guard for the ASCII branch and `all(|c| !c.is_uppercase())` guard for the non-ASCII branch,
  each followed by `buf.push_str(s)` on the fast path)
- No new files. No changes to `src/lib.rs` or any other module.

**Acceptance Criteria:**
- [ ] `lowercase_into("hello world", &mut buf)` produces `"hello world"` in `buf`; on a second
  call with the same `buf` (already-capacitated), no reallocation occurs — confirmed by an
  explicit test that asserts `buf == "hello world"` after calling the function.
- [ ] `lowercase_into("Hello World", &mut buf)` still produces `"hello world"` — the
  mixed-case path is not regressed; test asserts `buf == "hello world"`.
- [ ] `lowercase_into("café", &mut buf)` (pre-lowercased non-ASCII) produces `"café"` in `buf`
  and the function takes the fast path — test asserts `buf == "café"`.
- [ ] `lowercase_into("Üniversität", &mut buf)` (non-ASCII with uppercase) still correctly
  lowercases to `"üniversität"`.
- [ ] All existing `ranking` unit tests continue to pass (`cargo test -p matchsorter -- ranking`).
- [ ] `cargo clippy -- -D warnings` produces zero warnings.

**Dependencies:** Ticket 1 (same file; apply after Ticket 1 is merged to avoid conflicts)
**Complexity:** M
**Maps to PRD AC:** AC 1, AC 5, AC 6, AC 7, AC 11

---

### Ticket 3: Pre-Allocate `candidate_buf` in `match_sorter`

**Description:**
Change the `candidate_buf` initialization in `match_sorter` in `src/lib.rs` from
`String::new()` (zero capacity) to `String::with_capacity(value.len().max(32))` so that the
first `lowercase_into` call does not trigger a grow-from-zero heap reallocation. Update the
accompanying doc comment on that line to explain the heuristic.

**Scope:**
- Modify: `src/lib.rs` — single line change at `let mut candidate_buf = String::new();`
  (update to `String::with_capacity(value.len().max(32))`) plus its inline comment.
- No new files. No changes to `src/ranking/mod.rs` or benchmarks.

**Acceptance Criteria:**
- [ ] The line `String::new()` no longer appears adjacent to `candidate_buf` in `src/lib.rs`
  (verify with `grep -n 'candidate_buf = String::new' src/lib.rs` producing no output).
- [ ] `String::with_capacity(value.len().max(32))` is the initializer and an explanatory doc
  comment is present on the preceding line.
- [ ] All existing `lib` unit tests continue to pass (`cargo test -p matchsorter` — no failures).
- [ ] `cargo clippy -- -D warnings` produces zero warnings.
- [ ] `cargo fmt --check` passes.

**Dependencies:** Tickets 1 and 2 are independent of this ticket; Ticket 3 can be applied at any
point after Ticket 1 is in place. Sequenced after Ticket 2 here for clean linear application.
**Complexity:** S
**Maps to PRD AC:** AC 1, AC 12

---

### Ticket 4: Verification and Benchmark Validation

**Description:**
Run the full PRD-004 acceptance criteria checklist end-to-end. Execute the Criterion benchmark
suite and the JS head-to-head comparison script to confirm the diacritics speedup >= 4x and
overall geometric mean >= 7x over JS. Confirm no regressions in the existing test suite and that
all lint/format checks pass cleanly.

**Scope:**
- No code changes expected. If any check below fails, fix the root cause in the relevant file
  (`src/ranking/mod.rs` or `src/lib.rs`) before marking this ticket done.
- Read: `benches/benchmarks.rs`, `bench-compare/run.sh` (reference only — no edits).

**Acceptance Criteria:**
- [ ] `cargo test` passes with zero failures across all test targets.
- [ ] `cargo clippy -- -D warnings` produces zero warnings.
- [ ] `cargo fmt --check` passes with no formatting violations.
- [ ] `cargo bench --bench benchmarks -- diacritics` shows the `strip_diacritics` median time
  is at least 40% lower than the pre-PR-004 baseline (AC 8).
- [ ] `bench-compare/run.sh` output shows the `Diacritics (10k) strip` row speedup >= 4.0x
  over JS (AC 9).
- [ ] `bench-compare/run.sh` output shows the overall geometric mean speedup >= 7.0x over JS
  (AC 10).
- [ ] `grep -n 'stripped == s' src/ranking/mod.rs` returns no output (post-collect equality
  check is gone).
- [ ] `grep -n 'candidate_buf = String::new' src/lib.rs` returns no output (zero-cap init
  is gone).

**Dependencies:** Tickets 1, 2, and 3
**Complexity:** M
**Maps to PRD AC:** AC 1, AC 8, AC 9, AC 10, AC 11, AC 12

---

## AC Coverage Matrix

| PRD AC # | Description                                                                                    | Covered By Ticket(s)    | Status  |
|----------|------------------------------------------------------------------------------------------------|-------------------------|---------|
| 1        | `cargo test` passes; existing coverage for all three changed functions continues to pass       | Ticket 1, 2, 3, 4       | Covered |
| 2        | `prepare_value_for_comparison("世界", false)` returns `Cow::Borrowed` without NFD allocation  | Ticket 1                | Covered |
| 3        | `prepare_value_for_comparison("caf\u{00E9}", false)` returns `Cow::Owned("cafe")`             | Ticket 1                | Covered |
| 4        | `prepare_value_for_comparison("cafe\u{0301}", false)` returns `Cow::Owned("cafe")`            | Ticket 1                | Covered |
| 5        | `lowercase_into("hello world", buf)` produces correct output with zero heap allocs on reuse   | Ticket 2                | Covered |
| 6        | `lowercase_into("Hello World", buf)` still produces `"hello world"` correctly                 | Ticket 2                | Covered |
| 7        | `lowercase_into` with pre-lowercased non-ASCII string produces correct result via fast path    | Ticket 2                | Covered |
| 8        | `cargo bench -- diacritics` shows strip_diacritics median at least 40% faster than baseline   | Ticket 4                | Covered |
| 9        | `bench-compare/run.sh` diacritics row speedup >= 4.0x over JS                                 | Ticket 4                | Covered |
| 10       | `bench-compare/run.sh` overall geometric mean speedup >= 7.0x over JS                         | Ticket 4                | Covered |
| 11       | `cargo clippy -- -D warnings` produces zero warnings                                          | Ticket 1, 2, 4          | Covered |
| 12       | `cargo fmt --check` passes with no formatting violations                                       | Ticket 3, 4             | Covered |

# PRD-004: Performance Hot-Path Optimizations

**Status:** TICKETS READY
**Created:** 2026-02-20
**Author:** PRD Writer Agent

---

## Problem Statement

The Rust `matchsorter` crate is already significantly faster than the JavaScript reference implementation for most workloads, but three measurable inefficiencies remain in the hot path. The diacritics path runs only 2.4-2.6x faster than JS (versus a 5.6x geometric mean overall), because `prepare_value_for_comparison` always runs a full NFD decomposition and heap allocation on any non-ASCII input even though most real-world candidates have no combining marks at all. Separately, `lowercase_into` always iterates through every character even when the string is already fully lowercase, adding unnecessary per-candidate work. Finally, `match_sorter` initializes `candidate_buf` with `String::new()` (zero capacity), causing a guaranteed first-element reallocation on every call site regardless of input size.

## Goals

- Eliminate NFD + heap allocation in `prepare_value_for_comparison` for non-ASCII strings that contain no combining marks, covering 90%+ of real-world candidates.
- Eliminate the character-iteration cost in `lowercase_into` when a candidate is already fully lowercase.
- Eliminate the first-element grow-reallocation in `match_sorter`'s `candidate_buf` by pre-allocating a sensible initial capacity.
- Raise the diacritics benchmark speedup from ~2.5x to >= 4x over JS.
- Raise the overall geometric mean speedup from ~5.6x to >= 7x over JS.

## Non-Goals

- Parallelism via `rayon` or any multi-threaded ranking (out of scope; evaluate separately).
- Algorithmic changes to the ranking tiers themselves (no tier behavior changes).
- Changes to the public API surface (`match_sorter`, `get_match_ranking`, `MatchSorterOptions`, `Ranking`).
- Python or WASM bindings.
- Locale-aware case folding (current `to_lowercase` / `to_ascii_lowercase` behavior is preserved exactly).
- SIMD intrinsics or `unsafe` code; all optimizations must use safe Rust.

## User Stories

- As a library user processing a large candidate list (10k+ items) with `keep_diacritics: false`, I want the ranking loop to avoid heap allocations for candidates that have no diacritics, so that throughput improves without any API change on my side.
- As a library user whose candidates are already lowercase (e.g. normalized database fields), I want `match_sorter` to skip redundant character iteration during the lowercasing step, so that I get measurably lower latency per call.
- As a library user calling `match_sorter` repeatedly in a hot loop (e.g. a UI search-as-you-type handler), I want the internal candidate buffer to not trigger a heap reallocation on the very first item of every call, so that overall allocation pressure is reduced.

## Technical Approach

All three changes are localized to two files: `src/ranking/mod.rs` and `src/lib.rs`. No new modules, traits, or public API symbols are introduced. The changes follow existing patterns in the codebase (Cow-returning helpers, reusable buffers, ASCII fast paths).

### Change 1: Diacritics early-exit in `prepare_value_for_comparison`

**File:** `src/ranking/mod.rs`, function `prepare_value_for_comparison`

**Current behavior:** After the existing ASCII fast path, any non-ASCII string immediately enters `s.nfd().filter(...).collect::<String>()`. This allocates a new `String` even when the non-ASCII content has no combining marks (e.g. CJK characters, emoji, plain Latin letters stored as precomposed NFC without accents that need stripping).

**New behavior:** Before invoking NFD decomposition, scan the raw bytes of `s` for characters in the Unicode combining mark range. Combining marks in Unicode start at U+0300 (first combining diacritical, 0xCC 0x80 in UTF-8) and the densest block ends around U+036F. A second significant block is U+1AB0–U+1AFF and a third is U+1DC0–U+1DFF; however, for a practical early-exit it is sufficient to scan for any byte sequence whose leading byte indicates a code point >= U+0300. The UTF-8 encoding of U+0300 is `0xCC 0x80`; any two-byte sequence with a leading byte of `0xCC` or higher (that is part of a combining block) indicates a potential combining mark.

A simpler and fully correct approach: iterate over the chars of `s` once with `s.chars().any(is_combining_mark)`. If no combining mark is found, the NFD-filtered result is guaranteed to equal `s`, so return `Cow::Borrowed(s)` immediately without allocating. Only when a combining mark is detected do we fall through to the existing NFD + collect path.

The existing post-collect equality check (`if stripped == s`) is correct but too late — it allocates first, then discards. The early-exit moves this check before the allocation.

```rust
// After the ASCII fast path:

// Early-exit: if no combining marks exist in the string, NFD+filter
// would produce the same bytes, so we can skip the allocation entirely.
// This covers the common case of non-ASCII strings without diacritics
// (CJK, emoji, plain NFC Latin without accent marks, etc.).
if !s.chars().any(is_combining_mark) {
    return Cow::Borrowed(s);
}

// Combining marks detected: run NFD decomposition and strip them.
let stripped: String = s.nfd().filter(|c| !is_combining_mark(*c)).collect();
Cow::Owned(stripped)
```

Note: the post-collect equality check can be removed because if we reach this branch, we know at least one combining mark exists and stripping will change the string. The `Cow::Owned` return is always correct here.

### Change 2: Already-lowercase early-exit in `lowercase_into`

**File:** `src/ranking/mod.rs`, function `lowercase_into`

**Current behavior:** After `buf.clear()`, the function always iterates every character (ASCII byte or Unicode char) to produce a lowercase copy.

**New behavior:** Before iterating, check whether `s` is already entirely lowercase. For the ASCII branch, this is `s.as_bytes().iter().all(|b| !b.is_ascii_uppercase())`. For the non-ASCII branch, this is `s.chars().all(|c| !c.is_uppercase())`. If the string is already lowercase, push the original string directly into `buf` without per-character mapping.

For the ASCII branch:

```rust
if s.is_ascii() {
    buf.reserve(s.len());
    if s.as_bytes().iter().all(|b| !b.is_ascii_uppercase()) {
        // Already lowercase: bulk-copy avoids per-byte case mapping.
        buf.push_str(s);
    } else {
        buf.extend(s.as_bytes().iter().map(|&b| b.to_ascii_lowercase() as char));
    }
}
```

For the non-ASCII branch:

```rust
} else {
    buf.reserve(s.len());
    if s.chars().all(|c| !c.is_uppercase()) {
        buf.push_str(s);
    } else {
        for c in s.chars() {
            for lc in c.to_lowercase() {
                buf.push(lc);
            }
        }
    }
}
```

The `all(...)` scan is O(n) in the worst case (a string that has an uppercase letter at the end), but for already-lowercase strings it short-circuits as soon as it confirms all chars are lowercase — which is the common case for pre-normalized data. For strings with an early uppercase letter the cost is comparable to the current iterate-and-map approach and the branch may be omitted by the optimizer.

### Change 3: Pre-allocate `candidate_buf` in `match_sorter`

**File:** `src/lib.rs`, function `match_sorter`

**Current behavior:**
```rust
let mut candidate_buf = String::new();
```

This has zero capacity. The first call to `lowercase_into` (which does `buf.reserve(s.len())` then `push_str` or character iteration) causes a heap allocation from zero.

**New behavior:** Pre-allocate the buffer with a capacity heuristic based on the input. A reasonable starting capacity is the length in bytes of the query string (`value.len()`), since candidates that reach the lowercasing step have at least as many characters as the query. A better heuristic is the median expected candidate length; since that is unknown, using `value.len().max(32)` as the initial capacity is a safe, cheap choice that eliminates the grow-from-zero reallocation in the common case while avoiding over-allocation for tiny queries.

```rust
// Pre-allocate with a capacity heuristic to avoid grow-on-first-use.
// `value.len().max(32)` covers most realistic candidate lengths without
// over-allocating for short queries.
let mut candidate_buf = String::with_capacity(value.len().max(32));
```

### Files changed

| File | Change |
|---|---|
| `src/ranking/mod.rs` | Add combining-mark early-exit before NFD in `prepare_value_for_comparison`; remove now-redundant post-collect equality check |
| `src/ranking/mod.rs` | Add already-lowercase early-exit in `lowercase_into` for both ASCII and non-ASCII branches |
| `src/lib.rs` | Change `String::new()` to `String::with_capacity(value.len().max(32))` for `candidate_buf` |

### Benchmark validation

The existing `bench_diacritics` group in `benches/benchmarks.rs` and the `bench-compare/run.sh` head-to-head script are the primary validation tools. No new benchmark groups are required, though the `bench_get_match_ranking` group already exercises the `prepare_value_for_comparison` path and will show improvement automatically.

## Acceptance Criteria

1. `cargo test` passes with no failures after all three changes are applied; the existing test coverage for `prepare_value_for_comparison`, `lowercase_into`, and `match_sorter` continues to pass without modification.
2. `prepare_value_for_comparison("世界", false)` returns `Cow::Borrowed` (non-ASCII, no combining marks — currently returns `Cow::Borrowed` only after a full NFD round-trip; must now return borrowed without any allocation).
3. `prepare_value_for_comparison("caf\u{00E9}", false)` still returns `Cow::Owned("cafe")` (combined mark detected, stripping proceeds correctly).
4. `prepare_value_for_comparison("cafe\u{0301}", false)` still returns `Cow::Owned("cafe")` (explicit combining mark, correctly stripped).
5. `lowercase_into("hello world", &mut buf)` produces `"hello world"` in `buf` and, when measured with a profiler or allocation counter, performs zero heap allocations on the second and subsequent calls to the same `buf` (the buffer's capacity is already sufficient and no `reserve` reallocation occurs).
6. `lowercase_into("Hello World", &mut buf)` still produces `"hello world"` correctly (uppercase-present path is not regressed).
7. `lowercase_into` with a non-ASCII already-lowercase string (e.g. `"café"` pre-lowercased) produces the correct lowercase copy without calling `to_lowercase` per character.
8. Running `cargo bench --bench benchmarks -- diacritics` shows the `strip_diacritics` result is at least 40% faster (lower median time) than the baseline recorded before this PRD's changes.
9. Running `bench-compare/run.sh` shows the `Diacritics (10k) strip` row's speedup is >= 4.0x over JS (up from ~2.5x baseline).
10. Running `bench-compare/run.sh` shows the overall geometric mean speedup is >= 7.0x over JS (up from ~5.6x baseline).
11. `cargo clippy -- -D warnings` produces zero warnings after all changes.
12. `cargo fmt --check` passes with no formatting violations.

## Open Questions

- The `all(|c| !c.is_uppercase())` scan for non-ASCII already-lowercase detection costs an extra O(n) pass over the string before the actual lowercasing pass. For strings that are mixed-case, this doubles the work. If profiling shows this to be a net regression on mixed-case non-ASCII workloads, the non-ASCII already-lowercase branch can be dropped and only the ASCII branch retained; the diacritics early-exit (change 1) is the dominant win regardless.
- The `value.len().max(32)` heuristic for `candidate_buf` initial capacity is an estimate. If benchmarks show the typical candidate is significantly longer (e.g. long document excerpts), a larger constant (e.g. 64) may reduce subsequent `reserve` calls. This can be tuned after the initial benchmark run.

## Dependencies

- `unicode_normalization::char::is_combining_mark` — already imported in `src/ranking/mod.rs` (used in the existing filter); no new crate dependencies.
- `unicode_normalization::UnicodeNormalization` — already imported; still needed for the `s.nfd()` call on the combining-mark-detected path.
- `memchr` — already in `Cargo.toml`; unchanged.
- Existing Criterion benchmark suite in `benches/benchmarks.rs` and `bench-compare/run.sh` — used for validation, unchanged structurally.

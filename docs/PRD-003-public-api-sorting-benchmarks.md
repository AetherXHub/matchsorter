# PRD-003: Public API, Sorting, and Benchmarks

**Status:** TICKETS READY

## Overview

Implement the top-level `match_sorter()` function, the sorting/filtering pipeline, and comprehensive benchmarks. This is the user-facing API that ties together the ranking engine (PRD-001) and key extraction (PRD-002) into a complete, production-ready crate.

## Background

The JS `matchSorter(items, value, options?)` function is the sole entry point. It ranks every item, filters by threshold, sorts by rank/key-index/base-sort, and returns the filtered+sorted list. The Rust port must replicate this pipeline with idiomatic Rust ergonomics.

## Goals

- Implement the complete `match_sorter()` public API
- Implement sorting with the three-level comparator (rank, key index, base sort)
- Implement the `threshold` filter
- Implement `base_sort` customization
- Implement `sorter` override
- Provide `MatchSorterOptions` configuration struct
- Benchmark against realistic workloads and optimize hot paths
- Ensure the crate is `lib` (not binary), with a clean public API

## Non-Goals

- Python or WASM bindings (future work)
- Async/parallel ranking (evaluate if beneficial in benchmarks, but not required)

## Detailed Design

### 1. Crate Structure

Convert the project from a binary to a library crate:

```
src/
  lib.rs          -- public API, re-exports
  ranking.rs      -- ranking tiers and get_match_ranking (PRD-001)
  key.rs          -- Key type and value extraction (PRD-002)
  diacritics.rs   -- accent/diacritic removal
  sort.rs         -- sorting logic
benches/
  benchmarks.rs   -- criterion benchmarks
tests/
  integration.rs  -- integration tests matching JS test suite
```

### 2. The `match_sorter` Function

```rust
/// Filter and sort items by how well they match a search query.
///
/// This is the main entry point for the library. It ranks every item
/// against the query, filters out items below the threshold, and sorts
/// the remaining items by match quality.
///
/// # Arguments
///
/// * `items` - Slice of items to search through
/// * `value` - The search query string
/// * `options` - Configuration options (threshold, keys, sorting, etc.)
///
/// # Returns
///
/// A new `Vec<&T>` containing references to matching items, sorted by
/// match quality (best matches first).
///
/// # Examples
///
/// ```
/// use matchsorter::{match_sorter, MatchSorterOptions};
///
/// let items = vec!["apple", "banana", "grape", "pineapple"];
/// let results = match_sorter(&items, "ple", MatchSorterOptions::default());
/// // Returns: ["apple", "pineapple", "grape"] (or similar, ranked by match quality)
/// ```
pub fn match_sorter<'a, T>(
    items: &'a [T],
    value: &str,
    options: MatchSorterOptions<T>,
) -> Vec<&'a T>
```

### 3. Options Struct

```rust
pub struct MatchSorterOptions<T> {
    /// Keys for extracting string values from items.
    /// If empty, items are used directly (must be string-like).
    pub keys: Vec<Key<T>>,

    /// Minimum ranking tier to include in results.
    /// Default: Ranking::Matches (include fuzzy matches and above).
    pub threshold: Ranking,

    /// Custom tiebreaker sort function.
    /// Called when two items have identical rank and key index.
    /// Default: alphabetical comparison of ranked values.
    pub base_sort: Option<Box<dyn Fn(&RankedItem<T>, &RankedItem<T>) -> Ordering>>,

    /// Whether to preserve diacritics/accents in comparisons.
    /// Default: false (strip diacritics).
    pub keep_diacritics: bool,

    /// Complete sort override. If set, replaces the entire sorting
    /// pipeline. Receives filtered items, must return sorted items.
    pub sorter: Option<Box<dyn Fn(Vec<RankedItem<T>>) -> Vec<RankedItem<T>>>>,
}
```

Implement `Default` with the same defaults as JS:
- `threshold`: `Ranking::Matches`
- `keep_diacritics`: `false`
- `base_sort`: `None` (uses default alphabetical)
- `sorter`: `None`

### 4. RankedItem Struct

```rust
/// An item annotated with its ranking information.
///
/// Used internally during sorting and exposed to custom sort functions.
pub struct RankedItem<'a, T> {
    /// Reference to the original item
    pub item: &'a T,

    /// Original index in the input slice (for stable sorting)
    pub index: usize,

    /// The ranking score
    pub rank: f64,

    /// The string value that produced the best match
    pub ranked_value: String,

    /// Index in the flattened key-values list
    pub key_index: usize,

    /// Per-key threshold if set
    pub key_threshold: Option<Ranking>,
}
```

### 5. Sorting Algorithm (`sort_ranked_values`)

Three-level comparison matching the JS implementation:

1. **Higher rank wins** (descending by rank)
2. **Lower key index wins** (ascending by key_index)
3. **`base_sort` tiebreaker** (default: `ranked_value` alphabetical via `str::cmp`)

```rust
fn sort_ranked_values<T>(
    a: &RankedItem<T>,
    b: &RankedItem<T>,
    base_sort: &dyn Fn(&RankedItem<T>, &RankedItem<T>) -> Ordering,
) -> Ordering {
    // Higher rank first
    b.rank.partial_cmp(&a.rank)
        .unwrap_or(Ordering::Equal)
        // Then lower key index first
        .then_with(|| a.key_index.cmp(&b.key_index))
        // Then base sort tiebreaker
        .then_with(|| base_sort(a, b))
}
```

### 6. Default Base Sort

The JS version uses `localeCompare`. For Rust, the default should be simple byte-wise string comparison (`str::cmp`). If locale-aware sorting is needed, users can provide a custom `base_sort`.

### 7. The Pipeline

```
match_sorter(items, query, options)
  |
  v
1. For each (index, item) in items.iter().enumerate():
     ranking_info = get_highest_ranking(item, &options.keys, query, &options)
     if ranking_info.rank >= effective_threshold:
       push RankedItem { item, index, ...ranking_info }
  |
  v
2. If options.sorter is Some:
     call sorter(ranked_items)
   Else:
     ranked_items.sort_by(|a, b| sort_ranked_values(a, b, base_sort))
  |
  v
3. Map to Vec<&T> via .iter().map(|ri| ri.item).collect()
```

### 8. No-Keys Mode (String-like Items)

When `options.keys` is empty, items must be convertible to `&str`. Support this via a trait:

```rust
pub trait AsMatchStr {
    fn as_match_str(&self) -> Vec<Cow<'_, str>>;
}

// Implement for common types
impl AsMatchStr for String { ... }
impl AsMatchStr for &str { ... }
impl AsMatchStr for Cow<'_, str> { ... }
```

When no keys are provided AND items implement `AsMatchStr`, rank the item directly.

### 9. Benchmarks

Use `criterion` for benchmarking. Benchmark scenarios:

#### Dataset Sizes
- **Small:** 100 items
- **Medium:** 10,000 items
- **Large:** 100,000 items

#### Query Types
- Exact match query
- Prefix query
- Substring query
- Fuzzy query (characters scattered)
- No-match query (worst case -- must check all tiers)

#### Key Configurations
- No keys (string items)
- Single key
- Multiple keys (3-5)
- Keys with multi-value extraction

#### Specific Benchmarks

1. **Throughput:** items/second for each dataset size + query type
2. **Ranking hot path:** time to rank a single item (isolated `get_match_ranking`)
3. **Diacritics overhead:** with vs without `keep_diacritics`
4. **Sort overhead:** sorting time for pre-ranked items
5. **Allocation profile:** measure allocations per call (use `dhat` or similar)

#### Performance Targets

- Ranking a single string pair: < 1 microsecond
- 10,000 items with single key: < 10ms
- 100,000 items with single key: < 100ms

### 10. Algorithm Selection and Optimization

#### Substring Search
- Use `memchr` crate for fast single-byte and multi-byte substring search
- For the `indexes_of` iterator, use `str::match_indices` or a custom iterator with `memchr`

#### Sorting
- Use the standard library `sort_by` (which is a stable merge sort variant)
- For very large result sets, consider `sort_unstable_by` with index tiebreaking for stability

#### Diacritics
- Use `unicode-normalization` for NFD decomposition
- Cache stripped strings if the same value is compared multiple times
- Skip stripping entirely when `keep_diacritics: true`

#### Parallelism
- For large datasets (100k+), consider using `rayon` for parallel ranking
- The ranking of each item is independent -- embarrassingly parallel
- Only add if benchmarks show meaningful improvement; keep it opt-in via a feature flag

### 11. Public API Surface (re-exports from lib.rs)

```rust
// Primary function
pub fn match_sorter<T>(...) -> Vec<&T>;

// Types
pub struct MatchSorterOptions<T>;
pub struct Key<T>;
pub struct RankedItem<T>;

// Ranking tiers
pub enum Ranking { ... }

// Utility (exposed for advanced users)
pub fn get_match_ranking(test_string: &str, query: &str, keep_diacritics: bool) -> f64;
pub fn default_base_sort<T>(a: &RankedItem<T>, b: &RankedItem<T>) -> Ordering;
```

### 12. Integration Tests

Port the JS test suite to Rust. The JS repo's tests cover:

- Basic string array matching
- Case sensitivity
- Diacritics handling
- Threshold filtering
- Key-based matching with structs
- Multi-value keys
- Per-key threshold, min_ranking, max_ranking
- Custom base_sort preserving original order
- Custom sorter override
- Empty query behavior
- Single character queries
- Acronym matching
- Word boundary detection
- Edge cases (empty items, null-like values, very long strings)

## Acceptance Criteria

1. `match_sorter(&["apple", "banana", "grape"], "ap", default_options)` returns `["apple", "grape"]` (apple first via StartsWith, grape via fuzzy)
2. Threshold filtering works: setting `threshold = Contains` excludes fuzzy-only matches
3. Custom `base_sort` preserving original order works correctly
4. Custom `sorter` override completely replaces sorting logic
5. No-keys mode works with `Vec<String>`, `Vec<&str>`
6. Key-based mode works with custom structs
7. The crate compiles as a library (`lib.rs`, not `main.rs`)
8. Criterion benchmarks exist and run via `cargo bench`
9. Performance meets targets (< 10ms for 10k items single key)
10. `cargo test` passes all unit + integration tests
11. `cargo clippy -- -D warnings` clean
12. `cargo fmt --check` clean
13. Zero `unsafe` blocks
14. All public items have doc comments
15. Results match JS `match-sorter` behavior for equivalent inputs

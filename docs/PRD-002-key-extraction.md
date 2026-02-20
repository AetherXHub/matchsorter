# PRD-002: Key Extraction and Value Resolution

**Status:** TICKETS READY

## Overview

Implement the key system that allows `matchsorter` to extract string values from arbitrary Rust types for ranking. This is the bridge between user-defined data structures and the core ranking engine (PRD-001).

## Background

The JS `match-sorter` supports multiple key types: simple string property names, dot-path nested access, callback functions, and object keys with per-key threshold/min/max ranking attributes. In Rust, we don't have dynamic property access, so keys must be modeled as closures or accessor functions.

## Goals

- Provide a flexible, ergonomic key system for extracting `&str` or `String` values from user types
- Support per-key `threshold`, `min_ranking`, and `max_ranking` overrides
- Support keys that return single values or multiple values (e.g., tags array)
- Maintain the key ordering semantics from JS (earlier keys = higher priority in tiebreaks)

## Non-Goals

- JavaScript-style dot-path string keys (Rust doesn't have runtime property access)
- Sorting/filtering logic (PRD-003)

## Detailed Design

### 1. Key Trait / Types

Since Rust is statically typed, we cannot use string property names. Instead, keys are closures that extract values from items.

```rust
/// A single key specification for extracting match-able values from an item.
pub struct Key<T> {
    /// Closure that extracts one or more string values from an item.
    /// Returns a Vec<String> to support multi-valued fields (e.g., tags).
    extractor: Box<dyn Fn(&T) -> Vec<String>>,

    /// Per-key threshold override. If set, this key's matches must meet
    /// this threshold to be considered.
    threshold: Option<Ranking>,

    /// Maximum ranking this key can contribute. Clamps the rank down.
    max_ranking: Ranking,

    /// Minimum ranking this key can contribute. Promotes matches
    /// (but never promotes NoMatch).
    min_ranking: Ranking,
}
```

### 2. Key Builder API

Provide ergonomic construction:

```rust
// Simple key: single value extraction
let key = Key::new(|item: &User| vec![item.name.clone()]);

// Key with attributes
let key = Key::new(|item: &User| vec![item.email.clone()])
    .max_ranking(Ranking::Contains)
    .threshold(Ranking::StartsWith);

// Multi-value key
let key = Key::new(|item: &User| item.tags.clone());

// Key with min_ranking (boost)
let key = Key::new(|item: &User| vec![item.alias.clone()])
    .min_ranking(Ranking::Contains);
```

### 3. Convenience Constructors

For simple single-field access, provide a shorthand:

```rust
// These should be equivalent:
Key::new(|u: &User| vec![u.name.clone()])
Key::from_fn(|u: &User| u.name.as_str())  // borrows, converts internally
```

Consider supporting both owned and borrowed returns to minimize allocations:

```rust
/// Key that extracts a single borrowed value
pub fn from_fn<F>(f: F) -> Key<T>
where
    F: Fn(&T) -> &str + 'static

/// Key that extracts multiple borrowed values
pub fn from_fn_multi<F>(f: F) -> Key<T>
where
    F: Fn(&T) -> Vec<&str> + 'static
```

Internally, the ranking engine works on `&str`, so borrowed returns avoid cloning.

### 4. Value Resolution (`get_item_values` equivalent)

```rust
/// Extract all string values from an item for a given key.
///
/// Returns a Vec of string values. If the extractor returns an empty
/// vec, the item produces no match candidates for this key.
fn get_item_values<T>(item: &T, key: &Key<T>) -> Vec<String>
```

### 5. Highest Ranking Across Keys (`get_highest_ranking` equivalent)

For a given item, evaluate all keys and return the best ranking:

```rust
pub struct RankingInfo {
    /// The ranking score (0.0 for NoMatch, up to 7.0 for CaseSensitiveEqual)
    pub rank: f64,

    /// The string value that produced the best match
    pub ranked_value: String,

    /// Index of the value in the flattened key-values list
    pub key_index: usize,

    /// Per-key threshold if set, otherwise None
    pub key_threshold: Option<Ranking>,
}
```

**Algorithm:**
1. Flatten all keys' extracted values into a single list, preserving key order
2. For each value, compute `get_match_ranking(value, query, options)`
3. Apply `min_ranking` / `max_ranking` clamping:
   - If `rank < min_ranking` AND `rank >= MATCHES` -> promote to `min_ranking`
   - If `rank > max_ranking` -> clamp to `max_ranking`
   - `NoMatch` is NEVER promoted by `min_ranking`
4. Track the best ranking across all values
5. Return the best `RankingInfo`

### 6. Handling Cow<'_, str> for Zero-Copy

Where possible, the key system should support borrowing from items rather than cloning:

```rust
// Ideal: zero-copy extraction
Key::new(|item: &User| vec![Cow::Borrowed(item.name.as_str())])

// Fallback: owned when transformation needed
Key::new(|item: &User| vec![Cow::Owned(item.name.to_uppercase())])
```

Use `Cow<'_, str>` internally to allow both patterns.

### 7. No-Keys Mode

When no keys are provided, items must implement a trait to be used directly as strings:

```rust
// For Vec<String> or Vec<&str> -- no keys needed
match_sorter(&items, "query", MatchSorterOptions::default())

// This requires items to be convertible to &str
```

Support this via a trait bound or by treating `String`/`&str` items specially.

## Acceptance Criteria

1. `Key::new` accepts closures that return `Vec<String>` or equivalent
2. Builder methods `.threshold()`, `.max_ranking()`, `.min_ranking()` work correctly
3. `max_ranking` clamps rankings down: a key with `max_ranking = Contains` never produces `StartsWith` or above
4. `min_ranking` promotes non-NoMatch rankings: a `Matches` result is promoted to `min_ranking` if `min_ranking > Matches`
5. `min_ranking` does NOT promote `NoMatch`: an item that doesn't match at all stays `NoMatch`
6. Multiple keys are evaluated in order; the best ranking wins
7. When two keys produce the same rank, the earlier key (lower index in flattened values) wins
8. Multi-value keys (e.g., tags array) rank each value independently; best wins
9. No-keys mode works with `Vec<String>` and `Vec<&str>` inputs
10. Zero `unsafe` blocks
11. Unit tests for all key configurations and edge cases
12. `cargo test` passes, `cargo clippy -- -D warnings` clean, `cargo fmt --check` clean

# matchsorter

Fuzzy string matching and sorting for Rust, inspired by Kent C. Dodds'
[match-sorter](https://github.com/kentcdodds/match-sorter) for JavaScript.

[![crates.io](https://img.shields.io/crates/v/matchsorter.svg)](https://crates.io/crates/matchsorter)
[![docs.rs](https://docs.rs/matchsorter/badge.svg)](https://docs.rs/matchsorter)
[![license](https://img.shields.io/crates/l/matchsorter.svg)](LICENSE-MIT)

## Overview

`matchsorter` ranks candidate strings against a search query using an **8-tier
ranking system**, then returns them sorted from best to worst match. It handles
everything from exact equality down to fuzzy character-by-character matching,
with optional diacritics normalization, key extraction for structs, and
per-key ranking controls.

Key differences from the JS library:

- Native Rust performance (5-8x faster on equivalent workloads)
- Zero-copy ranking in no-keys mode (borrows directly from input)
- SIMD-accelerated substring search via `memchr`
- Amortized allocations through reusable buffers and `PreparedQuery`

## Quick Start

Add to your `Cargo.toml`:

```sh
cargo add matchsorter
```

```rust
use matchsorter::{match_sorter, MatchSorterOptions};

let items = ["apple", "banana", "grape", "pineapple"];
let results = match_sorter(&items, "ap", MatchSorterOptions::default());
assert_eq!(results[0], &"apple");
```

## Ranking Tiers

Every candidate is classified into one of 8 tiers, checked in order from best
to worst. The first matching tier is used.

| Tier | Name | Example (query `"app"`) |
|------|------|-------------------------|
| 7 | **CaseSensitiveEqual** | `"app"` matches `"app"` exactly |
| 6 | **Equal** | `"app"` matches `"APP"` (case-insensitive) |
| 5 | **StartsWith** | `"app"` matches `"apple"` |
| 4 | **WordStartsWith** | `"app"` matches `"pine apple"` (word boundary) |
| 3 | **Contains** | `"app"` matches `"pineapple"` (substring) |
| 2 | **Acronym** | `"nwa"` matches `"North-West Airlines"` |
| 1..2 | **Matches** | `"plgnd"` fuzzy-matches `"playground"` |
| 0 | **NoMatch** | No match found |

## Features

- **8-tier ranking** from exact match to fuzzy character matching
- **Diacritics normalization** -- `"cafe"` matches `"cafe"` by default (toggle with `keep_diacritics`)
- **Key extraction** -- match against struct fields with `Key::new`, `Key::from_fn`, or `Key::from_fn_multi`
- **Per-key controls** -- `threshold`, `min_ranking`, and `max_ranking` per key
- **Custom sorting** -- replace the tiebreaker (`base_sort`) or the entire sort (`sorter`)
- **Zero-copy no-keys mode** -- `&str`, `String`, and `Cow<str>` work out of the box via `AsMatchStr`

## Advanced Usage

### Keys mode with structs

```rust
use matchsorter::{match_sorter, MatchSorterOptions, AsMatchStr};
use matchsorter::key::Key;

struct User { name: String, email: String }

// Required for compilation; unused in keys mode.
impl AsMatchStr for User {
    fn as_match_str(&self) -> &str { &self.name }
}

let users = vec![
    User { name: "Alice".into(), email: "alice@example.com".into() },
    User { name: "Bob".into(),   email: "bob@example.com".into() },
];

let opts = MatchSorterOptions {
    keys: vec![
        Key::from_fn(|u: &User| u.name.as_str()),
        Key::from_fn(|u: &User| u.email.as_str()),
    ],
    ..Default::default()
};

let results = match_sorter(&users, "ali", opts);
assert_eq!(results[0].name, "Alice");
```

### Custom threshold

```rust
use matchsorter::{match_sorter, MatchSorterOptions, Ranking};

let items = ["apple", "banana", "playground"];
let opts = MatchSorterOptions {
    threshold: Ranking::Contains,
    ..Default::default()
};
// Only items with a Contains ranking or better are returned.
// Fuzzy-only matches are excluded.
let results = match_sorter(&items, "pl", opts);
assert_eq!(results.len(), 2); // "apple" (Contains) and "playground" (StartsWith)
```

### Per-key clamping

```rust
use matchsorter::{match_sorter, MatchSorterOptions, AsMatchStr, Ranking};
use matchsorter::key::Key;

struct Item { name: String, description: String }

impl AsMatchStr for Item {
    fn as_match_str(&self) -> &str { &self.name }
}

let items = vec![
    Item { name: "Rust".into(), description: "A systems programming language".into() },
];

let opts = MatchSorterOptions {
    keys: vec![
        Key::from_fn(|i: &Item| i.name.as_str()),
        // Cap description matches to Contains so name matches always win
        Key::from_fn(|i: &Item| i.description.as_str())
            .max_ranking(Ranking::Contains),
    ],
    ..Default::default()
};

let results = match_sorter(&items, "rust", opts);
assert_eq!(results[0].name, "Rust");
```

## Performance

Benchmarked against the JS `match-sorter` library (Node.js v22) on 10,000
items. All times are median microseconds.

| Benchmark | Rust | JS | Speedup |
|-----------|-----:|---:|--------:|
| Throughput (10k items) | 481 us | 2,717 us | **5.6x** |
| Exact match | 380 us | 2,092 us | **5.5x** |
| Prefix match | 328 us | 2,224 us | **6.8x** |
| Substring match | 390 us | 2,113 us | **5.4x** |
| Fuzzy match | 495 us | 2,896 us | **5.9x** |
| No match (early exit) | 359 us | 1,870 us | **5.2x** |

See [`bench-compare/`](bench-compare/) for reproduction instructions.

## License

Licensed under either of

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.

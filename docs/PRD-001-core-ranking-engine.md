# PRD-001: Core Ranking Engine

**Status:** TICKETS READY

## Overview

Implement the foundational ranking algorithm for `matchsorter` -- the core engine that determines how well a candidate string matches a search query. This is the heart of the library and must be a faithful 1:1 port of the JavaScript `match-sorter` ranking tiers.

## Background

The JavaScript `match-sorter` library by Kent C. Dodds uses an 8-tier ranking system to classify how well a string matches a query. The tiers are checked in descending order of specificity, and the first matching tier is returned. A fuzzy `MATCHES` tier provides a continuous sub-score for partial character-by-character matches.

## Goals

- Implement all 8 ranking tiers with identical semantics to the JS version
- Implement diacritics/accent stripping (equivalent to JS `remove-accents`)
- Zero `unsafe` code
- Maximize performance: avoid unnecessary allocations, use `&str` where possible

## Non-Goals

- Key-based extraction from structs (PRD-002)
- Sorting, filtering, and the public `match_sorter()` API (PRD-003)
- Python bindings or WASM targets

## Detailed Design

### 1. Ranking Tiers

Define an enum or set of constants representing the 8 ranking tiers:

| Tier | Value | Description |
|------|-------|-------------|
| `CaseSensitiveEqual` | 7 | Exact byte-for-byte match |
| `Equal` | 6 | Case-insensitive full match |
| `StartsWith` | 5 | Item starts with query (case-insensitive) |
| `WordStartsWith` | 4 | A word within the item starts with query |
| `Contains` | 3 | Item contains query as substring |
| `Acronym` | 2 | Query matches item's acronym |
| `Matches` | 1..2 | Fuzzy in-order character match (sub-scored in [1, 2)) |
| `NoMatch` | 0 | No match |

The `Matches` tier produces a floating-point sub-score. The ranking type must support both integer tiers and the continuous `Matches` range. A good representation is an `f64` value, or a struct that pairs a tier enum with an optional sub-score.

### 2. The `get_match_ranking` Function

Given a `test_string` (candidate) and `string_to_rank` (query), determine the ranking. Both strings are first prepared via `prepare_value_for_comparison` (stringify + optional diacritics removal).

**Algorithm (must match JS exactly):**

1. If `query.len() > candidate.len()` -> `NoMatch`
2. If `candidate == query` (case-sensitive) -> `CaseSensitiveEqual`
3. Lowercase both strings
4. Find all indexes where `query` appears in `candidate` (substring search)
5. If first index is 0 and lengths are equal -> `Equal`
6. If first index is 0 -> `StartsWith`
7. If any index `pos > 0` has `candidate[pos - 1] == ' '` -> `WordStartsWith`
8. If first index exists (> 0) -> `Contains`
9. If `query.len() == 1` -> `NoMatch` (single char not found as substring cannot match further)
10. Compute acronym of candidate; if acronym contains query -> `Acronym`
11. Attempt fuzzy closeness ranking -> `Matches(sub_score)` or `NoMatch`

### 3. The `get_acronym` Function

Extract the acronym from a string. Word boundaries are spaces (`' '`) and hyphens (`'-'`) only.

- Start with a virtual leading delimiter (first char is always included)
- For each char: if previous was `' '` or `'-'` and current is neither, append current to acronym

Example: `"North-West Airlines"` -> `"nwa"` (after lowercasing in caller)

### 4. The `get_closeness_ranking` Function (Fuzzy Matching)

Greedy forward character-by-character scan:

1. For each character in `query`, scan forward in `candidate` to find it
2. If any character is not found -> `NoMatch`
3. Compute `spread = last_match_pos - first_match_pos`
4. Score = `MATCHES + (query.len() / query.len()) * (1.0 / spread)` = `1.0 + 1.0 / spread`
5. Result is in the range `(1.0, 2.0]`

**Performance note:** This is a linear scan O(n) where n = candidate length. The JS version uses the same greedy approach.

### 5. Diacritics Removal

The JS library uses the `remove-accents` npm package which uses a lookup table. For Rust:

- Use Unicode NFD normalization + filtering combining marks (category `Mn`)
- This handles the general case better than a lookup table
- Consider using the `unicode-normalization` crate for NFD
- Alternatively, use `deunicode` or a custom lookup table matching the JS behavior

**Decision needed:** The JS `remove-accents` handles some special cases (ligatures like "OE" -> "OE", thorn -> "TH") that simple NFD stripping misses. We should match the JS behavior as closely as practical. Use `unicode-normalization` for NFD decomposition and strip combining characters, with special-case handling for ligatures.

### 6. String Comparison Details

The JS version uses `.toLowerCase()` and `.indexOf()` which operate on UTF-16 code units. Rust strings are UTF-8. Key differences to handle:

- **Case folding:** Use `.to_lowercase()` which handles Unicode properly
- **Substring search:** Use `.find()` or a custom iterator for all indexes
- **Character iteration:** Use `.chars()` for Unicode-correct iteration
- **Word boundaries:** Only space (`' '`) for `WordStartsWith`; space and hyphen for acronyms

### 7. Performance Considerations

- Avoid allocating new `String`s where possible; use `Cow<'_, str>` for the diacritics path (return borrowed if no changes needed)
- The `indexes_of` equivalent should be a lazy iterator, not a collected `Vec`
- Consider using `memchr` crate for fast single-byte substring scanning
- Profile: the hot path is `get_match_ranking` called once per item per key value

## Acceptance Criteria

1. All 8 ranking tiers produce identical results to the JS version for the same inputs
2. `get_match_ranking("Green", "green")` -> `Equal`
3. `get_match_ranking("Green", "Green")` -> `CaseSensitiveEqual`
4. `get_match_ranking("Greenland", "green")` -> `StartsWith`
5. `get_match_ranking("San Francisco", "fran")` -> `WordStartsWith`
6. `get_match_ranking("abcdef", "cde")` -> `Contains`
7. `get_match_ranking("North-West Airlines", "nwa")` -> `Acronym`
8. `get_match_ranking("playground", "plgnd")` -> `Matches` with sub-score in (1.0, 2.0)
9. `get_match_ranking("abc", "xyz")` -> `NoMatch`
10. `get_match_ranking("cafe", "cafe")` with `keep_diacritics: false` -> `Equal` (accent stripped)
11. `get_match_ranking("cafe", "cafe")` with `keep_diacritics: true` -> scores lower or `NoMatch` depending on exact chars
12. Single character query `"x"` that is not a substring -> `NoMatch` (no acronym/fuzzy attempted)
13. Empty query `""` against any non-empty string -> `StartsWith`
14. Word boundary detection uses only spaces, not hyphens/underscores
15. Zero `unsafe` blocks in all code
16. Unit tests covering every tier and edge case
17. `cargo test` passes, `cargo clippy -- -D warnings` clean, `cargo fmt --check` clean

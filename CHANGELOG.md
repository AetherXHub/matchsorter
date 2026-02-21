# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-02-21

### Changed

- Latin-1 lookup table for O(n) byte-level diacritics stripping (~4x over JS, up from ~2.5x)
- Single-pass NFD with lazy allocation for non-Latin-1 diacritics
- Already-lowercase early-exit in `lowercase_into` for ASCII and non-ASCII
- Pre-allocated `candidate_buf` to avoid grow-from-zero reallocation

## [0.1.0] - 2026-02-21

### Added

- 8-tier ranking system (CaseSensitiveEqual, CaseInsensitiveEqual, StartsWith,
  WordStartsWith, StringCase, Contains, Acronym, Matches) with fuzzy sub-scores
  for fine-grained ordering within tiers
- `match_sorter()` main entry point for filtering and sorting string collections
- No-keys mode with `AsMatchStr` trait implementations for `&str`, `String`, and
  `Cow<str>`
- Keys mode with `Key::new`, `Key::from_fn`, and `Key::from_fn_multi` extractors
  for ranking structured data by arbitrary string fields
- Per-key `threshold`, `min_ranking`, and `max_ranking` controls for tuning match
  sensitivity
- Diacritics normalization via Unicode NFD decomposition and combining mark removal
- Custom sort overrides through `base_sort` and `sorter` options
- SIMD-accelerated substring search via `memchr`
- `PreparedQuery` for amortized per-item ranking cost when reusing a query
- Criterion benchmarks and JS comparison benchmarks
- Dual MIT/Apache-2.0 licensing

[Unreleased]: https://github.com/AetherXHub/matchsorter/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/AetherXHub/matchsorter/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/AetherXHub/matchsorter/releases/tag/v0.1.0

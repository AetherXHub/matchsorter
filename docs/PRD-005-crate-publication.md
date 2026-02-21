# PRD-005: Crate Publication Preparation

**Status:** DRAFT
**Created:** 2026-02-21
**Author:** PRD Writer Agent

---

## Problem Statement

The matchsorter crate is feature-complete with comprehensive inline documentation on all public items, 285 passing tests, and benchmarks. However it cannot be published to crates.io because it is missing essential publication artifacts: no README, no license files, incomplete Cargo.toml metadata, and the crate-level documentation (the docs.rs landing page) needs expansion to match the quality of the per-item docs.

## Goals

- Create dual MIT/Apache-2.0 license files.
- Add all required and recommended Cargo.toml metadata for crates.io.
- Create a best-in-class README.md with quick start, ranking tier reference, advanced usage, and performance comparison.
- Expand the crate-level `//!` documentation in `src/lib.rs` to serve as an excellent docs.rs landing page.
- Pass `cargo publish --dry-run` with zero errors.
- Tag `v0.1.0` for release.

## Non-Goals

- Actually running `cargo publish` (user will do this manually after review).
- Setting up CI/CD pipelines.
- Creating a GitHub repository (no repo URL yet; fields will be added later).
- Changelog generation (can be added in a future release).

## Prerequisites

- Commit the existing uncommitted performance optimizations (Changes 1-6) and PRD-004 as a separate commit before starting this work.

---

## Requirements

### 1. License Files

**Files:** `LICENSE-MIT`, `LICENSE-APACHE` (repo root)

Create standard license texts with copyright holder "matchsorter contributors".

**LICENSE-MIT:** Standard MIT license. Year: 2026. Copyright: "matchsorter contributors".

**LICENSE-APACHE:** Full Apache License, Version 2.0 text. Copyright notice in NOTICE-equivalent appendix: "matchsorter contributors".

### 2. Cargo.toml Metadata

**File:** `Cargo.toml`

Add the following fields to `[package]`:

```toml
description = "Fuzzy string matching and sorting, inspired by match-sorter"
license = "MIT OR Apache-2.0"
readme = "README.md"
keywords = ["fuzzy", "search", "filter", "sort", "match"]
categories = ["algorithms", "text-processing"]
exclude = ["bench-compare/", "docs/", ".claude/"]
```

Notes:
- `repository` and `homepage` are omitted until a GitHub repo is created.
- `keywords` limited to 5 (crates.io maximum).
- `categories` must be from the [crates.io allowed list](https://crates.io/category_slugs).
- `exclude` prevents benchmark comparison scripts, internal PRDs, and Claude config from being packaged.

### 3. README.md

**File:** `README.md` (repo root)

Structure modeled on high-quality crates (`regex`, `itertools`, `fuzzy-matcher`):

#### 3.1 Header
- Crate name as H1
- One-line description
- Badge placeholders for crates.io version, docs.rs, license (URLs filled in after first publish)

#### 3.2 Overview
- What the crate does (1-2 paragraphs)
- Mention it is a Rust port of the JS [match-sorter](https://github.com/kentcdodds/match-sorter) library by Kent C. Dodds
- Mention the 8-tier ranking system
- Highlight key differentiator: 5-10x faster than JS reference

#### 3.3 Quick Start
- `cargo add matchsorter`
- Minimal working example: filter a `&[&str]` with `match_sorter`

#### 3.4 Ranking Tiers
- Table with all 8 tiers: name, numeric value, description, example input/query/result
- Same content as the `Ranking` enum doc but formatted for README readability

#### 3.5 Features
Bulleted list:
- 8-tier ranking (exact -> fuzzy)
- Unicode diacritics normalization (optional)
- Key extraction for struct fields
- Per-key threshold, min/max ranking clamping
- Custom sort functions
- Zero `unsafe` code
- No runtime dependencies beyond `unicode-normalization` and `memchr`

#### 3.6 Advanced Usage
Code examples for:
- **Keys mode:** Matching structs by multiple fields
- **Custom threshold:** Filtering to only Contains and above
- **Per-key clamping:** `max_ranking` / `min_ranking` on keys

#### 3.7 Performance
- Table from `bench-compare/run.sh` output showing Rust vs JS timings
- Geometric mean speedup
- Note that benchmarks can be reproduced via `bash bench-compare/run.sh`

#### 3.8 License
- Dual-licensed under MIT and Apache-2.0
- Standard Rust dual-license blurb

### 4. Crate-Level Documentation Expansion

**File:** `src/lib.rs` (the `//!` block at the top)

The current crate-level doc is a brief overview + one Quick Start example. Expand to:

#### 4.1 Overview
- 2-3 sentence description of what the crate does
- Mention JS match-sorter inspiration
- Link to the `Ranking` enum for tier details

#### 4.2 Quick Start
- Keep the existing example (no-keys mode with `&[&str]`)

#### 4.3 Ranking Tiers
- Compact table or list of all 8 tiers with one-line descriptions
- Link to `Ranking` enum for full documentation

#### 4.4 Keys Mode Example
- Show matching a `Vec<User>` by `name` and `email` fields using `Key::from_fn`

#### 4.5 Threshold Example
- Show filtering with `Ranking::Contains` threshold

#### 4.6 Diacritics Example
- Show `keep_diacritics: true` vs default behavior

#### 4.7 Feature Highlights
- Bulleted list matching README features section

All examples must be runnable doc-tests (```` ```rust ```` blocks with `use` statements).

### 5. Verification

**No new file; commands to run.**

The following must all pass after all changes:

1. `cargo test` -- all existing + new doc-tests pass
2. `cargo clippy --tests --benches -- -D warnings` -- zero warnings
3. `cargo fmt --check` -- no formatting violations
4. `cargo doc --no-deps` -- builds without warnings
5. `cargo package --list` -- verify no unwanted files (no bench-compare/, docs/, .claude/)
6. `cargo publish --dry-run` -- passes crates.io validation

### 6. Git Tag

After committing all publication-prep changes:
- Create annotated tag `v0.1.0` with message "Initial release"

---

## Acceptance Criteria

1. `LICENSE-MIT` exists in repo root and contains valid MIT license text with "Copyright (c) 2026 matchsorter contributors".
2. `LICENSE-APACHE` exists in repo root and contains the full Apache License 2.0 text.
3. `Cargo.toml` contains `description`, `license`, `readme`, `keywords`, `categories`, and `exclude` fields as specified.
4. `README.md` exists in repo root and contains all sections listed in Requirement 3 (header, overview, quick start, ranking tiers table, features, advanced usage, performance, license).
5. `src/lib.rs` crate-level `//!` documentation contains an overview, quick start, keys mode example, threshold example, diacritics example, and feature highlights.
6. All code examples in README.md are valid Rust (manually verified against crate API).
7. All `//!` doc-test examples in `src/lib.rs` pass `cargo test --doc`.
8. `cargo doc --no-deps` produces zero warnings.
9. `cargo package --list` excludes `bench-compare/`, `docs/`, and `.claude/` directories.
10. `cargo publish --dry-run` succeeds with no errors.
11. `cargo clippy --tests --benches -- -D warnings` passes.
12. `cargo fmt --check` passes.
13. Git tag `v0.1.0` exists pointing to the publication-prep commit.

## Files Changed

| File | Action |
|------|--------|
| `LICENSE-MIT` | Create |
| `LICENSE-APACHE` | Create |
| `Cargo.toml` | Modify (add metadata) |
| `README.md` | Create |
| `src/lib.rs` | Modify (expand crate-level docs) |

## Dependencies

- No new crate dependencies.
- Existing `unicode-normalization` and `memchr` dependencies are unchanged.
- `criterion` dev-dependency is unchanged.

## Open Questions

- Should `repository` and `homepage` be set to a placeholder or omitted entirely until the GitHub repo is created? **Decision: omit for now.**
- Should we include a CHANGELOG.md for v0.1.0? **Decision: defer to a future release.**

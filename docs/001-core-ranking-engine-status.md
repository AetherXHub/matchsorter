# Build Status: PRD 001 -- Core Ranking Engine

**Source PRD:** docs/PRD-001-core-ranking-engine.md
**Tickets:** docs/001-core-ranking-engine-tickets.md
**Started:** 2026-02-20 06:15
**Last Updated:** 2026-02-20 07:10
**Overall Status:** QA READY

---

## Ticket Tracker

| Ticket | Title | Status | Impl Report | Review Report | Notes |
|--------|-------|--------|-------------|---------------|-------|
| 1 | Cargo Scaffold -- Dependencies and Module Skeleton | DONE | ticket-01-impl.md | ticket-01-review.md | APPROVED |
| 2 | `Ranking` Type -- Enum, Sub-Score, and Ordering | DONE | ticket-02-impl.md | ticket-02-review.md | APPROVED |
| 3 | `prepare_value_for_comparison` -- Diacritics Stripping | DONE | ticket-03-impl.md | ticket-03-review.md | APPROVED |
| 4 | `get_acronym` -- Word-Boundary Acronym Extraction | DONE | ticket-04-impl.md | ticket-04-review.md | APPROVED |
| 5 | `get_closeness_ranking` -- Fuzzy Character-by-Character Scorer | DONE | ticket-05-impl.md | ticket-05-review.md | APPROVED |
| 6 | `get_match_ranking` -- Top-Level Ranking Orchestrator | DONE | ticket-06-impl.md | ticket-06-review.md | APPROVED |
| 7 | Integration Test Suite -- All AC Scenarios | DONE | ticket-07-impl.md | ticket-07-review.md | APPROVED |
| 8 | Verification and Quality Gates | DONE | ticket-08-impl.md | -- | Verification only |

## Prior Work Summary

- `Cargo.toml` has `unicode-normalization = "0.1"` and `memchr = "2.8"` dependencies
- `src/lib.rs` re-exports `Ranking` and `get_match_ranking` at crate root
- `src/ranking/mod.rs` contains all ranking logic (~800 lines)
- `Ranking` enum: 8 variants, `Matches(f64)` for fuzzy sub-scores, manual `PartialEq`/`PartialOrd`
- `prepare_value_for_comparison(s, keep_diacritics)` -> `Cow<str>`: NFD + combining mark strip, ASCII fast path
- `get_acronym(s)` -> `String`: word boundaries = space + hyphen only
- `get_closeness_ranking(candidate, query)` -> `Ranking`: greedy forward scan, `1.0 + 1.0 / spread`
- `get_match_ranking(test_string, query, keep_diacritics)` -> `Ranking`: 11-step algorithm, all tiers
- `tests/ranking.rs`: 18 integration tests covering PRD AC 2-14 + edge cases
- Public API: `matchsorter::Ranking`, `matchsorter::get_match_ranking`
- 87 total tests (65 unit + 18 integration + 4 doc), all passing
- Zero `unsafe` blocks anywhere in the codebase
- All quality gates clean: `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`

## Follow-Up Tickets

[None.]

## Completion Report

**Completed:** 2026-02-20 07:10
**Tickets Completed:** 8/8

### Summary of Changes

**Files created/modified:**
- `Cargo.toml` -- added `unicode-normalization` and `memchr` dependencies
- `src/lib.rs` -- library crate root, re-exports `Ranking` and `get_match_ranking`
- `src/ranking/mod.rs` -- all ranking logic (~800 lines): `Ranking` enum, `prepare_value_for_comparison`, `get_acronym`, `get_closeness_ranking`, `get_match_ranking`, 65 unit tests, 4 doc tests
- `tests/ranking.rs` -- 18 integration tests covering all PRD acceptance criteria
- `src/main.rs` -- deleted (converted from binary to library crate)

**Key architectural decisions:**
- `Ranking` enum uses manual `PartialEq`/`PartialOrd` to handle `Matches(f64)` sub-scores while keeping fixed tiers integer-comparable
- Diacritics stripping uses `Cow<str>` to avoid allocation when input is already ASCII or unchanged
- ASCII fast path in `prepare_value_for_comparison` skips NFD decomposition entirely for pure-ASCII strings
- Word boundaries for `WordStartsWith` use only spaces (matching JS); acronym extraction uses spaces and hyphens
- Character count (not byte count) used for length comparisons to handle Unicode correctly

### Known Issues / Follow-Up
- AC 10 (diacritics stripped cafe) produces `CaseSensitiveEqual` rather than `Equal` because the case-sensitive check fires before lowercasing. This is correct behavior (both strings are "cafe" after stripping), and `CaseSensitiveEqual >= Equal` satisfies the AC intent.
- Minor: one test comment inaccuracy noted in review (Ticket 7, line 204 comment about "ubermanana" should say "uber-manana"). Non-functional.

### Ready for QA: YES

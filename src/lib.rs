#![warn(missing_docs)]

//! A fuzzy string matching and sorting library.
//!
//! `matchsorter` ranks candidate strings against a search query using an 8-tier
//! ranking system, providing both exact and fuzzy matching with optional
//! diacritics normalization.

/// Ranking algorithm for scoring how well a candidate string matches a query.
pub mod ranking;

// Re-export primary public API types and functions at the crate root.
pub use ranking::{Ranking, get_match_ranking};

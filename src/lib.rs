#![warn(missing_docs)]

//! A fuzzy string matching and sorting library.
//!
//! `matchsorter` ranks candidate strings against a search query using an 8-tier
//! ranking system, providing both exact and fuzzy matching with optional
//! diacritics normalization.

/// Ranking algorithm for scoring how well a candidate string matches a query.
pub mod ranking;

/// Key extraction types for pulling matchable string values from arbitrary items.
pub mod key;

/// No-keys mode for ranking string-like items directly without key extractors.
pub mod no_keys;

/// Configuration options for the match-sorting algorithm.
pub mod options;

// Re-export primary public API types and functions at the crate root.
pub use key::{Key, RankingInfo, get_highest_ranking, get_item_values};
pub use no_keys::{AsMatchStr, rank_item};
pub use options::MatchSorterOptions;
pub use ranking::{Ranking, get_match_ranking};

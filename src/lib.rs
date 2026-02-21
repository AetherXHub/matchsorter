#![warn(missing_docs)]

//! Fuzzy string matching and sorting, inspired by Kent C. Dodds'
//! [match-sorter](https://github.com/kentcdodds/match-sorter) for JavaScript.
//!
//! `matchsorter` ranks candidate strings against a search query using an
//! **8-tier ranking system**, then returns them sorted from best to worst
//! match. It handles everything from exact case-sensitive equality down to
//! fuzzy character-by-character matching, with optional diacritics
//! normalization, key extraction for structs, and per-key ranking controls.
//!
//! # Ranking Tiers
//!
//! Every candidate is classified into one of 8 tiers, checked in order from
//! best to worst. The first matching tier is returned.
//!
//! | Tier | Name | Example (query `"app"`) |
//! |------|------|-------------------------|
//! | 7 | **CaseSensitiveEqual** | `"app"` matches `"app"` exactly |
//! | 6 | **Equal** | `"app"` matches `"APP"` (case-insensitive) |
//! | 5 | **StartsWith** | `"app"` matches `"apple"` |
//! | 4 | **WordStartsWith** | `"app"` matches `"pine apple"` (word boundary) |
//! | 3 | **Contains** | `"app"` matches `"pineapple"` (substring) |
//! | 2 | **Acronym** | `"nwa"` matches `"North-West Airlines"` |
//! | 1..2 | **Matches** | `"plgnd"` fuzzy-matches `"playground"` |
//! | 0 | **NoMatch** | No match found |
//!
//! See [`Ranking`] for full details on each tier and the `Matches` sub-score.
//!
//! # Quick Start
//!
//! ```
//! use matchsorter::{match_sorter, MatchSorterOptions};
//!
//! let items = ["apple", "banana", "grape", "pineapple"];
//! let results = match_sorter(&items, "ap", MatchSorterOptions::default());
//! // "apple" (StartsWith), "grape" (Contains), "pineapple" (Contains); "banana" is dropped
//! assert_eq!(results, vec![&"apple", &"grape", &"pineapple"]);
//! ```
//!
//! # Keys Mode
//!
//! Match against struct fields by providing [`Key`] extractors:
//!
//! ```
//! use matchsorter::{match_sorter, MatchSorterOptions, AsMatchStr};
//! use matchsorter::key::Key;
//!
//! struct User { name: String, email: String }
//!
//! impl AsMatchStr for User {
//!     fn as_match_str(&self) -> &str { &self.name }
//! }
//!
//! let users = vec![
//!     User { name: "Alice".into(), email: "alice@example.com".into() },
//!     User { name: "Bob".into(),   email: "bob@example.com".into() },
//!     User { name: "Malika".into(), email: "malika@example.com".into() },
//! ];
//!
//! let opts = MatchSorterOptions {
//!     keys: vec![
//!         Key::from_fn(|u: &User| u.name.as_str()),
//!         Key::from_fn(|u: &User| u.email.as_str()),
//!     ],
//!     ..Default::default()
//! };
//!
//! let results = match_sorter(&users, "ali", opts);
//! // "Alice" (StartsWith on name), "Malika" (Contains "ali" in name); "Bob" is dropped
//! assert_eq!(results.len(), 2);
//! assert_eq!(results[0].name, "Alice");
//! assert_eq!(results[1].name, "Malika");
//! ```
//!
//! # Custom Threshold
//!
//! Exclude fuzzy-only matches by raising the threshold:
//!
//! ```
//! use matchsorter::{match_sorter, MatchSorterOptions, Ranking};
//!
//! let items = ["apple", "banana", "playground"];
//! let opts = MatchSorterOptions {
//!     threshold: Ranking::Contains,
//!     ..Default::default()
//! };
//! let results = match_sorter(&items, "pl", opts);
//! assert_eq!(results.len(), 2);
//! ```
//!
//! # Feature Highlights
//!
//! - **8-tier ranking** from exact match to fuzzy character matching
//! - **Diacritics normalization** -- `"cafe"` matches `"cafe"` by default
//! - **Key extraction** -- [`Key::new`](key::Key::new),
//!   [`Key::from_fn`](key::Key::from_fn), [`Key::from_fn_multi`](key::Key::from_fn_multi)
//! - **Per-key controls** -- `threshold`, `min_ranking`, `max_ranking`
//! - **Custom sorting** -- replace the tiebreaker or the entire sort
//! - **Zero-copy no-keys mode** -- `&str`, `String`, `Cow<str>` via [`AsMatchStr`]
//! - **SIMD-accelerated** substring search via `memchr`

/// Ranking algorithm for scoring how well a candidate string matches a query.
pub mod ranking;

/// Key extraction types for pulling matchable string values from arbitrary items.
pub mod key;

/// No-keys mode for ranking string-like items directly without key extractors.
pub mod no_keys;

/// Configuration options for the match-sorting algorithm.
pub mod options;

/// Sorting logic for ordering matched candidates by rank and tie-breaking criteria.
pub mod sort;

use std::borrow::Cow;

// Re-export primary public API types and functions at the crate root.
pub use key::{Key, RankingInfo, get_highest_ranking, get_item_values};
pub use no_keys::{AsMatchStr, rank_item};
pub use options::{MatchSorterOptions, RankedItem};
pub use ranking::{Ranking, get_match_ranking};
pub use sort::{default_base_sort, sort_ranked_values};

use key::get_highest_ranking_prepared as get_highest_ranking_prepared_impl;
use no_keys::AsMatchStr as AsMatchStrTrait;
use ranking::{PreparedQuery, get_match_ranking_prepared as get_match_ranking_prepared_impl};
use sort::{
    default_base_sort as default_base_sort_impl, sort_ranked_values as sort_ranked_values_impl,
};

/// Filter and sort items by how well they match a search query.
///
/// This is the main entry point for the library. It implements a three-step
/// pipeline:
///
/// 1. **Rank and filter** -- For each item, compute the best ranking. Items
///    below the effective threshold are discarded.
/// 2. **Sort** -- Remaining items are sorted by match quality using a
///    three-level comparator (rank descending, key index ascending, base-sort
///    tiebreaker), unless a custom `sorter` override is provided.
/// 3. **Extract** -- Sorted [`RankedItem`]s are mapped back to `&T` references.
///
/// When `options.keys` is empty (no-keys mode), items are ranked directly via
/// [`AsMatchStr::as_match_str()`]. When keys are provided, each key's extractor
/// is called and the best ranking across all keys is used.
///
/// # Arguments
///
/// * `items` - Slice of items to search through
/// * `value` - The search query string
/// * `options` - Configuration options (threshold, keys, sorting, etc.)
///
/// # Returns
///
/// A `Vec<&T>` containing references to matching items, sorted by match
/// quality (best matches first).
///
/// # Examples
///
/// ```
/// use matchsorter::{match_sorter, MatchSorterOptions};
///
/// let items = ["apple", "banana", "grape", "pineapple"];
/// let results = match_sorter(&items, "ap", MatchSorterOptions::default());
/// // "apple" (StartsWith), "grape" (Contains), "pineapple" (Contains); "banana" is dropped
/// assert_eq!(results, vec![&"apple", &"grape", &"pineapple"]);
/// ```
///
/// With a custom threshold to exclude fuzzy matches:
///
/// ```
/// use matchsorter::{match_sorter, MatchSorterOptions, Ranking};
///
/// let items = ["apple", "banana", "playground"];
/// let opts = MatchSorterOptions {
///     threshold: Ranking::Contains,
///     ..Default::default()
/// };
/// let results = match_sorter(&items, "pl", opts);
/// // "apple" contains "pl" -> Contains (passes). "playground" starts with
/// // "pl" -> StartsWith (passes). "banana" has no match above Contains.
/// assert_eq!(results.len(), 2);
/// ```
pub fn match_sorter<'a, T>(
    items: &'a [T],
    value: &str,
    options: MatchSorterOptions<T>,
) -> Vec<&'a T>
where
    T: AsMatchStrTrait,
{
    // Step 1: Rank each item and filter by the effective threshold.
    // Pre-compute query data once to avoid redundant work per item.
    let pq = PreparedQuery::new(value, options.keep_diacritics);
    let finder = if pq.lower.is_empty() {
        None
    } else {
        Some(memchr::memmem::Finder::new(pq.lower.as_bytes()))
    };
    // Reusable buffer for lowercasing each candidate (avoids per-item allocation).
    let mut candidate_buf = String::new();

    let mut ranked_items: Vec<RankedItem<'a, T>> = Vec::with_capacity(items.len());

    for (index, item) in items.iter().enumerate() {
        let (rank, ranked_value, key_index, key_threshold) = if options.keys.is_empty() {
            // No-keys mode: rank the item directly via AsMatchStr.
            let s = item.as_match_str();
            let rank = get_match_ranking_prepared_impl(
                s,
                &pq,
                options.keep_diacritics,
                &mut candidate_buf,
                finder.as_ref(),
            );
            // Zero-copy: borrow the string directly from the input item.
            (rank, Cow::Borrowed(s), 0_usize, None)
        } else {
            // Keys mode: evaluate all keys and pick the best ranking.
            let info = get_highest_ranking_prepared_impl(
                item,
                &options.keys,
                &pq,
                &options,
                &mut candidate_buf,
                finder.as_ref(),
            );
            (
                info.rank,
                Cow::Owned(info.ranked_value),
                info.key_index,
                info.key_threshold,
            )
        };

        // Use per-key threshold when set, otherwise fall back to global threshold.
        let effective_threshold = key_threshold.as_ref().unwrap_or(&options.threshold);
        if rank >= *effective_threshold {
            ranked_items.push(RankedItem {
                item,
                index,
                rank,
                ranked_value,
                key_index,
                key_threshold,
            });
        }
    }

    // Step 2: Sort the filtered items.
    if let Some(ref sorter) = options.sorter {
        ranked_items = sorter(ranked_items);
    } else {
        ranked_items.sort_by(|a, b| {
            if let Some(ref base_sort) = options.base_sort {
                sort_ranked_values_impl(a, b, base_sort.as_ref())
            } else {
                sort_ranked_values_impl(a, b, &default_base_sort_impl)
            }
        });
    }

    // Step 3: Extract references to the original items.
    ranked_items.iter().map(|ri| ri.item).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Basic no-keys mode tests ---

    #[test]
    fn no_keys_basic_str_slice() {
        let items = ["apple", "banana", "grape"];
        let results = match_sorter(&items, "ap", MatchSorterOptions::default());
        // "apple" matches via StartsWith (best), "grape" matches via fuzzy
        assert_eq!(results[0], &"apple");
        assert!(!results.is_empty());
    }

    #[test]
    fn no_keys_exact_match_first() {
        let items = ["banana", "apple", "pineapple"];
        let results = match_sorter(&items, "apple", MatchSorterOptions::default());
        // "apple" is CaseSensitiveEqual (best rank), should be first
        assert_eq!(results[0], &"apple");
    }

    #[test]
    fn no_keys_empty_query_returns_all_sorted() {
        let items = ["banana", "apple", "cherry"];
        let results = match_sorter(&items, "", MatchSorterOptions::default());
        // Empty query matches everything via StartsWith; sorted alphabetically
        // by base_sort since all have same rank and key_index
        assert_eq!(results.len(), 3);
        assert_eq!(results[0], &"apple");
        assert_eq!(results[1], &"banana");
        assert_eq!(results[2], &"cherry");
    }

    #[test]
    fn no_keys_no_match_returns_empty() {
        let items = ["apple", "banana", "grape"];
        let results = match_sorter(&items, "xyz", MatchSorterOptions::default());
        assert!(results.is_empty());
    }

    #[test]
    fn no_keys_string_items() {
        let items = vec!["hello".to_owned(), "help".to_owned(), "world".to_owned()];
        let results = match_sorter(&items, "hel", MatchSorterOptions::default());
        // Both "hello" and "help" start with "hel"
        assert_eq!(results.len(), 2);
        // Alphabetical tiebreaker: "hello" < "help"
        assert_eq!(results[0].as_str(), "hello");
        assert_eq!(results[1].as_str(), "help");
    }

    #[test]
    fn no_keys_empty_items() {
        let items: [&str; 0] = [];
        let results = match_sorter(&items, "test", MatchSorterOptions::default());
        assert!(results.is_empty());
    }

    // --- Threshold filtering tests ---

    #[test]
    fn threshold_filters_below() {
        let items = ["apple", "banana", "grape"];
        let opts = MatchSorterOptions {
            threshold: Ranking::Contains,
            ..Default::default()
        };
        let results = match_sorter(&items, "ap", opts);
        // "apple" has "ap" at position 0 -> StartsWith (>= Contains)
        // "grape" contains "ap" at position 2 -> Contains (>= Contains)
        // "banana" has no "ap" substring or fuzzy above Contains
        assert_eq!(results, vec![&"apple", &"grape"]);
    }

    #[test]
    fn threshold_case_sensitive_equal_excludes_case_insensitive() {
        let items = ["Apple", "apple", "APPLE"];
        let opts = MatchSorterOptions {
            threshold: Ranking::CaseSensitiveEqual,
            ..Default::default()
        };
        let results = match_sorter(&items, "apple", opts);
        // Only exact case-sensitive match passes
        assert_eq!(results, vec![&"apple"]);
    }

    #[test]
    fn key_threshold_overrides_global() {
        // Use keys mode: key with per-key threshold that is stricter
        let items = vec!["apple".to_owned(), "apricot".to_owned()];
        let opts = MatchSorterOptions {
            keys: vec![
                Key::new(|s: &String| vec![s.clone()]).threshold(Ranking::CaseSensitiveEqual),
            ],
            threshold: Ranking::Matches(1.0), // global is permissive
            ..Default::default()
        };
        let results = match_sorter(&items, "apple", opts);
        // Per-key threshold is CaseSensitiveEqual, so only exact match passes
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_str(), "apple");
    }

    // --- Custom sorter tests ---

    #[test]
    fn custom_sorter_replaces_default_sort() {
        let items = ["apple", "banana", "grape"];
        let opts = MatchSorterOptions {
            // Reverse the default order
            sorter: Some(Box::new(|mut items: Vec<RankedItem<&str>>| {
                items.reverse();
                items
            })),
            ..Default::default()
        };
        let default_results = match_sorter(
            &["apple", "banana", "grape"],
            "a",
            MatchSorterOptions::default(),
        );
        let custom_results = match_sorter(&items, "a", opts);

        // Custom sorter reverses the order
        assert_eq!(custom_results.len(), default_results.len());
        if custom_results.len() > 1 {
            assert_eq!(custom_results.first(), default_results.last(),);
        }
    }

    #[test]
    fn custom_sorter_called_with_filtered_items() {
        // Sorter receives only items that pass the threshold
        let items = ["apple", "xyz"];
        let opts: MatchSorterOptions<&str> = MatchSorterOptions {
            sorter: Some(Box::new(|items: Vec<RankedItem<&str>>| {
                // "xyz" should not be in here with query "ap"
                assert!(items.iter().all(|ri| *ri.item != "xyz"));
                items
            })),
            ..Default::default()
        };
        let _ = match_sorter(&items, "ap", opts);
    }

    // --- Custom base_sort tests ---

    #[test]
    fn custom_base_sort_reverse_alphabetical() {
        let items = ["alpha", "beta", "gamma"];
        let opts = MatchSorterOptions {
            base_sort: Some(Box::new(|a: &RankedItem<&str>, b: &RankedItem<&str>| {
                b.ranked_value.cmp(&a.ranked_value)
            })),
            ..Default::default()
        };
        // All items match empty-ish query via StartsWith with the same rank
        let results = match_sorter(&items, "", opts);
        // Reverse alphabetical: gamma, beta, alpha
        assert_eq!(results[0], &"gamma");
        assert_eq!(results[1], &"beta");
        assert_eq!(results[2], &"alpha");
    }

    // --- Keys mode tests ---

    #[test]
    fn keys_mode_single_key() {
        #[derive(Debug)]
        struct Item {
            name: String,
        }
        // AsMatchStr is needed for compilation but won't be used in keys mode.
        // We need to implement it to satisfy the bound.
        impl AsMatchStr for Item {
            fn as_match_str(&self) -> &str {
                &self.name
            }
        }

        let items = vec![
            Item {
                name: "Alice".to_owned(),
            },
            Item {
                name: "Bob".to_owned(),
            },
            Item {
                name: "Charlie".to_owned(),
            },
        ];
        let opts = MatchSorterOptions {
            keys: vec![Key::new(|i: &Item| vec![i.name.clone()])],
            ..Default::default()
        };
        let results = match_sorter(&items, "ali", opts);
        // "Alice" matches via StartsWith, "Charlie" matches via fuzzy
        // ('a','l','i' found in "charlie"). "Bob" does not match.
        assert!(!results.is_empty());
        assert_eq!(results[0].name, "Alice");
    }

    #[test]
    fn keys_mode_multiple_keys_best_wins() {
        #[derive(Debug)]
        struct Person {
            name: String,
            email: String,
        }
        impl AsMatchStr for Person {
            fn as_match_str(&self) -> &str {
                &self.name
            }
        }

        let items = vec![
            Person {
                name: "Alice".to_owned(),
                email: "alice@example.com".to_owned(),
            },
            Person {
                name: "Bob".to_owned(),
                email: "bob@example.com".to_owned(),
            },
        ];
        let opts = MatchSorterOptions {
            keys: vec![
                Key::new(|p: &Person| vec![p.name.clone()]),
                Key::new(|p: &Person| vec![p.email.clone()]),
            ],
            ..Default::default()
        };
        // "alice" matches name as Equal and email as StartsWith;
        // Equal > StartsWith so name key wins
        let results = match_sorter(&items, "alice", opts);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Alice");
    }

    // --- Sorting order verification ---

    #[test]
    fn items_sorted_by_rank_descending() {
        // "apple": query "app" -> StartsWith
        // "pineapple": query "app" -> Contains (substring at pos 4)
        let items = ["pineapple", "apple"];
        let results = match_sorter(&items, "app", MatchSorterOptions::default());
        assert_eq!(results[0], &"apple"); // StartsWith > Contains
        assert_eq!(results[1], &"pineapple");
    }

    #[test]
    fn diacritics_handling() {
        let items = ["cafe", "caf\u{00e9}"];
        let results = match_sorter(&items, "cafe", MatchSorterOptions::default());
        // Both should match (diacritics stripped by default)
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn keep_diacritics_option() {
        let items = ["cafe", "caf\u{00e9}"];
        let opts = MatchSorterOptions {
            keep_diacritics: true,
            ..Default::default()
        };
        let results = match_sorter(&items, "cafe", opts);
        // Only "cafe" matches when diacritics are kept
        assert_eq!(results, vec![&"cafe"]);
    }
}

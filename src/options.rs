//! Configuration options and ranked-item types for the match-sorting algorithm.
//!
//! [`MatchSorterOptions`] controls global behavior such as diacritics handling,
//! key extraction, threshold filtering, and sort customization.
//!
//! [`RankedItem`] annotates an item with its ranking metadata, used during
//! sorting and exposed to custom sort functions.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt;

use crate::key::Key;
use crate::ranking::Ranking;

/// Type alias for a custom tiebreaker sort closure used in [`MatchSorterOptions`].
///
/// Given two ranked items, returns their relative ordering for tie-breaking
/// when rank and key index are equal.
type BaseSortFn<T> = Box<dyn Fn(&RankedItem<T>, &RankedItem<T>) -> Ordering>;

/// Type alias for a complete sort-override closure used in [`MatchSorterOptions`].
///
/// Receives the filtered ranked items and returns them in the desired final order,
/// completely replacing the default three-level sort.
type SorterFn<T> = Box<dyn Fn(Vec<RankedItem<T>>) -> Vec<RankedItem<T>>>;

/// An item annotated with its ranking information.
///
/// Produced during the ranking phase of the match-sorting pipeline and
/// passed to sorting functions (both the default three-level comparator and
/// custom `base_sort` / `sorter` overrides).
///
/// # Type Parameters
///
/// * `'a` - Lifetime of the reference to the original item in the input slice.
/// * `T` - The item type being ranked.
///
/// # Examples
///
/// ```
/// use std::borrow::Cow;
/// use matchsorter::{RankedItem, Ranking};
///
/// let item = "hello".to_owned();
/// let ranked = RankedItem {
///     item: &item,
///     index: 0,
///     rank: Ranking::CaseSensitiveEqual,
///     ranked_value: Cow::Borrowed("hello"),
///     key_index: 0,
///     key_threshold: None,
/// };
/// assert_eq!(ranked.rank, Ranking::CaseSensitiveEqual);
/// assert_eq!(*ranked.item, "hello");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RankedItem<'a, T> {
    /// Reference to the original item in the input slice.
    pub item: &'a T,

    /// Original index of the item in the input slice, used for stable
    /// sort tie-breaking.
    pub index: usize,

    /// The ranking score representing how well the item matched the query.
    pub rank: Ranking,

    /// The string value (from one of the item's keys) that produced the
    /// best match against the query. Borrowed in no-keys mode (zero-copy
    /// from the input slice) and owned in keys mode.
    pub ranked_value: Cow<'a, str>,

    /// Index of the winning key-value pair in the flattened key-values list.
    /// Lower values indicate keys declared earlier in the keys array.
    pub key_index: usize,

    /// Per-key threshold override from the winning key, or `None` if the
    /// key uses the global threshold.
    pub key_threshold: Option<Ranking>,
}

/// Global options that control match-sorting behavior.
///
/// Generic over `T` to allow type-safe key extractors via [`Key<T>`].
///
/// # Defaults
///
/// All fields default to their most common usage:
/// - `keys`: empty (no-keys mode; items must be string-like)
/// - `threshold`: `Ranking::Matches(1.0)` (include fuzzy matches and above)
/// - `keep_diacritics`: `false` (diacritics are stripped before comparison)
/// - `base_sort`: `None` (uses default alphabetical tiebreaker)
/// - `sorter`: `None` (uses default three-level sort)
///
/// Because `base_sort` and `sorter` hold trait objects (`Box<dyn Fn>`),
/// `MatchSorterOptions<T>` cannot derive `Clone`, `PartialEq`, or `Default`.
/// A manual [`Default`] implementation is provided.
///
/// # Examples
///
/// ```
/// use matchsorter::MatchSorterOptions;
///
/// // Default options: strip diacritics, no keys, lowest threshold
/// let opts = MatchSorterOptions::<String>::default();
/// assert!(!opts.keep_diacritics);
/// assert!(opts.keys.is_empty());
/// assert!(opts.base_sort.is_none());
/// assert!(opts.sorter.is_none());
/// ```
pub struct MatchSorterOptions<T> {
    /// Key extractors for pulling matchable string values from items.
    ///
    /// When empty, items are ranked directly via [`AsMatchStr`](crate::no_keys::AsMatchStr)
    /// (no-keys mode). When non-empty, each key's extractor is called on
    /// every item to produce candidate strings for ranking.
    pub keys: Vec<Key<T>>,

    /// Minimum ranking tier required to include an item in results.
    ///
    /// Items whose best ranking falls below this threshold are filtered out.
    /// Defaults to `Ranking::Matches(1.0)`, the lowest valid fuzzy match
    /// score, meaning all matching items (including fuzzy) are included.
    pub threshold: Ranking,

    /// When `true`, diacritics (accents, combining marks) are preserved during
    /// comparison. When `false` (default), diacritics are stripped so that
    /// e.g. "cafe" matches "caf\u{00e9}".
    pub keep_diacritics: bool,

    /// Custom tiebreaker sort function.
    ///
    /// Called when two items have identical rank and key index during the
    /// default three-level sort. When `None`, the default alphabetical
    /// comparison of `ranked_value` is used.
    pub base_sort: Option<BaseSortFn<T>>,

    /// Complete sort override.
    ///
    /// When `Some`, replaces the entire default sorting pipeline. The
    /// closure receives the filtered `Vec<RankedItem<T>>` and must return
    /// the items in the desired final order. When `None`, the default
    /// three-level sort (rank descending, key_index ascending, base_sort
    /// tiebreaker) is used.
    pub sorter: Option<SorterFn<T>>,
}

impl<T> Default for MatchSorterOptions<T> {
    /// Returns default options matching the JS `match-sorter` library defaults.
    ///
    /// - `keys`: empty (no-keys mode)
    /// - `threshold`: `Ranking::Matches(1.0)` (include all fuzzy matches)
    /// - `keep_diacritics`: `false`
    /// - `base_sort`: `None`
    /// - `sorter`: `None`
    fn default() -> Self {
        Self {
            keys: Vec::new(),
            threshold: Ranking::Matches(1.0),
            keep_diacritics: false,
            base_sort: None,
            sorter: None,
        }
    }
}

// Manual `Debug` implementation because `Box<dyn Fn>` does not implement
// `Debug`. We print the function fields as `Some(<fn>)` or `None`.
impl<T> fmt::Debug for MatchSorterOptions<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MatchSorterOptions")
            .field("keys", &format_args!("[{} key(s)]", self.keys.len()))
            .field("threshold", &self.threshold)
            .field("keep_diacritics", &self.keep_diacritics)
            .field(
                "base_sort",
                if self.base_sort.is_some() {
                    &"Some(<fn>)" as &dyn fmt::Debug
                } else {
                    &"None" as &dyn fmt::Debug
                },
            )
            .field(
                "sorter",
                if self.sorter.is_some() {
                    &"Some(<fn>)" as &dyn fmt::Debug
                } else {
                    &"None" as &dyn fmt::Debug
                },
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_keep_diacritics_is_false() {
        let opts = MatchSorterOptions::<String>::default();
        assert!(!opts.keep_diacritics);
    }

    #[test]
    fn default_threshold_is_matches() {
        let opts = MatchSorterOptions::<String>::default();
        assert_eq!(opts.threshold, Ranking::Matches(1.0));
    }

    #[test]
    fn default_keys_is_empty() {
        let opts = MatchSorterOptions::<String>::default();
        assert!(opts.keys.is_empty());
    }

    #[test]
    fn default_base_sort_is_none() {
        let opts = MatchSorterOptions::<String>::default();
        assert!(opts.base_sort.is_none());
    }

    #[test]
    fn default_sorter_is_none() {
        let opts = MatchSorterOptions::<String>::default();
        assert!(opts.sorter.is_none());
    }

    #[test]
    fn debug_formatting() {
        let opts = MatchSorterOptions::<String>::default();
        let debug_str = format!("{opts:?}");
        assert!(debug_str.contains("keep_diacritics"));
        assert!(debug_str.contains("threshold"));
        assert!(debug_str.contains("MatchSorterOptions"));
    }

    #[test]
    fn debug_formatting_with_base_sort() {
        let opts = MatchSorterOptions::<String> {
            base_sort: Some(Box::new(|_a, _b| Ordering::Equal)),
            ..Default::default()
        };
        let debug_str = format!("{opts:?}");
        assert!(debug_str.contains("Some(<fn>)"));
    }

    #[test]
    fn ranked_item_construction() {
        let item = "hello".to_owned();
        let ranked = RankedItem {
            item: &item,
            index: 0,
            rank: Ranking::CaseSensitiveEqual,
            ranked_value: Cow::Borrowed("hello"),
            key_index: 0,
            key_threshold: None,
        };
        assert_eq!(ranked.rank, Ranking::CaseSensitiveEqual);
        assert_eq!(ranked.ranked_value, "hello");
        assert_eq!(ranked.index, 0);
        assert_eq!(ranked.key_index, 0);
        assert_eq!(ranked.key_threshold, None);
        assert_eq!(*ranked.item, "hello");
    }

    #[test]
    fn ranked_item_with_threshold() {
        let item = 42u32;
        let ranked = RankedItem {
            item: &item,
            index: 3,
            rank: Ranking::Contains,
            ranked_value: Cow::Borrowed("forty-two"),
            key_index: 1,
            key_threshold: Some(Ranking::StartsWith),
        };
        assert_eq!(ranked.key_threshold, Some(Ranking::StartsWith));
        assert_eq!(*ranked.item, 42);
    }

    #[test]
    fn ranked_item_debug() {
        let item = "test".to_owned();
        let ranked = RankedItem {
            item: &item,
            index: 0,
            rank: Ranking::Acronym,
            ranked_value: Cow::Borrowed("test"),
            key_index: 0,
            key_threshold: None,
        };
        let debug_str = format!("{ranked:?}");
        assert!(debug_str.contains("Acronym"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn ranked_item_clone() {
        let item = "world".to_owned();
        let ranked = RankedItem {
            item: &item,
            index: 1,
            rank: Ranking::StartsWith,
            ranked_value: Cow::Borrowed("world"),
            key_index: 2,
            key_threshold: Some(Ranking::Contains),
        };
        let cloned = ranked.clone();
        assert_eq!(ranked, cloned);
    }

    #[test]
    fn ranked_item_partial_eq() {
        let item = "a".to_owned();
        let a = RankedItem {
            item: &item,
            index: 0,
            rank: Ranking::Equal,
            ranked_value: Cow::Borrowed("a"),
            key_index: 0,
            key_threshold: None,
        };
        let b = RankedItem {
            item: &item,
            index: 0,
            rank: Ranking::Equal,
            ranked_value: Cow::Borrowed("a"),
            key_index: 0,
            key_threshold: None,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn ranked_item_partial_eq_different_rank() {
        let item = "a".to_owned();
        let a = RankedItem {
            item: &item,
            index: 0,
            rank: Ranking::Equal,
            ranked_value: Cow::Borrowed("a"),
            key_index: 0,
            key_threshold: None,
        };
        let b = RankedItem {
            item: &item,
            index: 0,
            rank: Ranking::Contains,
            ranked_value: Cow::Borrowed("a"),
            key_index: 0,
            key_threshold: None,
        };
        assert_ne!(a, b);
    }
}

//! Sorting logic for ordering matched candidates by rank and tie-breaking criteria.
//!
//! Provides the default three-level comparator used by the match-sorting pipeline:
//! rank (descending), key index (ascending), then a pluggable tiebreaker.

use std::cmp::Ordering;

use crate::options::RankedItem;

/// Alphabetical tiebreaker sort for ranked items.
///
/// Compares two ranked items by their `ranked_value` field using standard
/// byte-wise string ordering (`str::cmp`). This is the Rust equivalent of
/// the JS `localeCompare` used as the default `baseSort` in the original
/// `match-sorter` library.
///
/// # Arguments
///
/// * `a` - First ranked item
/// * `b` - Second ranked item
///
/// # Returns
///
/// [`Ordering`] based on alphabetical comparison of `ranked_value` strings.
///
/// # Examples
///
/// ```
/// use std::borrow::Cow;
/// use matchsorter::{RankedItem, Ranking, default_base_sort};
/// use std::cmp::Ordering;
///
/// let item_a = "apple".to_owned();
/// let item_b = "banana".to_owned();
///
/// let a = RankedItem {
///     item: &item_a,
///     index: 0,
///     rank: Ranking::Equal,
///     ranked_value: Cow::Borrowed("apple"),
///     key_index: 0,
///     key_threshold: None,
/// };
/// let b = RankedItem {
///     item: &item_b,
///     index: 1,
///     rank: Ranking::Equal,
///     ranked_value: Cow::Borrowed("banana"),
///     key_index: 0,
///     key_threshold: None,
/// };
///
/// assert_eq!(default_base_sort(&a, &b), Ordering::Less);
/// ```
pub fn default_base_sort<T>(a: &RankedItem<T>, b: &RankedItem<T>) -> Ordering {
    a.ranked_value.cmp(&b.ranked_value)
}

/// Three-level comparator for sorting ranked items.
///
/// Implements the same sorting logic as the JS `match-sorter` library:
///
/// 1. **Higher rank wins** -- items with a better (higher) ranking come first.
/// 2. **Lower key index wins** -- when ranks are equal, items matched by an
///    earlier key come first.
/// 3. **Base sort tiebreaker** -- when both rank and key index are equal, the
///    provided `base_sort` function breaks the tie (default: alphabetical by
///    `ranked_value`).
///
/// # Arguments
///
/// * `a` - First ranked item to compare
/// * `b` - Second ranked item to compare
/// * `base_sort` - Tiebreaker function called when rank and key index are equal
///
/// # Returns
///
/// [`Ordering`] suitable for use with [`slice::sort_by`] or similar sorting methods.
///
/// # Examples
///
/// ```
/// use std::borrow::Cow;
/// use matchsorter::{RankedItem, Ranking, sort_ranked_values, default_base_sort};
/// use std::cmp::Ordering;
///
/// let items = vec!["alpha".to_owned(), "beta".to_owned()];
///
/// let a = RankedItem {
///     item: &items[0],
///     index: 0,
///     rank: Ranking::StartsWith,
///     ranked_value: Cow::Borrowed("alpha"),
///     key_index: 0,
///     key_threshold: None,
/// };
/// let b = RankedItem {
///     item: &items[1],
///     index: 1,
///     rank: Ranking::Contains,
///     ranked_value: Cow::Borrowed("beta"),
///     key_index: 0,
///     key_threshold: None,
/// };
///
/// // StartsWith > Contains, so `a` comes first (Less).
/// assert_eq!(sort_ranked_values(&a, &b, &default_base_sort), Ordering::Less);
/// ```
pub fn sort_ranked_values<T>(
    a: &RankedItem<T>,
    b: &RankedItem<T>,
    base_sort: &dyn Fn(&RankedItem<T>, &RankedItem<T>) -> Ordering,
) -> Ordering {
    // Level 1: Higher rank first (descending). `partial_cmp` returns `Option`
    // because `Ranking` contains `f64` in the `Matches` variant. If comparison
    // is indeterminate (e.g., NaN), treat as equal.
    b.rank
        .partial_cmp(&a.rank)
        .unwrap_or(Ordering::Equal)
        // Level 2: Lower key_index first (ascending).
        .then_with(|| a.key_index.cmp(&b.key_index))
        // Level 3: Tiebreaker via the caller-supplied base_sort function.
        .then_with(|| base_sort(a, b))
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;
    use crate::ranking::Ranking;

    /// Sentinel item value used by all tests. The sort functions never inspect
    /// `item` itself, so a shared static value keeps the test helpers simple.
    const ITEM: &str = "";

    /// Helper to build a `RankedItem` with only the fields relevant to sorting.
    /// Uses a shared empty `&str` as the item since the comparator functions
    /// only examine `rank`, `key_index`, and `ranked_value`.
    fn make_ranked(
        rank: Ranking,
        ranked_value: &'static str,
        key_index: usize,
    ) -> RankedItem<'static, &'static str> {
        RankedItem {
            item: &ITEM,
            index: 0,
            rank,
            ranked_value: Cow::Borrowed(ranked_value),
            key_index,
            key_threshold: None,
        }
    }

    // --- default_base_sort tests ---

    #[test]
    fn base_sort_alphabetical_less() {
        let a = make_ranked(Ranking::Equal, "apple", 0);
        let b = make_ranked(Ranking::Equal, "banana", 0);
        assert_eq!(default_base_sort(&a, &b), Ordering::Less);
    }

    #[test]
    fn base_sort_alphabetical_greater() {
        let a = make_ranked(Ranking::Equal, "banana", 0);
        let b = make_ranked(Ranking::Equal, "apple", 0);
        assert_eq!(default_base_sort(&a, &b), Ordering::Greater);
    }

    #[test]
    fn base_sort_alphabetical_equal() {
        let a = make_ranked(Ranking::Equal, "same", 0);
        let b = make_ranked(Ranking::Equal, "same", 0);
        assert_eq!(default_base_sort(&a, &b), Ordering::Equal);
    }

    #[test]
    fn base_sort_case_sensitive_ordering() {
        // Byte-wise comparison: uppercase 'A' (0x41) < lowercase 'a' (0x61).
        let a = make_ranked(Ranking::Equal, "Apple", 0);
        let b = make_ranked(Ranking::Equal, "apple", 0);
        assert_eq!(default_base_sort(&a, &b), Ordering::Less);
    }

    #[test]
    fn base_sort_empty_strings() {
        let a = make_ranked(Ranking::Equal, "", 0);
        let b = make_ranked(Ranking::Equal, "", 0);
        assert_eq!(default_base_sort(&a, &b), Ordering::Equal);
    }

    #[test]
    fn base_sort_empty_vs_nonempty() {
        let a = make_ranked(Ranking::Equal, "", 0);
        let b = make_ranked(Ranking::Equal, "z", 0);
        assert_eq!(default_base_sort(&a, &b), Ordering::Less);
    }

    // --- sort_ranked_values: rank comparison tests ---

    #[test]
    fn higher_rank_sorts_first() {
        let a = make_ranked(Ranking::StartsWith, "a", 0);
        let b = make_ranked(Ranking::Contains, "b", 0);
        assert_eq!(
            sort_ranked_values(&a, &b, &default_base_sort),
            Ordering::Less
        );
    }

    #[test]
    fn lower_rank_sorts_second() {
        let a = make_ranked(Ranking::Contains, "a", 0);
        let b = make_ranked(Ranking::StartsWith, "b", 0);
        assert_eq!(
            sort_ranked_values(&a, &b, &default_base_sort),
            Ordering::Greater
        );
    }

    #[test]
    fn case_sensitive_equal_before_equal() {
        let a = make_ranked(Ranking::CaseSensitiveEqual, "a", 0);
        let b = make_ranked(Ranking::Equal, "b", 0);
        assert_eq!(
            sort_ranked_values(&a, &b, &default_base_sort),
            Ordering::Less
        );
    }

    #[test]
    fn matches_variant_compared_by_sub_score() {
        // Higher sub-score should come first (descending rank).
        let a = make_ranked(Ranking::Matches(1.8), "a", 0);
        let b = make_ranked(Ranking::Matches(1.2), "b", 0);
        assert_eq!(
            sort_ranked_values(&a, &b, &default_base_sort),
            Ordering::Less
        );
    }

    #[test]
    fn matches_lower_sub_score_sorts_second() {
        let a = make_ranked(Ranking::Matches(1.2), "a", 0);
        let b = make_ranked(Ranking::Matches(1.8), "b", 0);
        assert_eq!(
            sort_ranked_values(&a, &b, &default_base_sort),
            Ordering::Greater
        );
    }

    // --- sort_ranked_values: key_index comparison tests ---

    #[test]
    fn lower_key_index_sorts_first_when_ranks_equal() {
        let a = make_ranked(Ranking::Contains, "z", 0);
        let b = make_ranked(Ranking::Contains, "a", 2);
        assert_eq!(
            sort_ranked_values(&a, &b, &default_base_sort),
            Ordering::Less
        );
    }

    #[test]
    fn higher_key_index_sorts_second_when_ranks_equal() {
        let a = make_ranked(Ranking::Contains, "a", 5);
        let b = make_ranked(Ranking::Contains, "z", 1);
        assert_eq!(
            sort_ranked_values(&a, &b, &default_base_sort),
            Ordering::Greater
        );
    }

    #[test]
    fn key_index_ignored_when_ranks_differ() {
        // Even though `a` has a higher key_index, it has a better rank.
        let a = make_ranked(Ranking::StartsWith, "z", 10);
        let b = make_ranked(Ranking::Contains, "a", 0);
        assert_eq!(
            sort_ranked_values(&a, &b, &default_base_sort),
            Ordering::Less
        );
    }

    // --- sort_ranked_values: base_sort tiebreaker tests ---

    #[test]
    fn base_sort_breaks_tie_when_rank_and_key_index_equal() {
        let a = make_ranked(Ranking::Contains, "apple", 0);
        let b = make_ranked(Ranking::Contains, "banana", 0);
        assert_eq!(
            sort_ranked_values(&a, &b, &default_base_sort),
            Ordering::Less
        );
    }

    #[test]
    fn base_sort_reverse_alphabetical_when_rank_and_key_index_equal() {
        let a = make_ranked(Ranking::Contains, "banana", 0);
        let b = make_ranked(Ranking::Contains, "apple", 0);
        assert_eq!(
            sort_ranked_values(&a, &b, &default_base_sort),
            Ordering::Greater
        );
    }

    #[test]
    fn all_equal_returns_equal() {
        let a = make_ranked(Ranking::Contains, "same", 0);
        let b = make_ranked(Ranking::Contains, "same", 0);
        assert_eq!(
            sort_ranked_values(&a, &b, &default_base_sort),
            Ordering::Equal
        );
    }

    // --- sort_ranked_values: custom base_sort tests ---

    #[test]
    fn custom_base_sort_reverse_alphabetical() {
        let reverse_sort =
            |a: &RankedItem<&str>, b: &RankedItem<&str>| b.ranked_value.cmp(&a.ranked_value);
        let a = make_ranked(Ranking::Contains, "apple", 0);
        let b = make_ranked(Ranking::Contains, "banana", 0);
        // With reverse alphabetical, "banana" > "apple", so b comes first.
        assert_eq!(sort_ranked_values(&a, &b, &reverse_sort), Ordering::Greater);
    }

    #[test]
    fn custom_base_sort_by_original_index() {
        // Sort by original item index as tiebreaker (preserves input order).
        let index_sort = |a: &RankedItem<&str>, b: &RankedItem<&str>| a.index.cmp(&b.index);

        let mut a = make_ranked(Ranking::Contains, "x", 0);
        a.index = 5;
        let mut b = make_ranked(Ranking::Contains, "y", 0);
        b.index = 2;
        // index 5 > index 2, so a sorts after b.
        assert_eq!(sort_ranked_values(&a, &b, &index_sort), Ordering::Greater);
    }

    #[test]
    fn custom_base_sort_not_reached_when_rank_differs() {
        // The custom base_sort should never be called when ranks differ.
        let panic_sort = |_a: &RankedItem<&str>, _b: &RankedItem<&str>| -> Ordering {
            panic!("base_sort should not be called when ranks differ");
        };
        let a = make_ranked(Ranking::StartsWith, "a", 0);
        let b = make_ranked(Ranking::Contains, "b", 0);
        assert_eq!(sort_ranked_values(&a, &b, &panic_sort), Ordering::Less);
    }

    #[test]
    fn custom_base_sort_not_reached_when_key_index_differs() {
        // The custom base_sort should never be called when key_indexes differ.
        let panic_sort = |_a: &RankedItem<&str>, _b: &RankedItem<&str>| -> Ordering {
            panic!("base_sort should not be called when key_indexes differ");
        };
        let a = make_ranked(Ranking::Contains, "a", 0);
        let b = make_ranked(Ranking::Contains, "b", 3);
        assert_eq!(sort_ranked_values(&a, &b, &panic_sort), Ordering::Less);
    }

    // --- sort_ranked_values: integration with slice::sort_by ---

    #[test]
    fn sort_by_produces_correct_order() {
        let mut ranked: Vec<RankedItem<&str>> = vec![
            make_ranked(Ranking::Contains, "cherry", 0),
            make_ranked(Ranking::StartsWith, "apple", 0),
            make_ranked(Ranking::Contains, "banana", 0),
        ];
        ranked.sort_by(|a, b| sort_ranked_values(a, b, &default_base_sort));

        // StartsWith > Contains, so "apple" first. Then "banana" < "cherry"
        // alphabetically among the Contains items.
        assert_eq!(ranked[0].ranked_value, "apple");
        assert_eq!(ranked[1].ranked_value, "banana");
        assert_eq!(ranked[2].ranked_value, "cherry");
    }

    #[test]
    fn sort_by_with_mixed_key_indexes() {
        let mut ranked: Vec<RankedItem<&str>> = vec![
            make_ranked(Ranking::Contains, "a", 2),
            make_ranked(Ranking::Contains, "b", 0),
            make_ranked(Ranking::Contains, "c", 1),
        ];
        ranked.sort_by(|a, b| sort_ranked_values(a, b, &default_base_sort));

        // All same rank, so sorted by key_index ascending: 0, 1, 2.
        assert_eq!(ranked[0].ranked_value, "b"); // key_index 0
        assert_eq!(ranked[1].ranked_value, "c"); // key_index 1
        assert_eq!(ranked[2].ranked_value, "a"); // key_index 2
    }

    #[test]
    fn sort_by_all_three_levels() {
        // Construct items that exercise all three sort levels.
        let mut ranked: Vec<RankedItem<&str>> = vec![
            // Group 1: rank=Contains, key_index=0 -- will tiebreak on ranked_value
            make_ranked(Ranking::Contains, "zebra", 0),
            make_ranked(Ranking::Contains, "alpha", 0),
            // Group 2: rank=Contains, key_index=1
            make_ranked(Ranking::Contains, "mango", 1),
            // Group 3: rank=StartsWith (highest rank)
            make_ranked(Ranking::StartsWith, "banana", 0),
        ];
        ranked.sort_by(|a, b| sort_ranked_values(a, b, &default_base_sort));

        // Expected order:
        // 1. "banana" (StartsWith, best rank)
        // 2. "alpha"  (Contains, key_index=0, alphabetically before "zebra")
        // 3. "zebra"  (Contains, key_index=0, alphabetically after "alpha")
        // 4. "mango"  (Contains, key_index=1)
        assert_eq!(ranked[0].ranked_value, "banana");
        assert_eq!(ranked[1].ranked_value, "alpha");
        assert_eq!(ranked[2].ranked_value, "zebra");
        assert_eq!(ranked[3].ranked_value, "mango");
    }
}

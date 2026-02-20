//! No-keys mode for ranking string-like items directly.
//!
//! When items are themselves strings (or string-like), there is no need to
//! construct [`Key`](crate::key::Key) extractors. The [`AsMatchStr`] trait
//! provides a uniform way to obtain a `&str` from any string-like type, and
//! [`rank_item`] uses it to score items directly against a query.

use std::borrow::Cow;

use crate::ranking::{Ranking, get_match_ranking};

/// Trait for types that can be used directly as match candidates without keys.
///
/// Implementors expose their string content via [`as_match_str`](AsMatchStr::as_match_str),
/// allowing the ranking engine to score them without key extraction.
///
/// # Built-in Implementations
///
/// - [`String`] -- delegates to [`String::as_str`]
/// - [`str`] -- returns `self`
/// - [`&str`] -- dereferences and returns `self`
/// - [`Cow<'_, str>`] -- delegates to [`AsRef::as_ref`]
///
/// # Examples
///
/// ```
/// use matchsorter::AsMatchStr;
///
/// let owned = String::from("hello");
/// assert_eq!(owned.as_match_str(), "hello");
///
/// let borrowed: &str = "world";
/// assert_eq!(borrowed.as_match_str(), "world");
/// ```
pub trait AsMatchStr {
    /// Returns the string representation of this item for matching.
    fn as_match_str(&self) -> &str;
}

impl AsMatchStr for String {
    fn as_match_str(&self) -> &str {
        self.as_str()
    }
}

impl AsMatchStr for str {
    fn as_match_str(&self) -> &str {
        self
    }
}

// Blanket impl for `&&str`, `&&&str`, etc. is not needed; this covers
// the `&str` case because Rust auto-derefs to `str` through `AsMatchStr`.
// However, an explicit impl for `&str` is necessary so that `T = &str`
// satisfies the `AsMatchStr` bound without requiring the caller to
// double-reference.
impl AsMatchStr for &str {
    fn as_match_str(&self) -> &str {
        self
    }
}

impl AsMatchStr for Cow<'_, str> {
    fn as_match_str(&self) -> &str {
        self.as_ref()
    }
}

/// Rank a string-like item directly against a query (no-keys mode).
///
/// This is a convenience wrapper around [`get_match_ranking`] for items that
/// implement [`AsMatchStr`]. It avoids the need to construct
/// [`Key`](crate::key::Key) extractors when the item itself is a string.
///
/// # Arguments
///
/// * `item` - The string-like item to rank
/// * `query` - The search query to match against
/// * `keep_diacritics` - If `true`, diacritics are preserved during comparison;
///   if `false`, they are stripped (see [`prepare_value_for_comparison`](crate::ranking::prepare_value_for_comparison))
///
/// # Returns
///
/// The [`Ranking`] tier that best describes how the query matches the item.
///
/// # Examples
///
/// ```
/// use matchsorter::no_keys::rank_item;
/// use matchsorter::Ranking;
///
/// // Rank an owned String
/// let item = String::from("Green");
/// assert_eq!(rank_item(&item, "Green", false), Ranking::CaseSensitiveEqual);
///
/// // Rank a &str
/// let item = "Greenland";
/// assert_eq!(rank_item(&item, "green", false), Ranking::StartsWith);
/// ```
pub fn rank_item<T: AsMatchStr>(item: &T, query: &str, keep_diacritics: bool) -> Ranking {
    get_match_ranking(item.as_match_str(), query, keep_diacritics)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- AsMatchStr trait implementation tests ---

    #[test]
    fn as_match_str_string() {
        let s = String::from("hello");
        assert_eq!(s.as_match_str(), "hello");
    }

    #[test]
    fn as_match_str_str_ref() {
        let s: &str = "world";
        assert_eq!(s.as_match_str(), "world");
    }

    #[test]
    fn as_match_str_cow_borrowed() {
        let cow: Cow<'_, str> = Cow::Borrowed("borrowed");
        assert_eq!(cow.as_match_str(), "borrowed");
    }

    #[test]
    fn as_match_str_cow_owned() {
        let cow: Cow<'_, str> = Cow::Owned("owned".to_owned());
        assert_eq!(cow.as_match_str(), "owned");
    }

    #[test]
    fn as_match_str_empty_string() {
        let s = String::new();
        assert_eq!(s.as_match_str(), "");
    }

    #[test]
    fn as_match_str_empty_str() {
        let s: &str = "";
        assert_eq!(s.as_match_str(), "");
    }

    // --- rank_item with String items ---

    #[test]
    fn rank_string_case_sensitive_equal() {
        let item = String::from("Green");
        assert_eq!(
            rank_item(&item, "Green", false),
            Ranking::CaseSensitiveEqual
        );
    }

    #[test]
    fn rank_string_equal() {
        let item = String::from("Green");
        assert_eq!(rank_item(&item, "green", false), Ranking::Equal);
    }

    #[test]
    fn rank_string_starts_with() {
        let item = String::from("Greenland");
        assert_eq!(rank_item(&item, "green", false), Ranking::StartsWith);
    }

    #[test]
    fn rank_string_word_starts_with() {
        let item = String::from("San Francisco");
        assert_eq!(rank_item(&item, "fran", false), Ranking::WordStartsWith);
    }

    #[test]
    fn rank_string_contains() {
        let item = String::from("abcdef");
        assert_eq!(rank_item(&item, "cde", false), Ranking::Contains);
    }

    #[test]
    fn rank_string_acronym() {
        let item = String::from("North-West Airlines");
        assert_eq!(rank_item(&item, "nwa", false), Ranking::Acronym);
    }

    #[test]
    fn rank_string_fuzzy_matches() {
        let item = String::from("playground");
        let rank = rank_item(&item, "plgnd", false);
        match rank {
            Ranking::Matches(s) => {
                assert!(
                    s > 1.0 && s < 2.0,
                    "expected sub-score in (1.0, 2.0), got {s}"
                );
            }
            other => panic!("expected Matches, got {other:?}"),
        }
    }

    #[test]
    fn rank_string_no_match() {
        let item = String::from("abc");
        assert_eq!(rank_item(&item, "xyz", false), Ranking::NoMatch);
    }

    #[test]
    fn rank_string_diacritics_stripped() {
        let item = String::from("caf\u{00e9}");
        assert_eq!(rank_item(&item, "cafe", false), Ranking::CaseSensitiveEqual);
    }

    #[test]
    fn rank_string_diacritics_kept() {
        let item = String::from("caf\u{00e9}");
        assert_eq!(rank_item(&item, "cafe", true), Ranking::NoMatch);
    }

    // --- rank_item with &str items ---

    #[test]
    fn rank_str_case_sensitive_equal() {
        let item: &str = "Green";
        assert_eq!(
            rank_item(&item, "Green", false),
            Ranking::CaseSensitiveEqual
        );
    }

    #[test]
    fn rank_str_equal() {
        let item: &str = "Green";
        assert_eq!(rank_item(&item, "green", false), Ranking::Equal);
    }

    #[test]
    fn rank_str_starts_with() {
        let item: &str = "Greenland";
        assert_eq!(rank_item(&item, "green", false), Ranking::StartsWith);
    }

    #[test]
    fn rank_str_word_starts_with() {
        let item: &str = "San Francisco";
        assert_eq!(rank_item(&item, "fran", false), Ranking::WordStartsWith);
    }

    #[test]
    fn rank_str_contains() {
        let item: &str = "abcdef";
        assert_eq!(rank_item(&item, "cde", false), Ranking::Contains);
    }

    #[test]
    fn rank_str_acronym() {
        let item: &str = "North-West Airlines";
        assert_eq!(rank_item(&item, "nwa", false), Ranking::Acronym);
    }

    #[test]
    fn rank_str_fuzzy_matches() {
        let item: &str = "playground";
        let rank = rank_item(&item, "plgnd", false);
        match rank {
            Ranking::Matches(s) => {
                assert!(
                    s > 1.0 && s < 2.0,
                    "expected sub-score in (1.0, 2.0), got {s}"
                );
            }
            other => panic!("expected Matches, got {other:?}"),
        }
    }

    #[test]
    fn rank_str_no_match() {
        let item: &str = "abc";
        assert_eq!(rank_item(&item, "xyz", false), Ranking::NoMatch);
    }

    // --- rank_item with Cow<str> items ---

    #[test]
    fn rank_cow_borrowed() {
        let item: Cow<'_, str> = Cow::Borrowed("Green");
        assert_eq!(
            rank_item(&item, "Green", false),
            Ranking::CaseSensitiveEqual
        );
    }

    #[test]
    fn rank_cow_owned() {
        let item: Cow<'_, str> = Cow::Owned("Greenland".to_owned());
        assert_eq!(rank_item(&item, "green", false), Ranking::StartsWith);
    }

    // --- Edge cases ---

    #[test]
    fn rank_empty_item_empty_query() {
        let item = String::new();
        assert_eq!(rank_item(&item, "", false), Ranking::CaseSensitiveEqual);
    }

    #[test]
    fn rank_empty_query_nonempty_item() {
        let item = String::from("anything");
        assert_eq!(rank_item(&item, "", false), Ranking::StartsWith);
    }

    #[test]
    fn rank_query_longer_than_item() {
        let item = String::from("ab");
        assert_eq!(rank_item(&item, "abcdef", false), Ranking::NoMatch);
    }

    // --- Verify no-keys path produces same results as get_match_ranking ---

    #[test]
    fn rank_item_matches_get_match_ranking_for_string() {
        let item = String::from("San Francisco");
        let via_rank_item = rank_item(&item, "fran", false);
        let via_direct = get_match_ranking("San Francisco", "fran", false);
        assert_eq!(via_rank_item, via_direct);
    }

    #[test]
    fn rank_item_matches_get_match_ranking_for_str() {
        let item: &str = "playground";
        let via_rank_item = rank_item(&item, "plgnd", false);
        let via_direct = get_match_ranking("playground", "plgnd", false);
        assert_eq!(via_rank_item, via_direct);
    }
}

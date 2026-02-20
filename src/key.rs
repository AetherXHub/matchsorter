//! Key extraction types and builder API.
//!
//! A [`Key<T>`] describes how to extract one or more string values from an
//! item of type `T` for ranking. Each key carries an extractor closure and
//! optional per-key ranking attributes (`threshold`, `min_ranking`,
//! `max_ranking`) that override global defaults during match evaluation.
//!
//! [`RankingInfo`] captures the result of evaluating a single item against
//! a query across all of its keys.

use crate::options::MatchSorterOptions;
use crate::ranking::{Ranking, get_match_ranking};

/// Extract all string values from an item for a given key.
///
/// Calls the key's extractor closure and returns the resulting values. This is
/// a thin wrapper around [`Key::extract`] that provides a free-function API
/// matching the JS `match-sorter` library's `getItemValues` function.
///
/// # Arguments
///
/// * `item` - A reference to the item to extract values from.
/// * `key` - The key specification describing how to extract values.
///
/// # Returns
///
/// A `Vec<String>` of extracted values. Returns an empty vector if the key's
/// extractor produces no values for this item.
///
/// # Examples
///
/// ```
/// use matchsorter::key::{Key, get_item_values};
///
/// let key = Key::new(|s: &String| vec![s.clone()]);
/// let values = get_item_values(&"hello".to_owned(), &key);
/// assert_eq!(values, vec!["hello"]);
/// ```
pub fn get_item_values<T>(item: &T, key: &Key<T>) -> Vec<String> {
    key.extract(item)
}

/// Evaluate all keys for a single item and return the best ranking.
///
/// Flattens all keys' extracted values into a single indexed list preserving
/// key order. Each value is scored via [`get_match_ranking`], then clamped by
/// the owning key's `min_ranking` / `max_ranking` attributes. The best-ranked
/// value is returned. When two values produce equal rank, the one with the
/// lower `key_index` (earlier in the flattened list) wins.
///
/// # Clamping Rules
///
/// - If a value's rank exceeds the key's `max_ranking`, it is clamped **down**
///   to `max_ranking`.
/// - If a value's rank is below the key's `min_ranking` **and** the rank is
///   not [`Ranking::NoMatch`], it is promoted **up** to `min_ranking`.
/// - [`Ranking::NoMatch`] is **never** promoted by `min_ranking`.
///
/// # Arguments
///
/// * `item` - The item to extract values from via the keys
/// * `keys` - Slice of key specifications to evaluate
/// * `query` - The search query string
/// * `options` - Global match-sorting options (e.g., diacritics handling)
///
/// # Returns
///
/// A [`RankingInfo`] describing the best match found across all keys. If no
/// keys or no values are present, returns a `RankingInfo` with
/// [`Ranking::NoMatch`].
///
/// # Examples
///
/// ```
/// use matchsorter::key::{Key, get_highest_ranking};
/// use matchsorter::{MatchSorterOptions, Ranking};
///
/// let keys = vec![
///     Key::new(|s: &String| vec![s.clone()]),
/// ];
/// let opts = MatchSorterOptions::default();
/// let info = get_highest_ranking(&"hello".to_owned(), &keys, "hello", &opts);
/// assert_eq!(info.rank, Ranking::CaseSensitiveEqual);
/// ```
pub fn get_highest_ranking<T>(
    item: &T,
    keys: &[Key<T>],
    query: &str,
    options: &MatchSorterOptions<T>,
) -> RankingInfo {
    let mut best = RankingInfo {
        rank: Ranking::NoMatch,
        ranked_value: String::new(),
        key_index: 0,
        key_threshold: None,
    };

    // Flatten all keys' values into a single indexed sequence. The
    // `key_index` counter runs across all values from all keys, preserving
    // the order in which keys (and their values) appear.
    let mut key_index: usize = 0;

    for key in keys {
        let values = key.extract(item);
        let threshold = key.threshold.clone();
        let min = key.min_ranking_value();
        let max = key.max_ranking_value();

        for value in &values {
            let mut rank = get_match_ranking(value, query, options.keep_diacritics);

            // Clamp down: if the rank exceeds the key's max_ranking, cap it.
            if rank > *max {
                rank = max.clone();
            }

            // Promote up: if the rank is below the key's min_ranking AND the
            // rank is NOT NoMatch, boost it to min_ranking. NoMatch is never
            // promoted -- an item that doesn't match stays unmatched.
            if rank < *min && rank != Ranking::NoMatch {
                rank = min.clone();
            }

            // Update best: strictly better rank wins, or equal rank with a
            // lower key_index wins (but since we iterate in order, the first
            // occurrence at a given rank level is already the lowest index,
            // so we only replace on strictly-greater).
            if rank > best.rank {
                best = RankingInfo {
                    rank,
                    ranked_value: value.clone(),
                    key_index,
                    key_threshold: threshold.clone(),
                };
            }

            key_index += 1;
        }
    }

    best
}

/// Type alias for the boxed extractor closure stored inside a [`Key`].
///
/// Given a reference to an item of type `T`, the extractor returns a
/// `Vec<String>` of values to rank against the query.
type Extractor<T> = Box<dyn Fn(&T) -> Vec<String>>;

/// A single key specification for extracting matchable string values from an item.
///
/// Keys are constructed via [`Key::new`], [`Key::from_fn`], or
/// [`Key::from_fn_multi`], then optionally refined with builder methods
/// (`.threshold()`, `.min_ranking()`, `.max_ranking()`).
///
/// # Type Parameter
///
/// * `T` - The item type that this key can extract values from.
///
/// # Examples
///
/// ```
/// use matchsorter::key::Key;
/// use matchsorter::Ranking;
///
/// struct User { name: String, email: String }
///
/// // Simple single-value key
/// let key = Key::new(|u: &User| vec![u.name.clone()]);
///
/// // Key with per-key ranking attributes
/// let key = Key::new(|u: &User| vec![u.email.clone()])
///     .threshold(Ranking::StartsWith)
///     .max_ranking(Ranking::Contains);
///
/// // Convenience constructor for single borrowed value
/// let key = Key::<User>::from_fn(|u| u.name.as_str());
/// ```
pub struct Key<T> {
    /// Closure that extracts one or more string values from an item.
    /// Returns a `Vec<String>` to support multi-valued fields (e.g., tags).
    extractor: Extractor<T>,

    /// Per-key threshold override. When `Some`, this key's matches must meet
    /// this ranking to be considered. When `None`, the global threshold
    /// applies.
    pub(crate) threshold: Option<Ranking>,

    /// Maximum ranking this key can contribute. Clamps the rank down so that
    /// a match on this key never exceeds this tier.
    ///
    /// Defaults to [`Ranking::CaseSensitiveEqual`] (no clamping).
    pub(crate) max_ranking: Ranking,

    /// Minimum ranking this key can contribute. Promotes non-`NoMatch`
    /// results up to this tier (but never promotes `NoMatch` itself).
    ///
    /// Defaults to [`Ranking::NoMatch`] (no boosting).
    pub(crate) min_ranking: Ranking,
}

impl<T> Key<T> {
    /// Create a key from a closure that returns zero or more owned strings.
    ///
    /// This is the most general constructor. For single-value extraction,
    /// consider [`Key::from_fn`]; for multi-value borrowed extraction,
    /// consider [`Key::from_fn_multi`].
    ///
    /// # Arguments
    ///
    /// * `extractor` - A closure that, given a reference to an item, returns
    ///   a `Vec<String>` of values to rank against the query.
    ///
    /// # Examples
    ///
    /// ```
    /// use matchsorter::key::Key;
    ///
    /// let key = Key::new(|s: &String| vec![s.clone()]);
    /// ```
    pub fn new<F>(extractor: F) -> Self
    where
        F: Fn(&T) -> Vec<String> + 'static,
    {
        Self {
            extractor: Box::new(extractor),
            threshold: None,
            min_ranking: Ranking::NoMatch,
            max_ranking: Ranking::CaseSensitiveEqual,
        }
    }

    /// Create a key from a closure that returns a single borrowed `&str`.
    ///
    /// The borrowed value is converted to an owned `String` internally.
    /// This is a convenience shorthand equivalent to:
    ///
    /// ```text
    /// Key::new(|item| vec![item.field.to_owned()])
    /// ```
    ///
    /// # Arguments
    ///
    /// * `f` - A closure that, given a reference to an item, returns a
    ///   borrowed string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use matchsorter::key::Key;
    ///
    /// struct User { name: String }
    ///
    /// let key = Key::<User>::from_fn(|u| u.name.as_str());
    /// ```
    pub fn from_fn<F>(f: F) -> Self
    where
        F: Fn(&T) -> &str + 'static,
    {
        Self {
            extractor: Box::new(move |item| vec![f(item).to_owned()]),
            threshold: None,
            min_ranking: Ranking::NoMatch,
            max_ranking: Ranking::CaseSensitiveEqual,
        }
    }

    /// Create a key from a closure that returns multiple borrowed `&str` values.
    ///
    /// Each borrowed value is converted to an owned `String` internally.
    /// This is useful for fields that contain multiple matchable values,
    /// such as a tags array.
    ///
    /// # Arguments
    ///
    /// * `f` - A closure that, given a reference to an item, returns a
    ///   `Vec<&str>` of borrowed string slices.
    ///
    /// # Examples
    ///
    /// ```
    /// use matchsorter::key::Key;
    ///
    /// struct Article { tags: Vec<String> }
    ///
    /// let key = Key::<Article>::from_fn_multi(|a| {
    ///     a.tags.iter().map(|t| t.as_str()).collect()
    /// });
    /// ```
    pub fn from_fn_multi<F>(f: F) -> Self
    where
        F: Fn(&T) -> Vec<&str> + 'static,
    {
        Self {
            extractor: Box::new(move |item| f(item).into_iter().map(|s| s.to_owned()).collect()),
            threshold: None,
            min_ranking: Ranking::NoMatch,
            max_ranking: Ranking::CaseSensitiveEqual,
        }
    }

    /// Set a per-key threshold override.
    ///
    /// When set, matches produced by this key must meet or exceed the given
    /// ranking to be considered. Matches below the threshold are treated as
    /// `NoMatch` for this key.
    ///
    /// # Arguments
    ///
    /// * `ranking` - The minimum ranking a match must achieve on this key.
    ///
    /// # Examples
    ///
    /// ```
    /// use matchsorter::key::Key;
    /// use matchsorter::Ranking;
    ///
    /// let key = Key::new(|s: &String| vec![s.clone()])
    ///     .threshold(Ranking::StartsWith);
    /// ```
    #[must_use]
    pub fn threshold(mut self, ranking: Ranking) -> Self {
        self.threshold = Some(ranking);
        self
    }

    /// Set the maximum ranking this key can contribute.
    ///
    /// The ranking produced by this key is clamped down to at most this
    /// value. For example, setting `max_ranking` to [`Ranking::Contains`]
    /// means this key can never produce `StartsWith`, `Equal`, or
    /// `CaseSensitiveEqual`.
    ///
    /// Defaults to [`Ranking::CaseSensitiveEqual`] (no clamping).
    ///
    /// # Arguments
    ///
    /// * `ranking` - The ceiling for rankings produced by this key.
    ///
    /// # Examples
    ///
    /// ```
    /// use matchsorter::key::Key;
    /// use matchsorter::Ranking;
    ///
    /// let key = Key::new(|s: &String| vec![s.clone()])
    ///     .max_ranking(Ranking::Contains);
    /// ```
    #[must_use]
    pub fn max_ranking(mut self, ranking: Ranking) -> Self {
        self.max_ranking = ranking;
        self
    }

    /// Set the minimum ranking this key can contribute.
    ///
    /// Non-`NoMatch` results are promoted up to at least this ranking.
    /// A `NoMatch` result is never promoted -- an item that does not match
    /// at all stays `NoMatch` regardless of this setting.
    ///
    /// Defaults to [`Ranking::NoMatch`] (no boosting).
    ///
    /// # Arguments
    ///
    /// * `ranking` - The floor for non-`NoMatch` rankings produced by this key.
    ///
    /// # Examples
    ///
    /// ```
    /// use matchsorter::key::Key;
    /// use matchsorter::Ranking;
    ///
    /// let key = Key::new(|s: &String| vec![s.clone()])
    ///     .min_ranking(Ranking::Contains);
    /// ```
    #[must_use]
    pub fn min_ranking(mut self, ranking: Ranking) -> Self {
        self.min_ranking = ranking;
        self
    }

    /// Extract string values from an item using this key's extractor closure.
    ///
    /// # Arguments
    ///
    /// * `item` - A reference to the item to extract values from.
    ///
    /// # Returns
    ///
    /// A `Vec<String>` of extracted values. An empty vector means the item
    /// produces no match candidates for this key.
    ///
    /// # Examples
    ///
    /// ```
    /// use matchsorter::key::Key;
    ///
    /// let key = Key::new(|s: &String| vec![s.clone()]);
    /// let values = key.extract(&"hello".to_owned());
    /// assert_eq!(values, vec!["hello"]);
    /// ```
    pub fn extract(&self, item: &T) -> Vec<String> {
        (self.extractor)(item)
    }

    /// Returns the per-key threshold override, if set.
    ///
    /// When `Some`, matches on this key must meet or exceed this ranking.
    /// When `None`, the global threshold applies.
    pub fn threshold_value(&self) -> Option<&Ranking> {
        self.threshold.as_ref()
    }

    /// Returns the maximum ranking this key can contribute.
    pub fn max_ranking_value(&self) -> &Ranking {
        &self.max_ranking
    }

    /// Returns the minimum ranking this key can contribute.
    pub fn min_ranking_value(&self) -> &Ranking {
        &self.min_ranking
    }
}

/// The result of ranking a single item against a query across all keys.
///
/// Captures which key and value produced the best match, along with the
/// resulting ranking and any per-key threshold that applied.
///
/// # Examples
///
/// ```
/// use matchsorter::key::RankingInfo;
/// use matchsorter::Ranking;
///
/// let info = RankingInfo {
///     rank: Ranking::Contains,
///     ranked_value: "hello".to_owned(),
///     key_index: 0,
///     key_threshold: None,
/// };
/// assert_eq!(info.rank, Ranking::Contains);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RankingInfo {
    /// The ranking score for the best-matching key/value combination.
    pub rank: Ranking,

    /// The string value that produced the best match.
    pub ranked_value: String,

    /// Index of the key (in the flattened key-values list) that produced
    /// the best match.
    pub key_index: usize,

    /// Per-key threshold override from the winning key, or `None` if the
    /// key uses the global threshold.
    pub key_threshold: Option<Ranking>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Helper types for testing ---

    #[derive(Debug)]
    struct User {
        name: String,
        email: String,
        tags: Vec<String>,
    }

    fn sample_user() -> User {
        User {
            name: "Alice".to_owned(),
            email: "alice@example.com".to_owned(),
            tags: vec!["admin".to_owned(), "staff".to_owned()],
        }
    }

    // --- Key::new tests ---

    #[test]
    fn new_accepts_closure_returning_vec_string() {
        let key = Key::new(|u: &User| vec![u.name.clone()]);
        let values = key.extract(&sample_user());
        assert_eq!(values, vec!["Alice"]);
    }

    #[test]
    fn new_default_threshold_is_none() {
        let key = Key::new(|_: &User| vec![]);
        assert_eq!(key.threshold, None);
    }

    #[test]
    fn new_default_min_ranking_is_no_match() {
        let key = Key::new(|_: &User| vec![]);
        assert_eq!(key.min_ranking, Ranking::NoMatch);
    }

    #[test]
    fn new_default_max_ranking_is_case_sensitive_equal() {
        let key = Key::new(|_: &User| vec![]);
        assert_eq!(key.max_ranking, Ranking::CaseSensitiveEqual);
    }

    // --- Key::from_fn tests ---

    #[test]
    fn from_fn_single_value_extraction() {
        let key = Key::<User>::from_fn(|u| u.name.as_str());
        let values = key.extract(&sample_user());
        assert_eq!(values, vec!["Alice"]);
    }

    #[test]
    fn from_fn_equivalent_to_new_with_vec() {
        let user = sample_user();
        let key_new = Key::new(|u: &User| vec![u.name.clone()]);
        let key_fn = Key::<User>::from_fn(|u| u.name.as_str());

        let values_new = key_new.extract(&user);
        let values_fn = key_fn.extract(&user);
        assert_eq!(values_new, values_fn);
    }

    #[test]
    fn from_fn_default_attributes() {
        let key = Key::<User>::from_fn(|u| u.name.as_str());
        assert_eq!(key.threshold, None);
        assert_eq!(key.min_ranking, Ranking::NoMatch);
        assert_eq!(key.max_ranking, Ranking::CaseSensitiveEqual);
    }

    // --- Key::from_fn_multi tests ---

    #[test]
    fn from_fn_multi_extracts_multiple_values() {
        let key = Key::<User>::from_fn_multi(|u| u.tags.iter().map(|t| t.as_str()).collect());
        let values = key.extract(&sample_user());
        assert_eq!(values, vec!["admin", "staff"]);
    }

    #[test]
    fn from_fn_multi_default_attributes() {
        let key = Key::<User>::from_fn_multi(|u| u.tags.iter().map(|t| t.as_str()).collect());
        assert_eq!(key.threshold, None);
        assert_eq!(key.min_ranking, Ranking::NoMatch);
        assert_eq!(key.max_ranking, Ranking::CaseSensitiveEqual);
    }

    #[test]
    fn from_fn_multi_empty_vec() {
        let key = Key::<User>::from_fn_multi(|_| vec![]);
        let values = key.extract(&sample_user());
        assert!(values.is_empty());
    }

    // --- Builder method tests ---

    #[test]
    fn threshold_sets_value() {
        let key = Key::new(|_: &User| vec![]).threshold(Ranking::StartsWith);
        assert_eq!(key.threshold, Some(Ranking::StartsWith));
    }

    #[test]
    fn max_ranking_sets_value() {
        let key = Key::new(|_: &User| vec![]).max_ranking(Ranking::Contains);
        assert_eq!(key.max_ranking, Ranking::Contains);
    }

    #[test]
    fn min_ranking_sets_value() {
        let key = Key::new(|_: &User| vec![]).min_ranking(Ranking::Contains);
        assert_eq!(key.min_ranking, Ranking::Contains);
    }

    #[test]
    fn builder_chain_all_three() {
        let key = Key::new(|u: &User| vec![u.email.clone()])
            .threshold(Ranking::Acronym)
            .max_ranking(Ranking::Equal)
            .min_ranking(Ranking::Contains);

        assert_eq!(key.threshold, Some(Ranking::Acronym));
        assert_eq!(key.max_ranking, Ranking::Equal);
        assert_eq!(key.min_ranking, Ranking::Contains);
    }

    #[test]
    fn builder_chain_preserves_extractor() {
        let key = Key::new(|u: &User| vec![u.name.clone()])
            .threshold(Ranking::StartsWith)
            .max_ranking(Ranking::Contains)
            .min_ranking(Ranking::Acronym);

        let values = key.extract(&sample_user());
        assert_eq!(values, vec!["Alice"]);
    }

    #[test]
    fn builder_from_fn_with_chain() {
        let key = Key::<User>::from_fn(|u| u.email.as_str())
            .threshold(Ranking::WordStartsWith)
            .max_ranking(Ranking::StartsWith);

        assert_eq!(key.threshold, Some(Ranking::WordStartsWith));
        assert_eq!(key.max_ranking, Ranking::StartsWith);
        // min_ranking left at default
        assert_eq!(key.min_ranking, Ranking::NoMatch);

        let values = key.extract(&sample_user());
        assert_eq!(values, vec!["alice@example.com"]);
    }

    #[test]
    fn builder_from_fn_multi_with_chain() {
        let key = Key::<User>::from_fn_multi(|u| u.tags.iter().map(|t| t.as_str()).collect())
            .min_ranking(Ranking::Contains);

        assert_eq!(key.min_ranking, Ranking::Contains);
        assert_eq!(key.threshold, None);
        assert_eq!(key.max_ranking, Ranking::CaseSensitiveEqual);

        let values = key.extract(&sample_user());
        assert_eq!(values, vec!["admin", "staff"]);
    }

    // --- Builder override tests ---

    #[test]
    fn builder_last_call_wins_for_same_method() {
        let key = Key::new(|_: &User| vec![])
            .threshold(Ranking::Contains)
            .threshold(Ranking::StartsWith);
        assert_eq!(key.threshold, Some(Ranking::StartsWith));
    }

    #[test]
    fn builder_matches_variant_in_threshold() {
        // Ensure Matches(f64) variant works in builder methods.
        let key = Key::new(|_: &User| vec![])
            .threshold(Ranking::Matches(1.5))
            .min_ranking(Ranking::Matches(1.2))
            .max_ranking(Ranking::Matches(1.8));

        assert_eq!(key.threshold, Some(Ranking::Matches(1.5)));
        assert_eq!(key.min_ranking, Ranking::Matches(1.2));
        assert_eq!(key.max_ranking, Ranking::Matches(1.8));
    }

    // --- RankingInfo tests ---

    #[test]
    fn ranking_info_construction() {
        let info = RankingInfo {
            rank: Ranking::Contains,
            ranked_value: "hello".to_owned(),
            key_index: 2,
            key_threshold: Some(Ranking::StartsWith),
        };

        assert_eq!(info.rank, Ranking::Contains);
        assert_eq!(info.ranked_value, "hello");
        assert_eq!(info.key_index, 2);
        assert_eq!(info.key_threshold, Some(Ranking::StartsWith));
    }

    #[test]
    fn ranking_info_with_no_threshold() {
        let info = RankingInfo {
            rank: Ranking::Equal,
            ranked_value: "world".to_owned(),
            key_index: 0,
            key_threshold: None,
        };

        assert_eq!(info.key_threshold, None);
    }

    #[test]
    fn ranking_info_debug_formatting() {
        let info = RankingInfo {
            rank: Ranking::Acronym,
            ranked_value: "test".to_owned(),
            key_index: 1,
            key_threshold: None,
        };
        let debug_str = format!("{info:?}");
        assert!(debug_str.contains("Acronym"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn ranking_info_clone() {
        let info = RankingInfo {
            rank: Ranking::StartsWith,
            ranked_value: "cloned".to_owned(),
            key_index: 3,
            key_threshold: Some(Ranking::Contains),
        };
        let cloned = info.clone();
        assert_eq!(info, cloned);
    }

    #[test]
    fn ranking_info_partial_eq() {
        let a = RankingInfo {
            rank: Ranking::Contains,
            ranked_value: "val".to_owned(),
            key_index: 0,
            key_threshold: None,
        };
        let b = RankingInfo {
            rank: Ranking::Contains,
            ranked_value: "val".to_owned(),
            key_index: 0,
            key_threshold: None,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn ranking_info_partial_eq_different_rank() {
        let a = RankingInfo {
            rank: Ranking::Contains,
            ranked_value: "val".to_owned(),
            key_index: 0,
            key_threshold: None,
        };
        let b = RankingInfo {
            rank: Ranking::Equal,
            ranked_value: "val".to_owned(),
            key_index: 0,
            key_threshold: None,
        };
        assert_ne!(a, b);
    }

    // --- Key with primitive types ---

    #[test]
    fn key_with_string_type() {
        let key = Key::new(|s: &String| vec![s.clone()]);
        let values = key.extract(&"hello world".to_owned());
        assert_eq!(values, vec!["hello world"]);
    }

    #[test]
    fn from_fn_with_string_type() {
        let key = Key::<String>::from_fn(|s| s.as_str());
        let values = key.extract(&"test".to_owned());
        assert_eq!(values, vec!["test"]);
    }

    // --- get_item_values tests ---

    #[test]
    fn get_item_values_single_value() {
        let key = Key::<User>::from_fn(|u| u.name.as_str());
        let values = get_item_values(&sample_user(), &key);
        assert_eq!(values, vec!["Alice"]);
    }

    #[test]
    fn get_item_values_multi_value() {
        let key = Key::<User>::from_fn_multi(|u| u.tags.iter().map(|t| t.as_str()).collect());
        let values = get_item_values(&sample_user(), &key);
        assert_eq!(values, vec!["admin", "staff"]);
    }

    #[test]
    fn get_item_values_empty() {
        let key = Key::new(|_: &User| vec![]);
        let values = get_item_values(&sample_user(), &key);
        assert!(values.is_empty());
    }

    // --- get_highest_ranking tests ---

    fn default_opts<T>() -> MatchSorterOptions<T> {
        MatchSorterOptions::default()
    }

    #[test]
    fn highest_ranking_single_key_exact_match() {
        // "Alice" queried with "Alice" -> CaseSensitiveEqual
        let keys = vec![Key::new(|u: &User| vec![u.name.clone()])];
        let info = get_highest_ranking(&sample_user(), &keys, "Alice", &default_opts());
        assert_eq!(info.rank, Ranking::CaseSensitiveEqual);
        assert_eq!(info.ranked_value, "Alice");
        assert_eq!(info.key_index, 0);
        assert_eq!(info.key_threshold, None);
    }

    #[test]
    fn highest_ranking_picks_best_across_multiple_keys() {
        // Key 0 extracts email ("alice@example.com"), query "Alice" -> Contains
        // Key 1 extracts name ("Alice"), query "Alice" -> CaseSensitiveEqual
        let keys: Vec<Key<User>> = vec![
            Key::new(|u: &User| vec![u.email.clone()]),
            Key::new(|u: &User| vec![u.name.clone()]),
        ];
        let info = get_highest_ranking(&sample_user(), &keys, "Alice", &default_opts());
        assert_eq!(info.rank, Ranking::CaseSensitiveEqual);
        assert_eq!(info.ranked_value, "Alice");
        // Key 0 produces 1 value (index 0), key 1 produces 1 value (index 1)
        assert_eq!(info.key_index, 1);
    }

    #[test]
    fn highest_ranking_max_ranking_clamps_down() {
        // "Alice" queried with "Alice" would normally be CaseSensitiveEqual,
        // but max_ranking = Contains clamps it down to Contains.
        let keys = vec![Key::new(|u: &User| vec![u.name.clone()]).max_ranking(Ranking::Contains)];
        let info = get_highest_ranking(&sample_user(), &keys, "Alice", &default_opts());
        assert_eq!(info.rank, Ranking::Contains);
    }

    #[test]
    fn highest_ranking_max_ranking_clamps_starts_with_to_contains() {
        // "Alice" queried with "ali" -> StartsWith normally, clamped to Contains
        let keys = vec![Key::new(|u: &User| vec![u.name.clone()]).max_ranking(Ranking::Contains)];
        let info = get_highest_ranking(&sample_user(), &keys, "ali", &default_opts());
        assert_eq!(info.rank, Ranking::Contains);
    }

    #[test]
    fn highest_ranking_min_ranking_promotes_matches_to_contains() {
        // "playground" queried with "plgnd" -> Matches(~1.11) normally.
        // min_ranking = Contains promotes it up to Contains.
        let item = "playground".to_owned();
        let keys = vec![Key::new(|s: &String| vec![s.clone()]).min_ranking(Ranking::Contains)];
        let info = get_highest_ranking(&item, &keys, "plgnd", &default_opts());
        assert_eq!(info.rank, Ranking::Contains);
    }

    #[test]
    fn highest_ranking_min_ranking_does_not_promote_no_match() {
        // "abc" queried with "xyz" -> NoMatch. min_ranking = Contains should
        // NOT promote it; NoMatch stays NoMatch.
        let item = "abc".to_owned();
        let keys = vec![Key::new(|s: &String| vec![s.clone()]).min_ranking(Ranking::Contains)];
        let info = get_highest_ranking(&item, &keys, "xyz", &default_opts());
        assert_eq!(info.rank, Ranking::NoMatch);
    }

    #[test]
    fn highest_ranking_tie_break_lower_key_index_wins() {
        // Both keys produce the same value "Alice" with the same ranking.
        // The first key's value (key_index 0) should win.
        let keys: Vec<Key<User>> = vec![
            Key::new(|u: &User| vec![u.name.clone()]),
            Key::new(|u: &User| vec![u.name.clone()]),
        ];
        let info = get_highest_ranking(&sample_user(), &keys, "Alice", &default_opts());
        assert_eq!(info.rank, Ranking::CaseSensitiveEqual);
        assert_eq!(info.key_index, 0);
    }

    #[test]
    fn highest_ranking_tie_break_with_clamping() {
        // Key 0: name "Alice", max_ranking = Contains -> clamped to Contains
        // Key 1: email "alice@example.com", query "alice" -> StartsWith,
        //        max_ranking = Contains -> clamped to Contains
        // Both produce Contains, but key_index 0 wins.
        let keys: Vec<Key<User>> = vec![
            Key::new(|u: &User| vec![u.name.clone()]).max_ranking(Ranking::Contains),
            Key::new(|u: &User| vec![u.email.clone()]).max_ranking(Ranking::Contains),
        ];
        let info = get_highest_ranking(&sample_user(), &keys, "alice", &default_opts());
        assert_eq!(info.rank, Ranking::Contains);
        assert_eq!(info.key_index, 0);
        assert_eq!(info.ranked_value, "Alice");
    }

    #[test]
    fn highest_ranking_key_threshold_reflected() {
        // Key with a threshold set -- the returned RankingInfo should have
        // the key's threshold in key_threshold.
        let keys = vec![Key::new(|u: &User| vec![u.name.clone()]).threshold(Ranking::StartsWith)];
        let info = get_highest_ranking(&sample_user(), &keys, "Alice", &default_opts());
        assert_eq!(info.key_threshold, Some(Ranking::StartsWith));
    }

    #[test]
    fn highest_ranking_key_threshold_none_when_not_set() {
        let keys = vec![Key::new(|u: &User| vec![u.name.clone()])];
        let info = get_highest_ranking(&sample_user(), &keys, "Alice", &default_opts());
        assert_eq!(info.key_threshold, None);
    }

    #[test]
    fn highest_ranking_multi_value_key_best_value_wins() {
        // Key extracts tags: ["admin", "staff"]. Query "admin" matches
        // "admin" as CaseSensitiveEqual but "staff" as NoMatch.
        // Best should be CaseSensitiveEqual for "admin".
        let keys = vec![Key::new(|u: &User| u.tags.clone())];
        let info = get_highest_ranking(&sample_user(), &keys, "admin", &default_opts());
        assert_eq!(info.rank, Ranking::CaseSensitiveEqual);
        assert_eq!(info.ranked_value, "admin");
        assert_eq!(info.key_index, 0);
    }

    #[test]
    fn highest_ranking_flattened_index_across_keys() {
        // Key 0 extracts tags: ["admin", "staff"] (indices 0, 1)
        // Key 1 extracts name: ["Alice"] (index 2)
        // Query "Alice" matches name at CaseSensitiveEqual (best).
        let keys: Vec<Key<User>> = vec![
            Key::new(|u: &User| u.tags.clone()),
            Key::new(|u: &User| vec![u.name.clone()]),
        ];
        let info = get_highest_ranking(&sample_user(), &keys, "Alice", &default_opts());
        assert_eq!(info.rank, Ranking::CaseSensitiveEqual);
        assert_eq!(info.key_index, 2);
    }

    #[test]
    fn highest_ranking_no_keys_returns_no_match() {
        let keys: Vec<Key<User>> = vec![];
        let info = get_highest_ranking(&sample_user(), &keys, "Alice", &default_opts());
        assert_eq!(info.rank, Ranking::NoMatch);
    }

    #[test]
    fn highest_ranking_empty_extractor_returns_no_match() {
        let keys = vec![Key::new(|_: &User| vec![])];
        let info = get_highest_ranking(&sample_user(), &keys, "Alice", &default_opts());
        assert_eq!(info.rank, Ranking::NoMatch);
    }

    #[test]
    fn highest_ranking_max_ranking_does_not_affect_lower_ranks() {
        // max_ranking = StartsWith. Query "admin" vs "admin" -> CaseSensitiveEqual,
        // clamped to StartsWith. But if the natural rank is Contains (lower
        // than max_ranking), no clamping occurs.
        let item = "xxadminxx".to_owned();
        let keys = vec![Key::new(|s: &String| vec![s.clone()]).max_ranking(Ranking::StartsWith)];
        // "xxadminxx" contains "admin" -> Contains, which is below StartsWith
        let info = get_highest_ranking(&item, &keys, "admin", &default_opts());
        assert_eq!(info.rank, Ranking::Contains);
    }

    #[test]
    fn highest_ranking_min_ranking_does_not_affect_higher_ranks() {
        // min_ranking = Contains. If the rank is already above Contains
        // (e.g., StartsWith), it should not be affected.
        let keys = vec![Key::new(|u: &User| vec![u.name.clone()]).min_ranking(Ranking::Contains)];
        // "Alice" queried with "ali" -> StartsWith, which is above Contains
        let info = get_highest_ranking(&sample_user(), &keys, "ali", &default_opts());
        assert_eq!(info.rank, Ranking::StartsWith);
    }

    #[test]
    fn highest_ranking_both_clamps_applied() {
        // max_ranking = Contains and min_ranking = Contains effectively
        // forces all non-NoMatch results to exactly Contains.
        let keys = vec![
            Key::new(|u: &User| vec![u.name.clone()])
                .min_ranking(Ranking::Contains)
                .max_ranking(Ranking::Contains),
        ];
        // "Alice" queried with "Alice" -> CaseSensitiveEqual, clamped to Contains
        let info = get_highest_ranking(&sample_user(), &keys, "Alice", &default_opts());
        assert_eq!(info.rank, Ranking::Contains);
    }

    #[test]
    fn highest_ranking_winning_key_threshold_from_correct_key() {
        // Key 0 has threshold StartsWith, extracts email -> Contains for "alice"
        // Key 1 has threshold Acronym, extracts name -> CaseSensitiveEqual for "Alice"
        // Key 1 wins, so key_threshold should be Acronym (key 1's threshold).
        let keys: Vec<Key<User>> = vec![
            Key::new(|u: &User| vec![u.email.clone()]).threshold(Ranking::StartsWith),
            Key::new(|u: &User| vec![u.name.clone()]).threshold(Ranking::Acronym),
        ];
        let info = get_highest_ranking(&sample_user(), &keys, "Alice", &default_opts());
        assert_eq!(info.rank, Ranking::CaseSensitiveEqual);
        assert_eq!(info.key_threshold, Some(Ranking::Acronym));
    }

    #[test]
    fn highest_ranking_keep_diacritics_option_passed() {
        // "cafe" + combining acute = "caf\u{e9}" equivalent. Query "cafe" with
        // keep_diacritics=true means they won't match (different chars).
        let item = "caf\u{e9}".to_owned();
        let keys = vec![Key::new(|s: &String| vec![s.clone()])];

        let opts_strip = MatchSorterOptions {
            keep_diacritics: false,
            ..Default::default()
        };
        let info_strip = get_highest_ranking(&item, &keys, "cafe", &opts_strip);
        assert_eq!(info_strip.rank, Ranking::CaseSensitiveEqual);

        let opts_keep = MatchSorterOptions {
            keep_diacritics: true,
            ..Default::default()
        };
        let info_keep = get_highest_ranking(&item, &keys, "cafe", &opts_keep);
        assert_eq!(info_keep.rank, Ranking::NoMatch);
    }
}

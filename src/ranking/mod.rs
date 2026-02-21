//! Ranking tiers and scoring logic for string matching.
//!
//! This module implements the 8-tier ranking system that determines how well
//! a candidate string matches a search query, from exact case-sensitive
//! equality down to fuzzy character-by-character matching.

use std::borrow::Cow;

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// Represents the quality of a match between a candidate string and a query.
///
/// The ranking system has 8 tiers ordered from best to worst:
///
/// | Tier                 | Value | Description                                  |
/// |----------------------|-------|----------------------------------------------|
/// | `CaseSensitiveEqual` | 7     | Exact byte-for-byte match                    |
/// | `Equal`              | 6     | Case-insensitive full match                  |
/// | `StartsWith`         | 5     | Candidate starts with query (case-insensitive)|
/// | `WordStartsWith`     | 4     | A word in the candidate starts with query    |
/// | `Contains`           | 3     | Candidate contains query as substring        |
/// | `Acronym`            | 2     | Query matches the candidate's acronym        |
/// | `Matches(f64)`       | 1..2  | Fuzzy in-order character match with sub-score|
/// | `NoMatch`            | 0     | No match found                               |
///
/// # Sub-score invariant for `Matches`
///
/// The `Matches` variant carries a continuous sub-score that should fall in the
/// range `(1.0, 2.0]` by convention. This is not enforced at runtime; callers
/// are responsible for producing valid sub-scores. A higher sub-score indicates
/// a tighter fuzzy match (characters found closer together).
///
/// # Ordering
///
/// `Ranking` implements [`PartialOrd`] such that higher-quality matches compare
/// as greater. For two `Matches` variants, the one with the higher sub-score
/// is greater.
#[derive(Debug, Clone, Copy)]
pub enum Ranking {
    /// Exact byte-for-byte match (tier 7).
    CaseSensitiveEqual,
    /// Case-insensitive full match (tier 6).
    Equal,
    /// Candidate starts with the query, case-insensitively (tier 5).
    StartsWith,
    /// A word boundary within the candidate starts with the query (tier 4).
    WordStartsWith,
    /// Candidate contains the query as a substring (tier 3).
    Contains,
    /// Query matches the candidate's acronym (tier 2).
    Acronym,
    /// Fuzzy in-order character match with a continuous sub-score in `(1.0, 2.0]` (tier 1..2).
    ///
    /// A higher sub-score means the matched characters are closer together,
    /// indicating a tighter match. The sub-score is computed as
    /// `1.0 + 1.0 / spread` where `spread` is the distance between the first
    /// and last matched character positions.
    Matches(f64),
    /// No match found (tier 0).
    NoMatch,
}

impl Ranking {
    /// Returns the integer tier value for this ranking.
    ///
    /// Fixed tiers return their integer value (0-7). The `Matches` variant
    /// returns 1, since its effective value is the continuous sub-score
    /// stored in the variant (which falls in `(1.0, 2.0]`).
    fn tier_value(&self) -> u8 {
        match self {
            Ranking::CaseSensitiveEqual => 7,
            Ranking::Equal => 6,
            Ranking::StartsWith => 5,
            Ranking::WordStartsWith => 4,
            Ranking::Contains => 3,
            Ranking::Acronym => 2,
            // Matches uses the sub-score for ordering, but its base tier is 1.
            Ranking::Matches(_) => 1,
            Ranking::NoMatch => 0,
        }
    }
}

// Manual `PartialEq` because `f64` does not implement `Eq`, and we want
// comparison semantics that match our ordering (two `Matches` variants are
// equal iff their sub-scores are equal).
impl PartialEq for Ranking {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Ranking::Matches(a), Ranking::Matches(b)) => a == b,
            _ => self.tier_value() == other.tier_value(),
        }
    }
}

// Manual `PartialOrd` to enable ranking comparisons. Higher-quality matches
// compare as greater. Fixed tiers are compared by their integer tier value.
// Two `Matches` variants are compared by their sub-scores. A `Matches`
// variant vs. a fixed tier is compared by tier value (where `Matches` has
// tier 1), ensuring fixed tiers like `Acronym` (tier 2) always outrank
// `Matches` even at its maximum sub-score of 2.0.
impl PartialOrd for Ranking {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            // Both are Matches: compare by sub-score directly.
            (Ranking::Matches(a), Ranking::Matches(b)) => a.partial_cmp(b),
            // All other cases: compare by integer tier value.
            _ => self.tier_value().partial_cmp(&other.tier_value()),
        }
    }
}

/// Compute a fuzzy closeness ranking via greedy forward character matching.
///
/// For each character in `query`, scans forward through `candidate` to find it.
/// If all query characters are found in order, the ranking is based on how
/// closely together they appear (the "spread").
///
/// # Arguments
///
/// * `candidate` - The string being evaluated
/// * `query` - The search query whose characters must appear in order
///
/// # Returns
///
/// - [`Ranking::NoMatch`] if any query character cannot be found in order
/// - `Ranking::Matches(2.0)` when `spread == 0` (single-character query)
/// - `Ranking::Matches(1.0 + 1.0 / spread)` otherwise, where `spread` is
///   the distance (in char positions) between the first and last matched
///   characters. The result is always in the range `(1.0, 2.0]`.
///
/// # Examples
///
/// ```
/// use matchsorter::ranking::{get_closeness_ranking, Ranking};
///
/// // Fuzzy match: chars spread across the candidate
/// let rank = get_closeness_ranking("playground", "plgnd");
/// assert!(matches!(rank, Ranking::Matches(s) if s > 1.0 && s < 2.0));
///
/// // No match: query chars not present
/// assert_eq!(get_closeness_ranking("abc", "xyz"), Ranking::NoMatch);
///
/// // Single char: spread is 0, returns upper-bound score
/// assert_eq!(get_closeness_ranking("ab", "a"), Ranking::Matches(2.0));
/// ```
pub fn get_closeness_ranking(candidate: &str, query: &str) -> Ranking {
    // Tracks our position as we scan forward through the candidate.
    // `.chars()` gives us an iterator over Unicode scalar values, which is
    // critical for correct character-by-character matching.
    let mut candidate_chars = candidate.chars().enumerate();

    let mut first_match_index: Option<usize> = None;
    let mut last_match_index: usize = 0;

    for query_char in query.chars() {
        // Scan forward through the remaining candidate characters to find
        // the next occurrence of `query_char`. This greedy approach mirrors
        // the JS `findMatchingCharacter` function.
        let found = candidate_chars.find(|&(_, c)| c == query_char);

        match found {
            Some((pos, _)) => {
                if first_match_index.is_none() {
                    first_match_index = Some(pos);
                }
                last_match_index = pos;
            }
            None => return Ranking::NoMatch,
        }
    }

    // `first_match_index` is `None` only if `query` was empty.
    // An empty query trivially matches with spread 0.
    let first = first_match_index.unwrap_or(0);
    let spread = last_match_index - first;

    if spread == 0 {
        // Single-character query or empty query: return the upper bound.
        // In the JS version this case produces `Infinity` from `1/0`,
        // which is clamped. We use 2.0 as a safe maximum.
        Ranking::Matches(2.0)
    } else {
        Ranking::Matches(1.0 + 1.0 / spread as f64)
    }
}

/// Returns whether `c` is an acronym word-boundary delimiter.
///
/// Only space (`' '`) and hyphen (`'-'`) are recognized as delimiters.
fn is_acronym_delimiter(c: char) -> bool {
    c == ' ' || c == '-'
}

/// Extract the acronym from a string by collecting word-initial characters.
///
/// Word boundaries are space (`' '`) and hyphen (`'-'`) only. The first
/// character is always included; subsequent characters are included when
/// the previous character was a delimiter and the current character is not
/// itself a delimiter. The caller is responsible for lowercasing the input
/// before calling.
///
/// # Arguments
///
/// * `s` - The input string to extract an acronym from
///
/// # Returns
///
/// A `String` containing the first character of each word
///
/// # Examples
///
/// ```
/// use matchsorter::ranking::get_acronym;
///
/// assert_eq!(get_acronym("north-west airlines"), "nwa");
/// assert_eq!(get_acronym("san francisco"), "sf");
/// assert_eq!(get_acronym("single"), "s");
/// assert_eq!(get_acronym(""), "");
/// ```
pub fn get_acronym(s: &str) -> String {
    let mut chars = s.chars();

    // Empty string produces an empty acronym.
    let first = match chars.next() {
        Some(c) => c,
        None => return String::new(),
    };

    // Estimate capacity: one char per word. Use memchr for a fast count of
    // delimiter bytes (space and hyphen are single-byte ASCII).
    let word_count_estimate = 1 + memchr::memchr2_iter(b' ', b'-', s.as_bytes()).count();
    let mut acronym = String::with_capacity(word_count_estimate);

    // First character is always included (virtual leading delimiter).
    acronym.push(first);

    // Track the previous character to detect word boundaries.
    let mut prev = first;
    for c in chars {
        if is_acronym_delimiter(prev) && !is_acronym_delimiter(c) {
            acronym.push(c);
        }
        prev = c;
    }

    acronym
}

/// Prepare a string for comparison by optionally stripping diacritics.
///
/// When `keep_diacritics` is `false`, applies Unicode NFD decomposition and
/// removes combining marks (`General_Category = Mark`), effectively stripping
/// accents and diacritical marks from characters. When `keep_diacritics` is
/// `true`, the original string is returned unchanged.
///
/// Returns [`Cow::Borrowed`] when no modification is needed (either because
/// `keep_diacritics` is `true`, or because the string contains no diacritics
/// to strip). Only allocates a new `String` ([`Cow::Owned`]) when characters
/// are actually removed.
///
/// # Arguments
///
/// * `s` - The input string to prepare
/// * `keep_diacritics` - If `true`, skip diacritics stripping entirely
///
/// # Returns
///
/// A [`Cow<'_, str>`] that is either borrowed from the input or an owned
/// string with combining marks removed.
///
/// # Examples
///
/// ```
/// use matchsorter::ranking::prepare_value_for_comparison;
///
/// // Stripping an accent produces a new string
/// let result = prepare_value_for_comparison("cafe\u{0301}", false);
/// assert_eq!(result, "cafe");
/// assert!(matches!(result, std::borrow::Cow::Owned(_)));
///
/// // ASCII strings are returned borrowed (no allocation)
/// let result = prepare_value_for_comparison("cafe", false);
/// assert_eq!(result, "cafe");
/// assert!(matches!(result, std::borrow::Cow::Borrowed(_)));
///
/// // With keep_diacritics=true, the original is always returned
/// let result = prepare_value_for_comparison("cafe\u{0301}", true);
/// assert_eq!(result, "cafe\u{0301}");
/// assert!(matches!(result, std::borrow::Cow::Borrowed(_)));
/// ```
pub fn prepare_value_for_comparison(s: &str, keep_diacritics: bool) -> Cow<'_, str> {
    if keep_diacritics {
        return Cow::Borrowed(s);
    }

    // Fast path: ASCII strings never contain diacritics or combining marks.
    if s.is_ascii() {
        return Cow::Borrowed(s);
    }

    // Apply NFD decomposition and filter out combining marks (accents,
    // diacritical marks, etc.). NFD splits precomposed characters like
    // U+00E9 (e-acute) into their base letter + combining mark, so
    // filtering the marks effectively strips the diacritics.
    let stripped: String = s.nfd().filter(|c| !is_combining_mark(*c)).collect();

    if stripped == s {
        // NFD + filtering produced the same bytes as the original,
        // so return borrowed to avoid keeping the allocation.
        Cow::Borrowed(s)
    } else {
        Cow::Owned(stripped)
    }
}

/// Pre-computed query data for amortizing repeated per-item ranking calls.
///
/// Caches the prepared (diacritics-stripped) query, its lowercased form,
/// character count, and an ASCII flag so that `match_sorter` can avoid
/// redundant work when ranking thousands of candidates against the same query.
///
/// Constructed once before the ranking loop via [`PreparedQuery::new`] and
/// passed by reference to [`get_match_ranking_prepared`].
pub(crate) struct PreparedQuery {
    /// The query after optional diacritics stripping.
    prepared: String,
    /// Lowercased version of the prepared query.
    pub(crate) lower: String,
    /// Character count of the lowercased query (cached to avoid repeated
    /// `.chars().count()` calls).
    char_count: usize,
}

impl PreparedQuery {
    /// Create a new `PreparedQuery` by preparing and lowercasing the query once.
    ///
    /// # Arguments
    ///
    /// * `query` - The raw search query string
    /// * `keep_diacritics` - If `true`, skip diacritics stripping
    pub(crate) fn new(query: &str, keep_diacritics: bool) -> Self {
        let prepared = prepare_value_for_comparison(query, keep_diacritics).into_owned();
        let lower = prepared.to_lowercase();
        // ASCII fast path: byte length equals character count for ASCII strings.
        let char_count = if lower.is_ascii() {
            lower.len()
        } else {
            lower.chars().count()
        };
        Self {
            prepared,
            lower,
            char_count,
        }
    }
}

/// Lowercase `s` into `buf`, reusing the buffer's allocation.
///
/// When `s` is ASCII, uses a byte-level fast path that avoids Unicode
/// case-mapping tables entirely. For non-ASCII input, falls back to
/// `char::to_lowercase()`.
fn lowercase_into(s: &str, buf: &mut String) {
    buf.clear();
    if s.is_ascii() {
        buf.reserve(s.len());
        // ASCII bytes are single-byte UTF-8, so lowercasing byte-by-byte
        // and casting to char is safe and avoids Unicode lookup tables.
        buf.extend(s.as_bytes().iter().map(|&b| b.to_ascii_lowercase() as char));
    } else {
        buf.reserve(s.len());
        for c in s.chars() {
            for lc in c.to_lowercase() {
                buf.push(lc);
            }
        }
    }
}

/// Inner hot-path ranking function using pre-prepared query data and a
/// reusable candidate buffer.
///
/// This avoids redundant query preparation, lowercasing, and allocation
/// when called repeatedly in a loop (e.g., inside `match_sorter`).
///
/// # Arguments
///
/// * `test_string` - The candidate string being evaluated
/// * `pq` - Pre-computed query data
/// * `keep_diacritics` - If `true`, skip diacritics stripping on the candidate
/// * `candidate_buf` - Reusable buffer for lowercasing the candidate
/// * `finder` - SIMD-accelerated substring searcher for the lowercased query,
///   or `None` when the query is empty (since `memmem` panics on empty needles)
pub(crate) fn get_match_ranking_prepared(
    test_string: &str,
    pq: &PreparedQuery,
    keep_diacritics: bool,
    candidate_buf: &mut String,
    finder: Option<&memchr::memmem::Finder<'_>>,
) -> Ranking {
    // Prepare candidate (strip diacritics if requested).
    let candidate = prepare_value_for_comparison(test_string, keep_diacritics);

    // Step 1: If query has more characters than candidate, no match is possible.
    // ASCII fast path: byte length equals character count for ASCII strings.
    let candidate_char_count = if candidate.is_ascii() {
        candidate.len()
    } else {
        candidate.chars().count()
    };
    if pq.char_count > candidate_char_count {
        return Ranking::NoMatch;
    }

    // Step 2: Case-sensitive exact equality on the prepared strings.
    if *candidate == *pq.prepared {
        return Ranking::CaseSensitiveEqual;
    }

    // Step 3: Lowercase candidate into reusable buffer (avoids allocation).
    lowercase_into(&candidate, candidate_buf);

    // Steps 4-8: Substring search.
    if let Some(finder) = finder {
        // Use SIMD-accelerated memmem for substring search.
        let candidate_bytes = candidate_buf.as_bytes();
        let mut iter = finder.find_iter(candidate_bytes);

        if let Some(first) = iter.next() {
            if first == 0 {
                // Step 5: Substring at byte position 0 with equal byte
                // lengths means the lowercased strings are identical -> Equal.
                if candidate_buf.len() == pq.lower.len() {
                    return Ranking::Equal;
                }
                // Step 6: Starts with query but is longer -> StartsWith.
                return Ranking::StartsWith;
            }

            // Step 7: Check if any match position sits at a word boundary.
            // A word boundary means the byte immediately before the match
            // is a space (0x20). We already know first > 0 here.
            if candidate_bytes[first - 1] == b' ' {
                return Ranking::WordStartsWith;
            }
            // Check remaining match positions lazily.
            for pos in iter {
                if pos > 0 && candidate_bytes[pos - 1] == b' ' {
                    return Ranking::WordStartsWith;
                }
            }

            // Step 8: A substring match exists but not at a word boundary.
            return Ranking::Contains;
        }
    } else {
        // Empty query: always found at position 0.
        if candidate_buf.is_empty() {
            // Both are empty after lowercasing -> Equal.
            return Ranking::Equal;
        }
        return Ranking::StartsWith;
    }

    // No substring match found. Continue to acronym and fuzzy matching.

    // Step 9: Single-character query that was not found as a substring cannot
    // match via acronym or fuzzy.
    if pq.char_count == 1 {
        return Ranking::NoMatch;
    }

    // Step 10: Compute acronym of the lowercased candidate. If the acronym
    // contains the lowercased query as a substring, it is an acronym match.
    let acronym = get_acronym(candidate_buf);
    if acronym.contains(&pq.lower) {
        return Ranking::Acronym;
    }

    // Step 11: Attempt fuzzy closeness ranking on the lowercased strings.
    get_closeness_ranking(candidate_buf, &pq.lower)
}

/// Determine how well a candidate string matches a search query.
///
/// Implements an 11-step algorithm that classifies the match into one of the
/// 8 ranking tiers, checked in descending order of specificity. The first
/// matching tier is returned.
///
/// Both inputs are first prepared via [`prepare_value_for_comparison`] to
/// optionally strip diacritics. Steps 1-2 operate on the prepared strings;
/// steps 3-11 operate on lowercased versions of those strings.
///
/// # Arguments
///
/// * `test_string` - The candidate string being evaluated
/// * `string_to_rank` - The search query
/// * `keep_diacritics` - If `true`, skip diacritics stripping
///
/// # Returns
///
/// The [`Ranking`] tier that best describes how the query matches the candidate.
///
/// # Examples
///
/// ```
/// use matchsorter::{get_match_ranking, Ranking};
///
/// assert_eq!(get_match_ranking("Green", "Green", false), Ranking::CaseSensitiveEqual);
/// assert_eq!(get_match_ranking("Green", "green", false), Ranking::Equal);
/// assert_eq!(get_match_ranking("Greenland", "green", false), Ranking::StartsWith);
/// assert_eq!(get_match_ranking("abc", "xyz", false), Ranking::NoMatch);
/// ```
pub fn get_match_ranking(
    test_string: &str,
    string_to_rank: &str,
    keep_diacritics: bool,
) -> Ranking {
    // Thin wrapper: construct a PreparedQuery for one-off calls.
    let pq = PreparedQuery::new(string_to_rank, keep_diacritics);
    let finder = if pq.lower.is_empty() {
        None
    } else {
        Some(memchr::memmem::Finder::new(pq.lower.as_bytes()))
    };
    let mut buf = String::new();
    get_match_ranking_prepared(test_string, &pq, keep_diacritics, &mut buf, finder.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_tier_ordering_descending() {
        // Verify the complete ordering chain from best to worst.
        assert!(Ranking::CaseSensitiveEqual > Ranking::Equal);
        assert!(Ranking::Equal > Ranking::StartsWith);
        assert!(Ranking::StartsWith > Ranking::WordStartsWith);
        assert!(Ranking::WordStartsWith > Ranking::Contains);
        assert!(Ranking::Contains > Ranking::Acronym);
        assert!(Ranking::Acronym > Ranking::Matches(1.5));
        assert!(Ranking::Matches(1.5) > Ranking::NoMatch);
    }

    #[test]
    fn matches_sub_score_ordering() {
        // Higher sub-score means a better (greater) ranking.
        assert!(Ranking::Matches(1.9) > Ranking::Matches(1.1));
        assert!(Ranking::Matches(2.0) > Ranking::Matches(1.5));
        assert!(Ranking::Matches(1.5) > Ranking::Matches(1.01));
    }

    #[test]
    fn matches_below_acronym_above_no_match() {
        // Any valid Matches sub-score (1.0, 2.0] is below Acronym (2)
        // and above NoMatch (0).
        assert!(Ranking::Acronym > Ranking::Matches(2.0));
        assert!(Ranking::Matches(1.01) > Ranking::NoMatch);
    }

    #[test]
    fn equality_same_fixed_tiers() {
        assert_eq!(Ranking::CaseSensitiveEqual, Ranking::CaseSensitiveEqual);
        assert_eq!(Ranking::Equal, Ranking::Equal);
        assert_eq!(Ranking::NoMatch, Ranking::NoMatch);
    }

    #[test]
    fn equality_same_matches_sub_score() {
        assert_eq!(Ranking::Matches(1.5), Ranking::Matches(1.5));
    }

    #[test]
    fn inequality_different_tiers() {
        assert_ne!(Ranking::CaseSensitiveEqual, Ranking::Equal);
        assert_ne!(Ranking::Matches(1.5), Ranking::NoMatch);
    }

    #[test]
    fn inequality_different_sub_scores() {
        assert_ne!(Ranking::Matches(1.2), Ranking::Matches(1.8));
    }

    #[test]
    fn debug_formatting() {
        // Verify Debug is derived and produces reasonable output.
        let debug_str = format!("{:?}", Ranking::Matches(1.5));
        assert!(debug_str.contains("Matches"));
        assert!(debug_str.contains("1.5"));

        let debug_str = format!("{:?}", Ranking::CaseSensitiveEqual);
        assert!(debug_str.contains("CaseSensitiveEqual"));
    }

    #[test]
    fn copy_produces_equal_value() {
        let original = Ranking::Matches(1.75);
        let copied = original;
        assert_eq!(original, copied);

        let original = Ranking::Contains;
        let copied = original;
        assert_eq!(original, copied);
    }

    #[test]
    fn matches_at_boundary_values() {
        // Sub-score at the upper boundary (2.0) is still below Acronym.
        assert!(Ranking::Acronym > Ranking::Matches(2.0));
        // Sub-score just above the lower boundary is above NoMatch.
        assert!(Ranking::Matches(1.001) > Ranking::NoMatch);
    }

    // --- get_acronym tests ---

    #[test]
    fn acronym_hyphen_and_space() {
        assert_eq!(get_acronym("north-west airlines"), "nwa");
    }

    #[test]
    fn acronym_space_only() {
        assert_eq!(get_acronym("san francisco"), "sf");
    }

    #[test]
    fn acronym_single_word() {
        assert_eq!(get_acronym("single"), "s");
    }

    #[test]
    fn acronym_empty_string() {
        assert_eq!(get_acronym(""), "");
    }

    #[test]
    fn acronym_underscores_not_delimiters() {
        // Underscores do NOT act as word boundaries.
        assert_eq!(get_acronym("snake_case_word"), "s");
    }

    #[test]
    fn acronym_consecutive_spaces() {
        // Consecutive delimiters: only the non-delimiter char after the
        // last delimiter in a run is included.
        assert_eq!(get_acronym("hello  world"), "hw");
    }

    #[test]
    fn acronym_consecutive_hyphens() {
        assert_eq!(get_acronym("a--b"), "ab");
    }

    #[test]
    fn acronym_mixed_delimiters() {
        assert_eq!(get_acronym("one two-three four"), "ottf");
    }

    #[test]
    fn acronym_single_char() {
        assert_eq!(get_acronym("x"), "x");
    }

    #[test]
    fn acronym_trailing_delimiter() {
        // Trailing delimiter produces no extra character.
        assert_eq!(get_acronym("hello "), "h");
    }

    // --- prepare_value_for_comparison tests ---

    #[test]
    fn strips_combining_acute_accent() {
        // "cafe" followed by U+0301 COMBINING ACUTE ACCENT -> "cafe"
        let result = prepare_value_for_comparison("cafe\u{0301}", false);
        assert_eq!(result, "cafe");
        assert!(matches!(result, Cow::Owned(_)));
    }

    #[test]
    fn returns_borrowed_for_plain_ascii() {
        // Pure ASCII with no diacritics should not allocate.
        let result = prepare_value_for_comparison("cafe", false);
        assert_eq!(result, "cafe");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn returns_borrowed_when_keep_diacritics_is_true() {
        // When keep_diacritics is true, the input is returned as-is.
        let input = "cafe\u{0301}";
        let result = prepare_value_for_comparison(input, true);
        assert_eq!(result, input);
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn strips_precomposed_accent() {
        // U+00E9 (LATIN SMALL LETTER E WITH ACUTE) is a single precomposed
        // codepoint that NFD decomposes into 'e' + U+0301.
        let result = prepare_value_for_comparison("caf\u{00E9}", false);
        assert_eq!(result, "cafe");
        assert!(matches!(result, Cow::Owned(_)));
    }

    #[test]
    fn strips_multiple_diacritics() {
        // U+00FC = u with diaeresis, U+00F1 = n with tilde
        let result = prepare_value_for_comparison("\u{00FC}ber-ma\u{00F1}ana", false);
        assert_eq!(result, "uber-manana");
        assert!(matches!(result, Cow::Owned(_)));
    }

    #[test]
    fn returns_borrowed_for_empty_string() {
        let result = prepare_value_for_comparison("", false);
        assert_eq!(result, "");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn returns_borrowed_for_non_ascii_without_diacritics() {
        // CJK characters have no combining marks after NFD, so the stripped
        // result equals the original. Even though these are not ASCII, the
        // function detects no change and returns borrowed.
        let result = prepare_value_for_comparison("\u{4e16}\u{754c}", false);
        assert_eq!(result, "\u{4e16}\u{754c}");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn keep_diacritics_true_with_plain_ascii() {
        let result = prepare_value_for_comparison("hello", true);
        assert_eq!(result, "hello");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn strips_combining_tilde() {
        // 'n' + U+0303 COMBINING TILDE -> "n"
        let result = prepare_value_for_comparison("n\u{0303}", false);
        assert_eq!(result, "n");
        assert!(matches!(result, Cow::Owned(_)));
    }

    #[test]
    fn strips_multiple_combining_marks_on_single_base() {
        // 'a' + U+0300 (grave) + U+0301 (acute) -> "a"
        // Multiple stacked combining marks should all be removed.
        let result = prepare_value_for_comparison("a\u{0300}\u{0301}", false);
        assert_eq!(result, "a");
        assert!(matches!(result, Cow::Owned(_)));
    }

    // --- get_closeness_ranking tests ---

    #[test]
    fn closeness_fuzzy_match_playground() {
        // "plgnd" chars found at positions 0, 1, 4, 8, 9 in "playground".
        // spread = 9 - 0 = 9, score = 1.0 + 1.0/9 ~= 1.111
        let rank = get_closeness_ranking("playground", "plgnd");
        match rank {
            Ranking::Matches(s) => {
                assert!(s > 1.0, "sub-score {s} should be > 1.0");
                assert!(s < 2.0, "sub-score {s} should be < 2.0");
                // Verify the exact expected value.
                let expected = 1.0 + 1.0 / 9.0;
                assert!(
                    (s - expected).abs() < f64::EPSILON,
                    "expected {expected}, got {s}"
                );
            }
            other => panic!("expected Matches, got {other:?}"),
        }
    }

    #[test]
    fn closeness_no_match() {
        assert_eq!(get_closeness_ranking("abc", "xyz"), Ranking::NoMatch);
    }

    #[test]
    fn closeness_single_char_match() {
        // Single char "a" found at position 0 in "ab". spread = 0.
        assert_eq!(get_closeness_ranking("ab", "a"), Ranking::Matches(2.0));
    }

    #[test]
    fn closeness_single_char_not_found() {
        assert_eq!(get_closeness_ranking("ab", "z"), Ranking::NoMatch);
    }

    #[test]
    fn closeness_adjacent_chars() {
        // "abc" in "abcdef": positions 0, 1, 2. spread = 2.
        // score = 1.0 + 1.0/2 = 1.5
        let rank = get_closeness_ranking("abcdef", "abc");
        assert_eq!(rank, Ranking::Matches(1.5));
    }

    #[test]
    fn closeness_two_char_query() {
        // "ad" in "abcdef": positions 0, 3. spread = 3.
        // score = 1.0 + 1.0/3
        let rank = get_closeness_ranking("abcdef", "ad");
        match rank {
            Ranking::Matches(s) => {
                let expected = 1.0 + 1.0 / 3.0;
                assert!(
                    (s - expected).abs() < f64::EPSILON,
                    "expected {expected}, got {s}"
                );
            }
            other => panic!("expected Matches, got {other:?}"),
        }
    }

    #[test]
    fn closeness_partial_mismatch() {
        // "a" is found but "z" is not.
        assert_eq!(get_closeness_ranking("abcdef", "az"), Ranking::NoMatch);
    }

    #[test]
    fn closeness_query_longer_than_candidate() {
        // More query chars than candidate chars: guaranteed NoMatch.
        assert_eq!(get_closeness_ranking("ab", "abcdef"), Ranking::NoMatch);
    }

    #[test]
    fn closeness_result_always_in_range() {
        // Test several cases to verify the invariant (1.0, 2.0].
        let cases = [
            ("abcdefghijklmnop", "ap"),   // spread = 15
            ("abcdefghijklmnop", "abop"), // spread = 15
            ("abcdef", "af"),             // spread = 5
            ("ab", "ab"),                 // spread = 1
        ];
        for (candidate, query) in cases {
            let rank = get_closeness_ranking(candidate, query);
            match rank {
                Ranking::Matches(s) => {
                    assert!(
                        s > 1.0 && s <= 2.0,
                        "score {s} out of range for ({candidate}, {query})"
                    );
                }
                other => panic!("expected Matches for ({candidate}, {query}), got {other:?}"),
            }
        }
    }

    #[test]
    fn closeness_case_sensitive() {
        // The function does case-sensitive matching; caller is responsible
        // for lowercasing. 'A' != 'a'.
        assert_eq!(get_closeness_ranking("abc", "A"), Ranking::NoMatch);
    }

    #[test]
    fn closeness_empty_query() {
        // Empty query: no characters to find, spread = 0.
        assert_eq!(get_closeness_ranking("anything", ""), Ranking::Matches(2.0));
    }

    #[test]
    fn closeness_unicode_chars() {
        // Unicode characters are matched correctly via .chars().
        // candidate: "a b c" (3 chars), query: "ac" -> positions 0, 2, spread = 2
        let rank = get_closeness_ranking("a\u{00E9}c", "ac");
        assert_eq!(rank, Ranking::Matches(1.5));
    }

    // --- get_match_ranking tests ---

    #[test]
    fn ranking_equal() {
        // Case-insensitive full match: "Green" prepared -> "Green",
        // lowercased -> "green" == "green".
        assert_eq!(get_match_ranking("Green", "green", false), Ranking::Equal);
    }

    #[test]
    fn ranking_case_sensitive_equal() {
        // Exact byte-for-byte match after diacritics preparation.
        assert_eq!(
            get_match_ranking("Green", "Green", false),
            Ranking::CaseSensitiveEqual
        );
    }

    #[test]
    fn ranking_starts_with() {
        // "Greenland" lowercased starts with "green" but is longer.
        assert_eq!(
            get_match_ranking("Greenland", "green", false),
            Ranking::StartsWith
        );
    }

    #[test]
    fn ranking_word_starts_with() {
        // "San Francisco" lowercased contains "fran" at position 4,
        // preceded by a space at byte position 3.
        assert_eq!(
            get_match_ranking("San Francisco", "fran", false),
            Ranking::WordStartsWith
        );
    }

    #[test]
    fn ranking_contains() {
        // "abcdef" contains "cde" starting at position 2, not at a word boundary.
        assert_eq!(get_match_ranking("abcdef", "cde", false), Ranking::Contains);
    }

    #[test]
    fn ranking_acronym() {
        // "North-West Airlines" -> acronym "nwa" (lowercased), which contains "nwa".
        assert_eq!(
            get_match_ranking("North-West Airlines", "nwa", false),
            Ranking::Acronym
        );
    }

    #[test]
    fn ranking_fuzzy_matches() {
        // "playground" vs "plgnd": no substring match, no acronym match,
        // but fuzzy closeness finds all chars in order.
        let rank = get_match_ranking("playground", "plgnd", false);
        match rank {
            Ranking::Matches(s) => {
                assert!(s > 1.0, "sub-score {s} should be > 1.0");
                assert!(s < 2.0, "sub-score {s} should be < 2.0");
            }
            other => panic!("expected Matches, got {other:?}"),
        }
    }

    #[test]
    fn ranking_no_match() {
        // No characters in common.
        assert_eq!(get_match_ranking("abc", "xyz", false), Ranking::NoMatch);
    }

    #[test]
    fn ranking_query_longer_than_candidate() {
        // Step 1: query has more characters than candidate -> NoMatch.
        assert_eq!(get_match_ranking("ab", "abcdef", false), Ranking::NoMatch);
    }

    #[test]
    fn ranking_single_char_not_substring() {
        // Step 9: single character query not found as substring -> NoMatch.
        // "z" is not in "abcdef".
        assert_eq!(get_match_ranking("abcdef", "z", false), Ranking::NoMatch);
    }

    #[test]
    fn ranking_single_char_substring_found() {
        // Single character found as substring should still match (step 4-6).
        // "a" is found at position 0 and candidate is length 6 -> StartsWith.
        assert_eq!(get_match_ranking("abcdef", "a", false), Ranking::StartsWith);
    }

    #[test]
    fn ranking_single_char_equal() {
        // Single character that exactly matches the candidate.
        assert_eq!(
            get_match_ranking("a", "a", false),
            Ranking::CaseSensitiveEqual
        );
    }

    #[test]
    fn ranking_empty_query() {
        // Empty query against any non-empty string: the empty string is a
        // substring at position 0, and len("") != len(candidate) -> StartsWith.
        assert_eq!(
            get_match_ranking("anything", "", false),
            Ranking::StartsWith
        );
    }

    #[test]
    fn ranking_both_empty() {
        // Both empty: after preparation, candidate == query -> CaseSensitiveEqual.
        assert_eq!(
            get_match_ranking("", "", false),
            Ranking::CaseSensitiveEqual
        );
    }

    #[test]
    fn ranking_word_boundary_only_spaces() {
        // Hyphens do NOT act as word boundaries for WordStartsWith (step 7).
        // "North-West" lowercased contains "west" at position 6, preceded
        // by '-' which is NOT a space. So it falls through to Contains.
        assert_eq!(
            get_match_ranking("North-West", "west", false),
            Ranking::Contains
        );
    }

    #[test]
    fn ranking_word_boundary_second_occurrence() {
        // The first substring match is not at a word boundary, but a later
        // one is. "foo foobar" contains "foo" at positions 0 and 4.
        // Position 0 -> StartsWith (steps 5-6 apply before step 7).
        // Use a different example where first match is not at pos 0:
        // "xfoo bar foo" -> "foo" at positions 1, 8. Position 1 preceded by
        // 'x' (not space). Position 8 preceded by ' ' -> WordStartsWith.
        assert_eq!(
            get_match_ranking("xfoo bar foo", "foo", false),
            Ranking::WordStartsWith
        );
    }

    #[test]
    fn ranking_diacritics_stripping() {
        // "caf\u{00E9}" with keep_diacritics=false is prepared to "cafe".
        // Query "cafe" is prepared to "cafe". Exact match -> CaseSensitiveEqual.
        assert_eq!(
            get_match_ranking("caf\u{00E9}", "cafe", false),
            Ranking::CaseSensitiveEqual
        );
    }

    #[test]
    fn ranking_diacritics_kept() {
        // With keep_diacritics=true, "caf\u{00E9}" != "cafe" (different bytes).
        // Lowercased: "caf\u{00E9}" does not contain "cafe" as a substring.
        // Fuzzy match: c(0), a(1), f(2) found, but 'e' in query won't match
        // '\u{00E9}' in candidate -> depends on the actual chars.
        // Actually the lowercased \u{00E9} is still \u{00E9}, and 'e' != '\u{00E9}'.
        // So fuzzy fails too -> NoMatch.
        assert_eq!(
            get_match_ranking("caf\u{00E9}", "cafe", true),
            Ranking::NoMatch
        );
    }

    #[test]
    fn ranking_unicode_char_count_vs_byte_count() {
        // Step 1 compares CHARACTER counts, not byte counts.
        // "\u{00E9}" is 2 bytes but 1 character. So a candidate of 1 char
        // with a query of 2 chars should be NoMatch by character count.
        assert_eq!(get_match_ranking("\u{00E9}", "ab", true), Ranking::NoMatch);
    }

    #[test]
    fn ranking_acronym_not_reached_for_single_char() {
        // Step 9 prevents single-char queries from reaching acronym check.
        // "a b c" has acronym "abc", but query "x" (single char) not found
        // as substring -> NoMatch at step 9, never reaches acronym.
        assert_eq!(get_match_ranking("a b c", "x", false), Ranking::NoMatch);
    }

    #[test]
    fn ranking_acronym_multi_word() {
        // "as soon as possible" -> acronym "asap", query "asap" matches.
        assert_eq!(
            get_match_ranking("as soon as possible", "asap", false),
            Ranking::Acronym
        );
    }

    #[test]
    fn ranking_contains_mid_string() {
        // "hello world" contains "lo w" at position 3. Byte before pos 3
        // is 'l' (not space) -> Contains.
        assert_eq!(
            get_match_ranking("hello world", "lo w", false),
            Ranking::Contains
        );
    }

    #[test]
    fn ranking_query_longer_than_candidate_unicode() {
        // Unicode characters: candidate has 2 chars, query has 3 chars.
        assert_eq!(
            get_match_ranking("\u{4e16}\u{754c}", "abc", false),
            Ranking::NoMatch
        );
    }
}

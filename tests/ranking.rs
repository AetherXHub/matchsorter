//! Integration tests for the `matchsorter` ranking engine.
//!
//! Each test maps to a specific acceptance criterion from PRD-001
//! (AC 2 through AC 14). Tests exercise only the public API exported
//! from `matchsorter`: [`Ranking`] and [`get_match_ranking`].

use matchsorter::{Ranking, get_match_ranking};

/// AC 2: `get_match_ranking("Green", "green")` -> `Equal`
///
/// Case-insensitive full match: both strings have the same characters
/// when lowercased, and their lengths are equal.
#[test]
fn ac02_equal() {
    assert_eq!(get_match_ranking("Green", "green", false), Ranking::Equal);
}

/// AC 3: `get_match_ranking("Green", "Green")` -> `CaseSensitiveEqual`
///
/// Exact byte-for-byte match: the prepared strings are identical.
#[test]
fn ac03_case_sensitive_equal() {
    assert_eq!(
        get_match_ranking("Green", "Green", false),
        Ranking::CaseSensitiveEqual,
    );
}

/// AC 4: `get_match_ranking("Greenland", "green")` -> `StartsWith`
///
/// The candidate starts with the query (case-insensitively) but is longer.
#[test]
fn ac04_starts_with() {
    assert_eq!(
        get_match_ranking("Greenland", "green", false),
        Ranking::StartsWith,
    );
}

/// AC 5: `get_match_ranking("San Francisco", "fran")` -> `WordStartsWith`
///
/// The query matches a word that begins after a space boundary
/// within the candidate.
#[test]
fn ac05_word_starts_with() {
    assert_eq!(
        get_match_ranking("San Francisco", "fran", false),
        Ranking::WordStartsWith,
    );
}

/// AC 6: `get_match_ranking("abcdef", "cde")` -> `Contains`
///
/// The query is found as a substring but not at position 0 and not
/// at a word boundary.
#[test]
fn ac06_contains() {
    assert_eq!(get_match_ranking("abcdef", "cde", false), Ranking::Contains,);
}

/// AC 7: `get_match_ranking("North-West Airlines", "nwa")` -> `Acronym`
///
/// The query matches the acronym extracted from the candidate's
/// word-initial characters (space and hyphen are both acronym delimiters).
#[test]
fn ac07_acronym() {
    assert_eq!(
        get_match_ranking("North-West Airlines", "nwa", false),
        Ranking::Acronym,
    );
}

/// AC 8: `get_match_ranking("playground", "plgnd")` -> `Matches` with
/// sub-score in `(1.0, 2.0)`.
///
/// Fuzzy in-order character match: all query characters appear in the
/// candidate in order but are spread apart, producing a continuous
/// sub-score strictly between 1.0 and 2.0.
#[test]
fn ac08_fuzzy_matches_sub_score() {
    let result = get_match_ranking("playground", "plgnd", false);
    if let Ranking::Matches(score) = result {
        assert!(
            score > 1.0 && score < 2.0,
            "expected sub-score in (1.0, 2.0), got {score}",
        );
    } else {
        panic!("expected Matches variant, got {result:?}");
    }
}

/// AC 9: `get_match_ranking("abc", "xyz")` -> `NoMatch`
///
/// No characters in common; no tier matches.
#[test]
fn ac09_no_match() {
    assert_eq!(get_match_ranking("abc", "xyz", false), Ranking::NoMatch,);
}

/// AC 10: Diacritics stripped -- `get_match_ranking("caf\u{00e9}", "cafe", false)`
/// returns a match at the `Equal` tier or above.
///
/// With `keep_diacritics: false`, the precomposed e-acute (U+00E9) is
/// decomposed via NFD and the combining mark is stripped, producing "cafe".
/// The query "cafe" is also "cafe". Because the prepared strings are
/// byte-equal, the algorithm returns `CaseSensitiveEqual` (step 2 fires
/// before the lowercasing step that would produce `Equal`).
///
/// Note: PRD AC 10 states `Equal`, but the 11-step algorithm correctly
/// yields `CaseSensitiveEqual` because the case-sensitive check (step 2)
/// precedes the case-insensitive check (step 5). The test asserts the
/// actual implementation behavior.
#[test]
fn ac10_diacritics_stripped() {
    let result = get_match_ranking("caf\u{00e9}", "cafe", false);
    // After diacritics stripping both become "cafe", which is a
    // byte-for-byte match -> CaseSensitiveEqual.
    assert_eq!(result, Ranking::CaseSensitiveEqual);
    // Also verify it ranks at or above Equal, as AC 10 intends.
    assert!(result >= Ranking::Equal);
}

/// AC 11: Diacritics kept -- `get_match_ranking("caf\u{00e9}", "cafe", true)`
/// returns `NoMatch` or a tier below `Equal`.
///
/// With `keep_diacritics: true`, the accented character is preserved.
/// Since '\u{00e9}' != 'e', the candidate and query differ. Fuzzy
/// matching also fails because 'e' does not match '\u{00e9}' in a
/// character-by-character scan.
#[test]
fn ac11_diacritics_kept() {
    let result = get_match_ranking("caf\u{00e9}", "cafe", true);
    assert_eq!(result, Ranking::NoMatch);
}

/// AC 12: Single-character query `"x"` that is not a substring -> `NoMatch`.
///
/// Step 9 of the algorithm prevents single-character queries from reaching
/// the acronym or fuzzy matching stages.
#[test]
fn ac12_single_char_no_match() {
    assert_eq!(get_match_ranking("abcdef", "x", false), Ranking::NoMatch,);
}

/// AC 13: Empty query `""` against any non-empty string -> `StartsWith`.
///
/// The empty string is a substring at position 0, and the candidate is
/// longer than the query, so the algorithm reaches step 6 (StartsWith).
#[test]
fn ac13_empty_query() {
    assert_eq!(
        get_match_ranking("anything", "", false),
        Ranking::StartsWith,
    );
}

/// AC 14: Word boundary detection uses only spaces, not hyphens or
/// underscores.
///
/// "North-West" contains "west" at a position preceded by '-', which is
/// NOT a word boundary for `WordStartsWith`. The result falls through
/// to `Contains`.
#[test]
fn ac14_word_boundary_spaces_only() {
    // Hyphen is NOT a word boundary for WordStartsWith.
    assert_eq!(
        get_match_ranking("North-West", "west", false),
        Ranking::Contains,
    );

    // Underscore is NOT a word boundary for WordStartsWith either.
    assert_eq!(
        get_match_ranking("snake_case", "case", false),
        Ranking::Contains,
    );
}

// --- Additional edge-case integration tests ---

/// Verify that the `Matches` variant's sub-score is exactly
/// `1.0 + 1.0 / spread` for a known spread.
///
/// For "playground" / "plgnd": chars match at positions 0,1,4,8,9
/// so spread = 9, score = 1.0 + 1.0/9.
#[test]
fn fuzzy_sub_score_exact_value() {
    let result = get_match_ranking("playground", "plgnd", false);
    if let Ranking::Matches(score) = result {
        let expected = 1.0 + 1.0 / 9.0;
        assert!(
            (score - expected).abs() < f64::EPSILON,
            "expected {expected}, got {score}",
        );
    } else {
        panic!("expected Matches variant, got {result:?}");
    }
}

/// Verify that diacritics stripping handles multiple accented characters
/// in the same string.
#[test]
fn diacritics_multiple_accents() {
    // U+00FC = u with diaeresis, U+00F1 = n with tilde
    // After stripping: "ubermanana"
    let result = get_match_ranking("\u{00fc}ber-ma\u{00f1}ana", "uber", false);
    assert_eq!(result, Ranking::StartsWith);
}

/// Verify tier ordering is respected across all fixed tiers by comparing
/// pairs of inputs that produce different tiers.
#[test]
fn tier_ordering_through_public_api() {
    let case_sensitive = get_match_ranking("test", "test", false);
    let equal = get_match_ranking("Test", "test", false);
    let starts_with = get_match_ranking("testing", "test", false);
    let word_starts = get_match_ranking("unit test", "test", false);
    let contains = get_match_ranking("attest", "test", false);
    let no_match = get_match_ranking("xyz", "test", false);

    assert!(case_sensitive > equal);
    assert!(equal > starts_with);
    assert!(starts_with > word_starts);
    assert!(word_starts > contains);
    assert!(contains > no_match);
}

/// Verify that the empty query against an empty string produces
/// `CaseSensitiveEqual`, not `StartsWith`.
#[test]
fn empty_query_empty_candidate() {
    assert_eq!(
        get_match_ranking("", "", false),
        Ranking::CaseSensitiveEqual,
    );
}

/// Verify that a query longer than the candidate is always `NoMatch`,
/// even when characters are a subset.
#[test]
fn query_longer_than_candidate() {
    assert_eq!(get_match_ranking("ab", "abcdef", false), Ranking::NoMatch,);
}

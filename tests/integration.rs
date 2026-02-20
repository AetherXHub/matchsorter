//! Integration tests for the `match_sorter` public API.
//!
//! These tests exercise the top-level [`match_sorter`] function end-to-end,
//! covering all 14 scenario categories from PRD-003 Section 12. Each test
//! uses only the public API re-exported from the `matchsorter` crate root.

use matchsorter::{AsMatchStr, Key, MatchSorterOptions, RankedItem, Ranking, match_sorter};

// ---------------------------------------------------------------------------
// Shared test types
// ---------------------------------------------------------------------------

/// A simple struct for testing key-based matching.
#[derive(Debug, PartialEq)]
struct Item {
    name: String,
}

impl Item {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
        }
    }
}

// `AsMatchStr` is required by the `T: AsMatchStr` bound on `match_sorter`,
// even when keys are provided. For struct items we delegate to the name field.
impl AsMatchStr for Item {
    fn as_match_str(&self) -> &str {
        &self.name
    }
}

/// A struct with multiple matchable fields, including a multi-value tags field.
#[derive(Debug, PartialEq)]
struct TaggedItem {
    name: String,
    tags: Vec<String>,
}

impl TaggedItem {
    fn new(name: &str, tags: &[&str]) -> Self {
        Self {
            name: name.to_owned(),
            tags: tags.iter().map(|s| (*s).to_owned()).collect(),
        }
    }
}

impl AsMatchStr for TaggedItem {
    fn as_match_str(&self) -> &str {
        &self.name
    }
}

// ---------------------------------------------------------------------------
// 1. Basic string array
// ---------------------------------------------------------------------------

/// Basic string array matching: "ap" against ["apple", "banana", "grape"].
/// "apple" should appear first (StartsWith) and "grape" should also match
/// (Contains: "ap" is a substring at position 2).
#[test]
fn basic_string_array_apple_first() {
    let items = ["apple", "banana", "grape"];
    let results = match_sorter(&items, "ap", MatchSorterOptions::default());
    assert!(!results.is_empty(), "should have at least one match");
    assert_eq!(results[0], &"apple", "apple should be first (StartsWith)");
    // "grape" contains "ap" -> should be included
    assert!(
        results.contains(&&"grape"),
        "grape should match via Contains"
    );
}

/// Verify that exact matches sort above prefix matches, and prefix matches
/// sort above substring matches.
#[test]
fn basic_string_array_rank_ordering() {
    // "apple" -> CaseSensitiveEqual for "apple"
    // "applesauce" -> StartsWith for "apple"
    // "pineapple" -> Contains for "apple" (substring at position 4)
    let items = ["pineapple", "apple", "applesauce"];
    let results = match_sorter(&items, "apple", MatchSorterOptions::default());
    assert_eq!(results[0], &"apple", "exact match first");
    assert_eq!(results[1], &"applesauce", "StartsWith second");
    assert_eq!(results[2], &"pineapple", "Contains third");
}

// ---------------------------------------------------------------------------
// 2. Case sensitivity
// ---------------------------------------------------------------------------

/// Case-insensitive matching: "green" matches "Green" at the Equal tier.
/// The match_sorter function should include it in results.
#[test]
fn case_insensitive_matching() {
    let items = ["Green", "Red", "Blue"];
    let results = match_sorter(&items, "green", MatchSorterOptions::default());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], &"Green");
}

/// Case-sensitive exact match ranks higher than case-insensitive.
#[test]
fn case_sensitive_beats_insensitive() {
    let items = ["green", "Green"];
    let results = match_sorter(&items, "green", MatchSorterOptions::default());
    // "green" -> CaseSensitiveEqual, "Green" -> Equal
    assert_eq!(results[0], &"green", "exact case match should be first");
    assert_eq!(results[1], &"Green", "case-insensitive match second");
}

// ---------------------------------------------------------------------------
// 3. Diacritics
// ---------------------------------------------------------------------------

/// Diacritics stripping: "cafe" matches "caf\u{00e9}" when diacritics are
/// stripped (the default). Both "cafe" and "caf\u{00e9}" should appear.
#[test]
fn diacritics_cafe_matches_accented() {
    let items = ["cafe", "caf\u{00e9}", "restaurant"];
    let results = match_sorter(&items, "cafe", MatchSorterOptions::default());
    assert_eq!(results.len(), 2, "both cafe and cafe should match");
    assert!(results.contains(&&"cafe"));
    assert!(results.contains(&&"caf\u{00e9}"));
}

/// When `keep_diacritics` is true, "cafe" does NOT match "caf\u{00e9}".
#[test]
fn diacritics_kept_no_cross_match() {
    let items = ["cafe", "caf\u{00e9}"];
    let opts = MatchSorterOptions {
        keep_diacritics: true,
        ..Default::default()
    };
    let results = match_sorter(&items, "cafe", opts);
    assert_eq!(results, vec![&"cafe"], "only exact cafe matches");
}

// ---------------------------------------------------------------------------
// 4. Threshold filtering
// ---------------------------------------------------------------------------

/// Setting threshold to Contains excludes fuzzy-only matches.
/// "banana" has no substring "ap" and can only match via fuzzy (if at all).
#[test]
fn threshold_contains_excludes_fuzzy() {
    let items = ["apple", "banana", "grape"];
    let opts = MatchSorterOptions {
        threshold: Ranking::Contains,
        ..Default::default()
    };
    let results = match_sorter(&items, "ap", opts);
    // "apple" -> StartsWith (>= Contains), "grape" -> Contains (>= Contains)
    // "banana" -> NoMatch for "ap" substring, possibly fuzzy only -> excluded
    assert_eq!(results, vec![&"apple", &"grape"]);
}

/// Threshold set to CaseSensitiveEqual only includes exact matches.
#[test]
fn threshold_case_sensitive_equal_strict() {
    let items = ["apple", "Apple", "APPLE"];
    let opts = MatchSorterOptions {
        threshold: Ranking::CaseSensitiveEqual,
        ..Default::default()
    };
    let results = match_sorter(&items, "apple", opts);
    assert_eq!(results, vec![&"apple"]);
}

// ---------------------------------------------------------------------------
// 5. Key-based matching with struct
// ---------------------------------------------------------------------------

/// Key-based matching: extract the `name` field from a struct and match
/// against it.
#[test]
fn key_based_struct_matching() {
    let items = vec![Item::new("Alice"), Item::new("Bob"), Item::new("Charlie")];
    let opts = MatchSorterOptions {
        keys: vec![Key::new(|i: &Item| vec![i.name.clone()])],
        ..Default::default()
    };
    let results = match_sorter(&items, "ali", opts);
    assert!(!results.is_empty());
    assert_eq!(results[0].name, "Alice", "Alice matches via StartsWith");
}

/// Key-based matching with from_fn convenience constructor.
#[test]
fn key_based_from_fn() {
    let items = vec![Item::new("Delta"), Item::new("Echo"), Item::new("Foxtrot")];
    let opts = MatchSorterOptions {
        keys: vec![Key::<Item>::from_fn(|i| i.name.as_str())],
        ..Default::default()
    };
    let results = match_sorter(&items, "echo", opts);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Echo");
}

// ---------------------------------------------------------------------------
// 6. Multi-value key
// ---------------------------------------------------------------------------

/// Multi-value key: a struct has tags, and the best matching tag determines
/// the ranking. "admin" should match the item with the "admin" tag.
#[test]
fn multi_value_key_best_tag_wins() {
    let items = vec![
        TaggedItem::new("Alice", &["admin", "staff"]),
        TaggedItem::new("Bob", &["user"]),
        TaggedItem::new("Charlie", &["moderator", "admin"]),
    ];
    let opts = MatchSorterOptions {
        keys: vec![Key::new(|i: &TaggedItem| i.tags.clone())],
        ..Default::default()
    };
    let results = match_sorter(&items, "admin", opts);
    // Both Alice and Charlie have "admin" tag -> CaseSensitiveEqual
    assert_eq!(results.len(), 2);
    let names: Vec<&str> = results.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"Alice"));
    assert!(names.contains(&"Charlie"));
}

/// Multi-value key with from_fn_multi: extract tags as borrowed slices.
#[test]
fn multi_value_key_from_fn_multi() {
    let items = vec![
        TaggedItem::new("Server", &["production", "linux"]),
        TaggedItem::new("Laptop", &["development", "macos"]),
    ];
    let opts = MatchSorterOptions {
        keys: vec![Key::<TaggedItem>::from_fn_multi(|i| {
            i.tags.iter().map(|t| t.as_str()).collect()
        })],
        ..Default::default()
    };
    let results = match_sorter(&items, "linux", opts);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Server");
}

// ---------------------------------------------------------------------------
// 7. Per-key min/max ranking clamping
// ---------------------------------------------------------------------------

/// max_ranking clamps a high ranking down. An exact match on the name key
/// is clamped to at most Contains.
#[test]
fn per_key_max_ranking_clamps_down() {
    let items = vec![Item::new("Alice"), Item::new("Bob")];
    let opts = MatchSorterOptions {
        keys: vec![Key::new(|i: &Item| vec![i.name.clone()]).max_ranking(Ranking::Contains)],
        ..Default::default()
    };
    // "Alice" queried with "Alice" would normally be CaseSensitiveEqual,
    // but clamped to Contains. Both items need to pass the default threshold
    // (Matches). Only Alice matches.
    let results = match_sorter(&items, "Alice", opts);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Alice");
}

/// min_ranking promotes a fuzzy match up to a higher tier.
#[test]
fn per_key_min_ranking_promotes() {
    let items = vec![Item::new("playground"), Item::new("apple")];
    let opts = MatchSorterOptions {
        keys: vec![Key::new(|i: &Item| vec![i.name.clone()]).min_ranking(Ranking::Contains)],
        ..Default::default()
    };
    // "playground" queried with "plgnd" -> fuzzy Matches, promoted to Contains.
    // "apple" queried with "plgnd" -> NoMatch (not promoted).
    let results = match_sorter(&items, "plgnd", opts);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "playground");
}

/// min_ranking does NOT promote NoMatch.
#[test]
fn per_key_min_ranking_does_not_promote_no_match() {
    let items = vec![Item::new("abc")];
    let opts = MatchSorterOptions {
        keys: vec![Key::new(|i: &Item| vec![i.name.clone()]).min_ranking(Ranking::Contains)],
        ..Default::default()
    };
    // "abc" queried with "xyz" -> NoMatch, not promoted.
    let results = match_sorter(&items, "xyz", opts);
    assert!(results.is_empty());
}

// ---------------------------------------------------------------------------
// 8. Custom base_sort: preserve original order
// ---------------------------------------------------------------------------

/// Custom base_sort that preserves original input order (sort by index)
/// instead of alphabetical tiebreaker.
#[test]
fn custom_base_sort_preserve_original_order() {
    let items = ["cherry", "banana", "apple"];
    let opts = MatchSorterOptions {
        base_sort: Some(Box::new(|a: &RankedItem<&str>, b: &RankedItem<&str>| {
            a.index.cmp(&b.index)
        })),
        ..Default::default()
    };
    // Empty query: all items match with the same rank (StartsWith) and
    // same key_index (0). Tiebreaker is base_sort, which preserves
    // original order instead of alphabetical.
    let results = match_sorter(&items, "", opts);
    assert_eq!(results, vec![&"cherry", &"banana", &"apple"]);
}

/// Verify the default base_sort produces alphabetical order for ties.
#[test]
fn default_base_sort_alphabetical() {
    let items = ["cherry", "banana", "apple"];
    let results = match_sorter(&items, "", MatchSorterOptions::default());
    // All same rank, tiebreak alphabetically: apple, banana, cherry
    assert_eq!(results, vec![&"apple", &"banana", &"cherry"]);
}

// ---------------------------------------------------------------------------
// 9. Sorter override
// ---------------------------------------------------------------------------

/// Custom sorter that reverses the default order.
#[test]
fn sorter_override_reverse() {
    let items = ["apple", "banana", "grape"];
    let default_results = match_sorter(&items, "a", MatchSorterOptions::default());

    let opts = MatchSorterOptions {
        sorter: Some(Box::new(|mut items: Vec<RankedItem<&str>>| {
            items.reverse();
            items
        })),
        ..Default::default()
    };
    let reversed_results = match_sorter(&items, "a", opts);

    // Reversed results should be the opposite order of the default.
    assert_eq!(reversed_results.len(), default_results.len());
    let mut reversed_default = default_results.clone();
    reversed_default.reverse();
    assert_eq!(reversed_results, reversed_default);
}

/// Custom sorter that sorts only by original index (input order).
#[test]
fn sorter_override_preserve_input_order() {
    let items = ["grape", "apple", "banana"];
    let opts = MatchSorterOptions {
        sorter: Some(Box::new(|mut items: Vec<RankedItem<&str>>| {
            items.sort_by_key(|ri| ri.index);
            items
        })),
        ..Default::default()
    };
    let results = match_sorter(&items, "", opts);
    // Sorter preserves input order regardless of rank.
    assert_eq!(results, vec![&"grape", &"apple", &"banana"]);
}

// ---------------------------------------------------------------------------
// 10. Empty query
// ---------------------------------------------------------------------------

/// Empty query returns all items, sorted alphabetically by default base_sort.
#[test]
fn empty_query_returns_all_sorted() {
    let items = ["banana", "apple", "cherry"];
    let results = match_sorter(&items, "", MatchSorterOptions::default());
    assert_eq!(results.len(), 3, "all items should be returned");
    assert_eq!(results[0], &"apple");
    assert_eq!(results[1], &"banana");
    assert_eq!(results[2], &"cherry");
}

/// Empty query with String items.
#[test]
fn empty_query_string_items() {
    let items = vec!["zebra".to_owned(), "mango".to_owned()];
    let results = match_sorter(&items, "", MatchSorterOptions::default());
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].as_str(), "mango");
    assert_eq!(results[1].as_str(), "zebra");
}

// ---------------------------------------------------------------------------
// 11. Single-char query
// ---------------------------------------------------------------------------

/// Single character "a" matches items containing "a" as a substring.
/// Items without "a" are excluded.
#[test]
fn single_char_query_matches_substring() {
    let items = ["apple", "banana", "plum", "grape"];
    let results = match_sorter(&items, "a", MatchSorterOptions::default());
    // "apple" -> StartsWith, "banana" -> Contains (a at pos 1),
    // "grape" -> Contains (a at pos 2), "plum" -> NoMatch
    assert!(!results.contains(&&"plum"), "plum has no 'a'");
    assert_eq!(results[0], &"apple", "apple starts with 'a'");
    assert!(results.contains(&&"banana"));
    assert!(results.contains(&&"grape"));
}

/// Single character that does not exist in any item -> empty results.
#[test]
fn single_char_query_no_match() {
    let items = ["hello", "world"];
    let results = match_sorter(&items, "z", MatchSorterOptions::default());
    assert!(results.is_empty());
}

// ---------------------------------------------------------------------------
// 12. Acronym matching
// ---------------------------------------------------------------------------

/// "nwa" matches "North-West Airlines" via the Acronym tier.
#[test]
fn acronym_matching_nwa() {
    let items = [
        "North-West Airlines",
        "National Weather Association",
        "Something Else",
    ];
    let results = match_sorter(&items, "nwa", MatchSorterOptions::default());
    assert!(
        results.contains(&&"North-West Airlines"),
        "North-West Airlines should match via Acronym"
    );
    assert!(
        results.contains(&&"National Weather Association"),
        "National Weather Association also has acronym nwa"
    );
    assert!(
        !results.contains(&&"Something Else"),
        "Something Else should not match"
    );
}

/// "asap" matches "as soon as possible" via Acronym.
#[test]
fn acronym_matching_asap() {
    let items = ["as soon as possible", "something random"];
    let results = match_sorter(&items, "asap", MatchSorterOptions::default());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], &"as soon as possible");
}

// ---------------------------------------------------------------------------
// 13. Word boundary detection
// ---------------------------------------------------------------------------

/// "fran" matches "San Francisco" via WordStartsWith because "Francisco"
/// starts at a word boundary (preceded by a space).
#[test]
fn word_boundary_fran_matches_san_francisco() {
    let items = ["San Francisco", "New York", "Frankfurt"];
    let results = match_sorter(&items, "fran", MatchSorterOptions::default());
    // "San Francisco" -> WordStartsWith, "Frankfurt" -> StartsWith
    assert!(results.contains(&&"San Francisco"));
    assert!(results.contains(&&"Frankfurt"));
    // Frankfurt (StartsWith) should rank higher than San Francisco (WordStartsWith)
    assert_eq!(results[0], &"Frankfurt");
    assert_eq!(results[1], &"San Francisco");
}

/// Hyphens are NOT word boundaries for WordStartsWith. "west" in
/// "North-West" is Contains, not WordStartsWith.
#[test]
fn word_boundary_hyphen_not_boundary() {
    let items = ["North-West", "South West"];
    let results = match_sorter(&items, "west", MatchSorterOptions::default());
    // "North-West" -> Contains (hyphen not a word boundary)
    // "South West" -> WordStartsWith (space is a word boundary)
    assert_eq!(
        results[0], &"South West",
        "South West should rank higher (WordStartsWith)"
    );
    assert_eq!(
        results[1], &"North-West",
        "North-West should rank lower (Contains)"
    );
}

// ---------------------------------------------------------------------------
// 14. Edge cases
// ---------------------------------------------------------------------------

/// Empty items slice returns an empty result.
#[test]
fn edge_empty_items() {
    let items: [&str; 0] = [];
    let results = match_sorter(&items, "test", MatchSorterOptions::default());
    assert!(results.is_empty());
}

/// Very long strings are handled without panic.
#[test]
fn edge_very_long_strings() {
    let long_string = "a".repeat(10_000);
    let items = [long_string.as_str()];
    let results = match_sorter(&items, "a", MatchSorterOptions::default());
    assert_eq!(results.len(), 1);
}

/// Very long query against short items produces empty results.
#[test]
fn edge_long_query_short_items() {
    let items = ["hi", "ok"];
    let long_query = "a".repeat(1_000);
    let results = match_sorter(&items, &long_query, MatchSorterOptions::default());
    assert!(results.is_empty());
}

/// Items with empty strings: empty string matches empty query exactly.
#[test]
fn edge_empty_string_item() {
    let items = ["", "nonempty"];
    let results = match_sorter(&items, "", MatchSorterOptions::default());
    // Both match: "" -> CaseSensitiveEqual, "nonempty" -> StartsWith
    assert_eq!(results.len(), 2);
    assert_eq!(results[0], &"", "empty string is CaseSensitiveEqual");
    assert_eq!(results[1], &"nonempty", "nonempty is StartsWith");
}

/// Unicode items and queries work correctly.
#[test]
fn edge_unicode_items() {
    let items = ["\u{4e16}\u{754c}", "hello"];
    let results = match_sorter(&items, "\u{4e16}", MatchSorterOptions::default());
    // \u{4e16} is a single character, found at position 0 of \u{4e16}\u{754c}
    // -> StartsWith
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], &"\u{4e16}\u{754c}");
}

// ---------------------------------------------------------------------------
// Additional scenario: per-key threshold override in keys mode
// ---------------------------------------------------------------------------

/// Per-key threshold overrides the global threshold. A key with
/// threshold=CaseSensitiveEqual only lets exact matches through, even
/// when the global threshold is permissive.
#[test]
fn per_key_threshold_override() {
    let items = vec![Item::new("apple"), Item::new("apricot")];
    let opts = MatchSorterOptions {
        keys: vec![
            Key::new(|i: &Item| vec![i.name.clone()]).threshold(Ranking::CaseSensitiveEqual),
        ],
        threshold: Ranking::Matches(1.0), // global is permissive
        ..Default::default()
    };
    let results = match_sorter(&items, "apple", opts);
    // Per-key threshold is CaseSensitiveEqual, so only exact match passes.
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "apple");
}

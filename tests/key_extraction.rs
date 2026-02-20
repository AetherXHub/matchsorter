//! Integration tests for PRD-002: Key Extraction and Value Resolution.
//!
//! Each test maps to a specific acceptance criterion from PRD-002.
//! Tests use a realistic `User` struct with `name`, `email`, and `tags`
//! fields, exercising the public API exported from `matchsorter`.

use matchsorter::{
    Key, MatchSorterOptions, Ranking, get_highest_ranking, get_item_values, rank_item,
};

// ---------------------------------------------------------------------------
// Shared test fixtures
// ---------------------------------------------------------------------------

/// Realistic user type with name, email, and tags fields.
struct User {
    name: String,
    email: String,
    tags: Vec<String>,
}

/// Returns a sample user for tests that do not need specific field values.
fn sample_user() -> User {
    User {
        name: "Alice".to_owned(),
        email: "alice@example.com".to_owned(),
        tags: vec!["admin".to_owned(), "staff".to_owned()],
    }
}

/// Shorthand for default options (strip diacritics).
fn default_opts() -> MatchSorterOptions<User> {
    MatchSorterOptions::default()
}

// ---------------------------------------------------------------------------
// AC 1: Key::new accepts closures that return Vec<String>
// ---------------------------------------------------------------------------

/// AC 1: `Key::new` accepts a closure returning `Vec<String>`, and extracting
/// values from a realistic `User` struct works correctly.
#[test]
fn ac01_key_new_accepts_closure_returning_vec_string() {
    let user = sample_user();
    let key = Key::new(|u: &User| vec![u.name.clone()]);
    let values = get_item_values(&user, &key);
    assert_eq!(values, vec!["Alice"]);
}

// ---------------------------------------------------------------------------
// AC 2: Builder methods .threshold(), .max_ranking(), .min_ranking() work
// ---------------------------------------------------------------------------

/// AC 2: Builder methods set fields correctly and affect `get_highest_ranking`
/// behavior. A key with `threshold`, `max_ranking`, and `min_ranking` all set
/// should reflect each attribute in the ranking result.
#[test]
fn ac02_builder_methods_set_fields_correctly() {
    let user = sample_user();

    // Build a key with all three builder methods.
    // max_ranking = Contains clamps "Alice"/"Alice" (CaseSensitiveEqual) down
    // to Contains. threshold is recorded in the RankingInfo. min_ranking is
    // below Contains so it has no effect here.
    let keys = vec![
        Key::new(|u: &User| vec![u.name.clone()])
            .threshold(Ranking::Acronym)
            .max_ranking(Ranking::Contains)
            .min_ranking(Ranking::Acronym),
    ];

    let info = get_highest_ranking(&user, &keys, "Alice", &default_opts());

    // max_ranking clamped CaseSensitiveEqual down to Contains.
    assert_eq!(info.rank, Ranking::Contains);
    // threshold is reflected in key_threshold.
    assert_eq!(info.key_threshold, Some(Ranking::Acronym));
}

// ---------------------------------------------------------------------------
// AC 3: max_ranking clamps rankings down
// ---------------------------------------------------------------------------

/// AC 3: A key with `max_ranking = Contains` clamps a match that would
/// naturally be `StartsWith` down to `Contains`.
#[test]
fn ac03_max_ranking_clamps_starts_with_to_contains() {
    let user = sample_user();

    // "Alice" queried with "ali" normally produces StartsWith.
    // max_ranking = Contains clamps it down.
    let keys = vec![Key::new(|u: &User| vec![u.name.clone()]).max_ranking(Ranking::Contains)];

    let info = get_highest_ranking(&user, &keys, "ali", &default_opts());
    assert_eq!(info.rank, Ranking::Contains);
}

// ---------------------------------------------------------------------------
// AC 4: min_ranking promotes non-NoMatch rankings
// ---------------------------------------------------------------------------

/// AC 4: A key with `min_ranking = Contains` promotes a fuzzy `Matches`
/// result up to `Contains`.
#[test]
fn ac04_min_ranking_promotes_fuzzy_match_to_contains() {
    // "playground" queried with "plgnd" naturally produces Matches(~1.11).
    // min_ranking = Contains promotes it up.
    let user = User {
        name: "playground".to_owned(),
        email: String::new(),
        tags: vec![],
    };

    let keys = vec![Key::new(|u: &User| vec![u.name.clone()]).min_ranking(Ranking::Contains)];

    let info = get_highest_ranking(&user, &keys, "plgnd", &default_opts());
    assert_eq!(info.rank, Ranking::Contains);
}

// ---------------------------------------------------------------------------
// AC 5: min_ranking does NOT promote NoMatch
// ---------------------------------------------------------------------------

/// AC 5: A key with `min_ranking = Contains` does NOT promote a `NoMatch`
/// result. An item that does not match at all stays `NoMatch`.
#[test]
fn ac05_min_ranking_does_not_promote_no_match() {
    let user = User {
        name: "abc".to_owned(),
        email: String::new(),
        tags: vec![],
    };

    let keys = vec![Key::new(|u: &User| vec![u.name.clone()]).min_ranking(Ranking::Contains)];

    let info = get_highest_ranking(&user, &keys, "xyz", &default_opts());
    assert_eq!(info.rank, Ranking::NoMatch);
}

// ---------------------------------------------------------------------------
// AC 6: Multiple keys evaluated; best ranking wins
// ---------------------------------------------------------------------------

/// AC 6: With two keys, the second key matches better (CaseSensitiveEqual
/// vs Contains), so the second key's rank is used.
#[test]
fn ac06_multiple_keys_best_ranking_wins() {
    let user = sample_user();

    // Key 0: email "alice@example.com" queried with "Alice" -> Contains
    //         (the substring "alice" is found case-insensitively, not at
    //         position 0 of "alice@example.com" -- actually it IS at pos 0,
    //         so this would be StartsWith. Let's use a query that only the
    //         name matches well.)
    // Key 0: email "alice@example.com" queried with "Alice" -> StartsWith
    // Key 1: name "Alice" queried with "Alice" -> CaseSensitiveEqual
    let keys: Vec<Key<User>> = vec![
        Key::new(|u: &User| vec![u.email.clone()]),
        Key::new(|u: &User| vec![u.name.clone()]),
    ];

    let info = get_highest_ranking(&user, &keys, "Alice", &default_opts());
    assert_eq!(info.rank, Ranking::CaseSensitiveEqual);
    assert_eq!(info.ranked_value, "Alice");
    // Key 0 produces 1 value (index 0), key 1 produces 1 value (index 1).
    assert_eq!(info.key_index, 1);
}

// ---------------------------------------------------------------------------
// AC 7: Equal rank tiebreak -- earlier key (lower key_index) wins
// ---------------------------------------------------------------------------

/// AC 7: When two keys produce the same rank (`CaseSensitiveEqual`), the
/// first key (lower `key_index`) wins.
#[test]
fn ac07_equal_rank_tiebreak_first_key_wins() {
    let user = sample_user();

    // Both keys extract the name "Alice". Both produce CaseSensitiveEqual
    // for query "Alice". The first key's value (key_index 0) should win
    // because the algorithm only replaces on strictly-greater rank.
    let keys: Vec<Key<User>> = vec![
        Key::new(|u: &User| vec![u.name.clone()]),
        Key::new(|u: &User| vec![u.name.clone()]),
    ];

    let info = get_highest_ranking(&user, &keys, "Alice", &default_opts());
    assert_eq!(info.rank, Ranking::CaseSensitiveEqual);
    assert_eq!(info.key_index, 0);
}

// ---------------------------------------------------------------------------
// AC 8: Multi-value keys rank each value independently; best wins
// ---------------------------------------------------------------------------

/// AC 8: A multi-value key extracting tags ranks each tag independently.
/// The matching tag's rank is used.
#[test]
fn ac08_multi_value_key_best_tag_wins() {
    let user = sample_user();

    // Tags are ["admin", "staff"]. Query "admin" matches "admin" as
    // CaseSensitiveEqual, and "staff" as NoMatch.
    let keys = vec![Key::new(|u: &User| u.tags.clone())];

    let info = get_highest_ranking(&user, &keys, "admin", &default_opts());
    assert_eq!(info.rank, Ranking::CaseSensitiveEqual);
    assert_eq!(info.ranked_value, "admin");
}

// ---------------------------------------------------------------------------
// AC 9: No-keys mode with Vec<String> and Vec<&str>
// ---------------------------------------------------------------------------

/// AC 9a: No-keys mode works with `String` items via `rank_item`.
#[test]
fn ac09a_no_keys_mode_vec_string() {
    let items: Vec<String> = vec!["Green".to_owned(), "Greenland".to_owned(), "abc".to_owned()];

    // Rank each item against "Green" using no-keys mode.
    let rankings: Vec<Ranking> = items
        .iter()
        .map(|item| rank_item(item, "Green", false))
        .collect();

    assert_eq!(rankings[0], Ranking::CaseSensitiveEqual);
    assert_eq!(rankings[1], Ranking::StartsWith);
    assert_eq!(rankings[2], Ranking::NoMatch);
}

/// AC 9b: No-keys mode works with `&str` items via `rank_item`.
#[test]
fn ac09b_no_keys_mode_vec_str() {
    let items: Vec<&str> = vec!["Green", "Greenland", "abc"];

    let rankings: Vec<Ranking> = items
        .iter()
        .map(|item| rank_item(item, "Green", false))
        .collect();

    assert_eq!(rankings[0], Ranking::CaseSensitiveEqual);
    assert_eq!(rankings[1], Ranking::StartsWith);
    assert_eq!(rankings[2], Ranking::NoMatch);
}

// ---------------------------------------------------------------------------
// AC 10: Zero unsafe blocks in the codebase
// ---------------------------------------------------------------------------

/// AC 10: Verify that the `src/` directory contains zero `unsafe` blocks.
///
/// This is a compile-time safety property. We verify it by reading all `.rs`
/// source files and asserting none contain the `unsafe` keyword in a
/// non-comment context.
#[test]
fn ac10_zero_unsafe_blocks() {
    let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    // Walk the source directory and collect all .rs files.
    let rs_files = collect_rs_files(&src_dir);
    assert!(!rs_files.is_empty(), "should find at least one .rs file");

    for path in &rs_files {
        let contents = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

        // Check each line for `unsafe` outside of comments.
        for (line_num, line) in contents.lines().enumerate() {
            let trimmed = line.trim();

            // Skip full-line comments.
            if trimmed.starts_with("//") {
                continue;
            }

            // Strip inline comments: take the portion before any `//`.
            let code_part = match trimmed.find("//") {
                Some(pos) => &trimmed[..pos],
                None => trimmed,
            };

            assert!(
                !code_part.contains("unsafe"),
                "found `unsafe` in {}:{}: {}",
                path.display(),
                line_num + 1,
                line,
            );
        }
    }
}

/// Recursively collect all `.rs` files under a directory.
fn collect_rs_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("failed to read dir {}: {e}", dir.display()))
        {
            let entry =
                entry.unwrap_or_else(|e| panic!("failed to read entry in {}: {e}", dir.display()));
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_rs_files(&path));
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    files
}

// ---------------------------------------------------------------------------
// AC 11 & AC 12: cargo test, clippy, and fmt clean
// ---------------------------------------------------------------------------
// These are verified by running the quality gate commands after writing the
// tests. They are not programmatic tests -- they are meta-checks on the
// build pipeline. See the completion report for results.

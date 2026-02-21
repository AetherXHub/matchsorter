use std::borrow::Cow;

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use matchsorter::{
    MatchSorterOptions, RankedItem, Ranking, default_base_sort, get_match_ranking, match_sorter,
    sort_ranked_values,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate a dataset of `n` simple string items: "item_0", "item_1", ...
fn generate_items(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("item_{i}")).collect()
}

/// Generate a dataset of `n` items that contain diacritics on every other entry.
fn generate_diacritics_items(n: usize) -> Vec<String> {
    (0..n)
        .map(|i| {
            if i % 2 == 0 {
                format!("caf\u{00e9}_{i}")
            } else {
                format!("cafe_{i}")
            }
        })
        .collect()
}

/// Build a `Vec<RankedItem>` suitable for benchmarking the sort step in
/// isolation. Items are assigned ranks in a round-robin pattern across
/// several tiers to exercise the three-level comparator.
fn generate_ranked_items(items: &[String]) -> Vec<RankedItem<'_, String>> {
    let tiers = [
        Ranking::CaseSensitiveEqual,
        Ranking::Equal,
        Ranking::StartsWith,
        Ranking::WordStartsWith,
        Ranking::Contains,
        Ranking::Acronym,
        Ranking::Matches(1.5),
    ];
    items
        .iter()
        .enumerate()
        .map(|(i, item)| RankedItem {
            item,
            index: i,
            rank: tiers[i % tiers.len()],
            ranked_value: Cow::Owned(item.clone()),
            key_index: i % 3,
            key_threshold: None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// 1. get_match_ranking micro-benchmark
// ---------------------------------------------------------------------------

fn bench_get_match_ranking(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_match_ranking");

    // Exact match (CaseSensitiveEqual path)
    group.bench_function("exact", |b| {
        b.iter(|| get_match_ranking(black_box("item_500"), black_box("item_500"), false));
    });

    // Prefix match (StartsWith path)
    group.bench_function("prefix", |b| {
        b.iter(|| get_match_ranking(black_box("item_500"), black_box("item"), false));
    });

    // Fuzzy match (Matches path -- characters scattered)
    group.bench_function("fuzzy", |b| {
        b.iter(|| get_match_ranking(black_box("playground"), black_box("plgnd"), false));
    });

    // No match (worst case -- falls through all tiers)
    group.bench_function("no_match", |b| {
        b.iter(|| get_match_ranking(black_box("abcdefghij"), black_box("zzz"), false));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// 2. Throughput at dataset sizes (100, 10_000, 100_000)
// ---------------------------------------------------------------------------

fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");

    for size in [100, 10_000, 100_000] {
        let items = generate_items(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &items, |b, items| {
            b.iter(|| {
                match_sorter(
                    black_box(items),
                    black_box("item_5"),
                    MatchSorterOptions::default(),
                )
            });
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// 3. Query type comparison (on 10k items)
// ---------------------------------------------------------------------------

fn bench_query_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_types");
    let items = generate_items(10_000);

    // Exact match -- matches "item_500" exactly
    group.bench_function("exact", |b| {
        b.iter(|| {
            match_sorter(
                black_box(&items),
                black_box("item_500"),
                MatchSorterOptions::default(),
            )
        });
    });

    // Prefix match -- all items start with "item_"
    group.bench_function("prefix", |b| {
        b.iter(|| {
            match_sorter(
                black_box(&items),
                black_box("item_"),
                MatchSorterOptions::default(),
            )
        });
    });

    // Substring match -- "_50" appears inside many items
    group.bench_function("substring", |b| {
        b.iter(|| {
            match_sorter(
                black_box(&items),
                black_box("_50"),
                MatchSorterOptions::default(),
            )
        });
    });

    // Fuzzy match -- characters spread across the candidate
    group.bench_function("fuzzy", |b| {
        b.iter(|| {
            match_sorter(
                black_box(&items),
                black_box("im5"),
                MatchSorterOptions::default(),
            )
        });
    });

    // No match -- worst case, must check all tiers for every item
    group.bench_function("no_match", |b| {
        b.iter(|| {
            match_sorter(
                black_box(&items),
                black_box("zzzzz"),
                MatchSorterOptions::default(),
            )
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// 4. Diacritics overhead
// ---------------------------------------------------------------------------

fn bench_diacritics(c: &mut Criterion) {
    let mut group = c.benchmark_group("diacritics");
    let items = generate_diacritics_items(10_000);

    // Default: strip diacritics (keep_diacritics = false)
    group.bench_function("strip_diacritics", |b| {
        b.iter(|| {
            match_sorter(
                black_box(&items),
                black_box("cafe"),
                MatchSorterOptions::default(),
            )
        });
    });

    // Keep diacritics (keep_diacritics = true)
    group.bench_function("keep_diacritics", |b| {
        b.iter(|| {
            let opts = MatchSorterOptions {
                keep_diacritics: true,
                ..Default::default()
            };
            match_sorter(black_box(&items), black_box("cafe"), opts)
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// 5. Sort overhead
// ---------------------------------------------------------------------------

fn bench_sort(c: &mut Criterion) {
    let mut group = c.benchmark_group("sort");
    let items = generate_items(10_000);
    let ranked = generate_ranked_items(&items);

    group.bench_function("sort_10k_ranked_items", |b| {
        b.iter_batched(
            || ranked.clone(),
            |mut data| {
                data.sort_by(|a, b| sort_ranked_values(a, b, &default_base_sort));
                data
            },
            criterion::BatchSize::LargeInput,
        );
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Criterion harness
// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    bench_get_match_ranking,
    bench_throughput,
    bench_query_types,
    bench_diacritics,
    bench_sort,
);
criterion_main!(benches);

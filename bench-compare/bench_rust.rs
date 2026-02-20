// Head-to-head benchmark for the Rust matchsorter crate.
// Outputs JSON results to stdout for comparison with the JS library.
//
// Build and run:
//   cargo build --release --example bench_compare
//   ./target/release/examples/bench_compare

use std::time::Instant;

use matchsorter::{MatchSorterOptions, Ranking, match_sorter};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn generate_items(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("item_{i}")).collect()
}

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

/// Run a closure `iterations` times after `warmup` warmup runs.
/// Returns (median_us, min_us, max_us, p25_us, p75_us).
fn benchmark<F: FnMut()>(mut f: F, iterations: usize, warmup: usize) -> BenchResult {
    // Warmup runs (results discarded).
    for _ in 0..warmup {
        f();
    }

    let mut times_us = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        f();
        let elapsed = start.elapsed();
        times_us.push(elapsed.as_secs_f64() * 1_000_000.0);
    }
    times_us.sort_by(|a, b| a.partial_cmp(b).unwrap());

    BenchResult {
        median_us: times_us[iterations / 2],
        min_us: times_us[0],
        max_us: times_us[iterations - 1],
        p25_us: times_us[iterations / 4],
        p75_us: times_us[iterations * 3 / 4],
    }
}

struct BenchResult {
    median_us: f64,
    min_us: f64,
    max_us: f64,
    p25_us: f64,
    p75_us: f64,
}

impl BenchResult {
    fn to_json(&self) -> String {
        format!(
            "{{ \"median_us\": {:.2}, \"min_us\": {:.2}, \"max_us\": {:.2}, \
             \"p25_us\": {:.2}, \"p75_us\": {:.2} }}",
            self.median_us, self.min_us, self.max_us, self.p25_us, self.p75_us
        )
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    // 1. Throughput at different dataset sizes
    let throughput_sizes: &[usize] = &[100, 1_000, 10_000, 100_000];
    let mut throughput_entries = Vec::new();
    for &size in throughput_sizes {
        let items = generate_items(size);
        let iters = if size >= 100_000 { 20 } else { 50 };
        let result = benchmark(
            || {
                let _ = match_sorter(&items, "item_5", MatchSorterOptions::default());
            },
            iters,
            10,
        );
        throughput_entries.push(format!("    \"{size}\": {}", result.to_json()));
    }

    // 2. Query types on 10k items
    let items_10k = generate_items(10_000);

    let exact = benchmark(
        || {
            let _ = match_sorter(&items_10k, "item_500", MatchSorterOptions::default());
        },
        50,
        10,
    );
    let prefix = benchmark(
        || {
            let _ = match_sorter(&items_10k, "item_", MatchSorterOptions::default());
        },
        50,
        10,
    );
    let substring = benchmark(
        || {
            let _ = match_sorter(&items_10k, "_50", MatchSorterOptions::default());
        },
        50,
        10,
    );
    let fuzzy = benchmark(
        || {
            let _ = match_sorter(&items_10k, "im5", MatchSorterOptions::default());
        },
        50,
        10,
    );
    let no_match = benchmark(
        || {
            let _ = match_sorter(&items_10k, "zzzzz", MatchSorterOptions::default());
        },
        50,
        10,
    );

    // 3. Diacritics overhead on 10k items
    let diacritics_items = generate_diacritics_items(10_000);
    let strip = benchmark(
        || {
            let _ = match_sorter(&diacritics_items, "cafe", MatchSorterOptions::default());
        },
        50,
        10,
    );
    let keep = benchmark(
        || {
            let opts = MatchSorterOptions {
                keep_diacritics: true,
                ..Default::default()
            };
            let _ = match_sorter(&diacritics_items, "cafe", opts);
        },
        50,
        10,
    );

    // 4. Threshold filtering
    let thresh_default = benchmark(
        || {
            let _ = match_sorter(&items_10k, "item_5", MatchSorterOptions::default());
        },
        50,
        10,
    );
    let thresh_contains = benchmark(
        || {
            let opts = MatchSorterOptions {
                threshold: Ranking::Contains,
                ..Default::default()
            };
            let _ = match_sorter(&items_10k, "item_5", opts);
        },
        50,
        10,
    );

    // 5. Empty query (all items returned and sorted)
    let items_100 = generate_items(100);
    let empty_100 = benchmark(
        || {
            let _ = match_sorter(&items_100, "", MatchSorterOptions::default());
        },
        50,
        10,
    );
    let empty_10k = benchmark(
        || {
            let _ = match_sorter(&items_10k, "", MatchSorterOptions::default());
        },
        50,
        10,
    );

    // Output JSON
    println!("{{");
    println!("  \"throughput\": {{");
    println!("{}", throughput_entries.join(",\n"));
    println!("  }},");
    println!("  \"query_types\": {{");
    println!("    \"exact\": {},", exact.to_json());
    println!("    \"prefix\": {},", prefix.to_json());
    println!("    \"substring\": {},", substring.to_json());
    println!("    \"fuzzy\": {},", fuzzy.to_json());
    println!("    \"no_match\": {}", no_match.to_json());
    println!("  }},");
    println!("  \"diacritics\": {{");
    println!("    \"strip\": {},", strip.to_json());
    println!("    \"keep\": {}", keep.to_json());
    println!("  }},");
    println!("  \"threshold\": {{");
    println!("    \"default\": {},", thresh_default.to_json());
    println!("    \"contains\": {}", thresh_contains.to_json());
    println!("  }},");
    println!("  \"empty_query\": {{");
    println!("    \"100\": {},", empty_100.to_json());
    println!("    \"10000\": {}", empty_10k.to_json());
    println!("  }}");
    println!("}}");
}

// Head-to-head benchmark for the JavaScript match-sorter library.
// Outputs JSON results to stdout for comparison with the Rust crate.

import { matchSorter, rankings } from "match-sorter";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function generateItems(n) {
  const items = new Array(n);
  for (let i = 0; i < n; i++) {
    items[i] = `item_${i}`;
  }
  return items;
}

function generateDiacriticsItems(n) {
  const items = new Array(n);
  for (let i = 0; i < n; i++) {
    items[i] = i % 2 === 0 ? `caf\u00e9_${i}` : `cafe_${i}`;
  }
  return items;
}

// Measure a function over `iterations` runs, return median time in microseconds.
function benchmark(fn, iterations = 50, warmup = 10) {
  // Warmup
  for (let i = 0; i < warmup; i++) fn();

  const times = new Array(iterations);
  for (let i = 0; i < iterations; i++) {
    const start = performance.now();
    fn();
    times[i] = (performance.now() - start) * 1000; // ms -> us
  }
  times.sort((a, b) => a - b);
  return {
    median_us: times[Math.floor(iterations / 2)],
    min_us: times[0],
    max_us: times[iterations - 1],
    p25_us: times[Math.floor(iterations * 0.25)],
    p75_us: times[Math.floor(iterations * 0.75)],
  };
}

// ---------------------------------------------------------------------------
// Benchmarks -- identical workloads to the Rust crate
// ---------------------------------------------------------------------------

const results = {};

// 1. Throughput at different dataset sizes
const throughputSizes = [100, 1_000, 10_000, 100_000];
results.throughput = {};
for (const size of throughputSizes) {
  const items = generateItems(size);
  results.throughput[size] = benchmark(
    () => matchSorter(items, "item_5"),
    size >= 100_000 ? 20 : 50
  );
}

// 2. Query types on 10k items
const items10k = generateItems(10_000);
results.query_types = {
  exact: benchmark(() => matchSorter(items10k, "item_500")),
  prefix: benchmark(() => matchSorter(items10k, "item_")),
  substring: benchmark(() => matchSorter(items10k, "_50")),
  fuzzy: benchmark(() => matchSorter(items10k, "im5")),
  no_match: benchmark(() => matchSorter(items10k, "zzzzz")),
};

// 3. Diacritics overhead on 10k items
const diacriticsItems = generateDiacriticsItems(10_000);
results.diacritics = {
  strip: benchmark(() => matchSorter(diacriticsItems, "cafe")),
  keep: benchmark(
    () => matchSorter(diacriticsItems, "cafe", { keepDiacritics: true })
  ),
};

// 4. Threshold filtering
results.threshold = {
  default: benchmark(() => matchSorter(items10k, "item_5")),
  contains: benchmark(
    () =>
      matchSorter(items10k, "item_5", { threshold: rankings.CONTAINS })
  ),
};

// 5. Empty query (all items returned and sorted)
results.empty_query = {
  "100": benchmark(() => matchSorter(generateItems(100), "")),
  "10000": benchmark(() => matchSorter(items10k, "")),
};

console.log(JSON.stringify(results, null, 2));

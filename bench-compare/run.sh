#!/usr/bin/env bash
# Run head-to-head benchmarks: Rust matchsorter vs JS match-sorter.
# Outputs a formatted comparison table.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "=== matchsorter: Rust vs JavaScript benchmark ==="
echo ""

# --- Step 1: Build Rust release binary ---
echo "[1/4] Building Rust (release)..."
cargo build --release --example bench_compare --manifest-path "$PROJECT_DIR/Cargo.toml" 2>&1 | tail -1

# --- Step 2: Install JS dependencies ---
echo "[2/4] Installing JS dependencies..."
cd "$SCRIPT_DIR"
npm install --silent 2>&1 | tail -1
cd "$PROJECT_DIR"

# --- Step 3: Run benchmarks ---
echo "[3/4] Running Rust benchmark..."
"$PROJECT_DIR/target/release/examples/bench_compare" > "$SCRIPT_DIR/results_rust.json"

echo "[4/4] Running JS benchmark..."
node "$SCRIPT_DIR/bench.mjs" > "$SCRIPT_DIR/results_js.json"

# --- Step 4: Print comparison ---
node -e "
const rust = JSON.parse(require('fs').readFileSync('$SCRIPT_DIR/results_rust.json', 'utf8'));
const js = JSON.parse(require('fs').readFileSync('$SCRIPT_DIR/results_js.json', 'utf8'));

function fmt(us) {
  if (us >= 1000000) return (us / 1000000).toFixed(2) + 's';
  if (us >= 1000) return (us / 1000).toFixed(2) + 'ms';
  return us.toFixed(0) + 'us';
}

function speedup(js_us, rust_us) {
  const ratio = js_us / rust_us;
  return ratio.toFixed(1) + 'x';
}

const rows = [];

function addRow(category, name, rustData, jsData) {
  rows.push({
    category,
    name,
    rust: fmt(rustData.median_us),
    js: fmt(jsData.median_us),
    speedup: speedup(jsData.median_us, rustData.median_us),
    rust_us: rustData.median_us,
    js_us: jsData.median_us,
  });
}

// Throughput
for (const size of ['100', '1000', '10000', '100000']) {
  if (rust.throughput[size] && js.throughput[size]) {
    addRow('Throughput', size + ' items', rust.throughput[size], js.throughput[size]);
  }
}

// Query types
for (const qt of ['exact', 'prefix', 'substring', 'fuzzy', 'no_match']) {
  if (rust.query_types[qt] && js.query_types[qt]) {
    addRow('Query Type (10k)', qt, rust.query_types[qt], js.query_types[qt]);
  }
}

// Diacritics
for (const d of ['strip', 'keep']) {
  if (rust.diacritics[d] && js.diacritics[d]) {
    addRow('Diacritics (10k)', d, rust.diacritics[d], js.diacritics[d]);
  }
}

// Threshold
for (const t of ['default', 'contains']) {
  if (rust.threshold[t] && js.threshold[t]) {
    addRow('Threshold (10k)', t, rust.threshold[t], js.threshold[t]);
  }
}

// Empty query
for (const e of ['100', '10000']) {
  if (rust.empty_query[e] && js.empty_query[e]) {
    addRow('Empty Query', e + ' items', rust.empty_query[e], js.empty_query[e]);
  }
}

// Print table
const catW = 20, nameW = 16, numW = 12, speedW = 10;
const header = 'Category'.padEnd(catW) + 'Benchmark'.padEnd(nameW) + 'Rust'.padStart(numW) + 'JS'.padStart(numW) + 'Speedup'.padStart(speedW);
const sep = '-'.repeat(header.length);

console.log('');
console.log(header);
console.log(sep);

for (const r of rows) {
  console.log(
    r.category.padEnd(catW) +
    r.name.padEnd(nameW) +
    r.rust.padStart(numW) +
    r.js.padStart(numW) +
    r.speedup.padStart(speedW)
  );
}

console.log(sep);

// Overall geometric mean speedup
const speedups = rows.map(r => r.js_us / r.rust_us);
const geoMean = Math.exp(speedups.reduce((sum, s) => sum + Math.log(s), 0) / speedups.length);
console.log('');
console.log('Geometric mean speedup: ' + geoMean.toFixed(1) + 'x faster (Rust vs JS)');
console.log('');
"

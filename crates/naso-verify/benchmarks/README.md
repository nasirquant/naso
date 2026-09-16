# Formal Verification Benchmarks

This directory contains curated Naso programs with known verification outcomes for the formal benchmark suite.

## Categories

- **quantum/** - Quantum algorithms (Bell pair, QFT, Grover, teleportation, error correction)
- **tensor/** - Tensor operations (matmul, convolution, attention, contractions)
- **linear/** - Linear resource patterns (alloc/free, borrow, move semantics)
- **polyhedral/** - Polyhedral loop optimizations (tiling, fusion, invariants)

## Running Benchmarks

```bash
# Run all benchmarks
cargo run --release -p naso-verify --example run_benchmarks

# Run specific category
cargo run --release -p naso-verify --example run_benchmarks -- quantum

# Output JSON for CI
cargo run --release -p naso-verify --example run_benchmarks -- --format=json > results.json
```

## Expected Outcomes

See `expected_outcomes.json` for the ground truth verification results for each benchmark file.

## Adding Benchmarks

1. Add a `.naso` file to the appropriate category directory
2. Update `expected_outcomes.json` with the expected result
3. Run the benchmark suite to verify it works
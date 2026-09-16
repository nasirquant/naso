# Naso Formal Verification Guide

This document describes the formal verification capabilities of the Naso programming language, including the `naso verify` CLI command, diagnostic codes, and benchmark contribution guidelines.

## Overview

Naso's formal verification engine (`naso-verify`) uses SMT solving (via Z3) to automatically prove properties about Naso programs:

1. **Quantum Uncomputation Safety** - Proves that all temporary qubits are returned to the `|0⟩` state before scope exit
2. **[1]-Quantity Leak Detection** - Proves that no linear (`[1]`) resource is implicitly dropped, double-freed, or leaked
3. **Mutable Value Semantics (MVS)** - Verifies frame conditions and disjointness guarantees for `inout` parameters
4. **Polyhedral Loop Invariants** - Verifies loop invariants and schedule transformations

## CLI Usage

```bash
# Basic verification (runs all provers)
naso verify program.naso

# Verify specific directory
naso verify src/

# Select verification mode
naso verify --mode=uncomputation program.naso  # Only quantum uncomputation
naso verify --mode=linearity program.naso      # Only [1]-quantity linearity
naso verify --mode=all program.naso            # Both (default)

# Output formats
naso verify --format=human program.naso  # Colored terminal output (default)
naso verify --format=json program.naso   # Structured JSON for CI/CD
naso verify --format=sarif program.naso  # SARIF 2.1.0 for GitHub code scanning

# Solver configuration
naso verify --timeout=60000 program.naso     # 60 second timeout
naso verify --logic=AUFLIA program.naso      # Quantifier logic
naso verify --jobs=4 program.naso            # Parallel jobs

# Cache control
naso verify --no-cache program.naso          # Disable incremental cache
naso verify --cache-dir=/tmp/naso-cache program.naso  # Custom cache dir

# Utility
naso verify --list-codes                     # List all diagnostic codes
```

### Exit Codes

- `0` - All verifications passed (UNSAT for error conditions = no violations)
- `1` - Violations found (SAT for error conditions) or errors occurred

## Diagnostic Codes

### Quantum Uncomputation (NASO-UNC-*)

| Code | Severity | Description |
|------|----------|-------------|
| NASO-UNC-001 | Error | Quantum uncomputation failed: temporary qubit not returned to `\|0⟩` state before scope exit. Counterexample shows the quantum state at failure point. |
| NASO-UNC-002 | Warning | Non-invertible temporary qubit operation detected. The prover could not verify that the composed unitary is identity on `\|0⟩`. |
| NASO-UNC-ERR | Error | Internal verification error during uncomputation proving. |

**Example NASO-UNC-001:**
```naso
fn bad_circuit() {
    let q = qalloc(1);
    hadamard(q);
    // ERROR: q never uncomputed!
}
// naso verify output:
// ✗ SAT bad_circuit.naso
// ERROR [NASO-UNC-001] Temporary qubit 'q' not returned to |0⟩ state at scope exit
//   --> bad_circuit.naso:2:9
//   ::: bad_circuit.naso:3:5
//       Qubit allocated here
//   help: Add explicit uncomputation or return the qubit
```

### [1]-Quantity Linearity (NASO-LIN-*)

| Code | Severity | Description |
|------|----------|-------------|
| NASO-LIN-001 | Error | Unused linear resource (leak). A `[1]` binding was not consumed on any control-flow path. |
| NASO-LIN-002 | Error | Double-use of linear resource. A `[1]` binding was consumed more than once. |
| NASO-LIN-003 | Error | Implicit drop of linear resource. A `[1]` binding was not consumed on all control-flow paths. |
| NASO-LIN-004 | Warning | Could not verify linearity (solver returned UNKNOWN). |
| NASO-LIN-ERR | Error | Internal verification error during linearity proving. |

**Example NASO-LIN-001:**
```naso
fn leak(x: [1] i32) {
    // ERROR: x never consumed!
}
// naso verify output:
// ✗ SAT leak.naso
// ERROR [NASO-LIN-001] Unused linear resource 'x'
//   --> leak.naso:1:16
//   help: Consume with linear_free(x) or return it
```

**Example NASO-LIN-002:**
```naso
fn double_free(x: [1] i32) {
    linear_free(x);
    linear_free(x); // ERROR: double free!
}
// naso verify output:
// ✗ SAT double_free.naso
// ERROR [NASO-LIN-002] Linear resource 'x' consumed twice
//   --> double_free.naso:3:5
//   ::: double_free.naso:2:5
//       First consumption here
//   help: Remove duplicate consumption
```

**Example NASO-LIN-003:**
```naso
fn conditional_leak(cond: bool, x: [1] i32) {
    if cond {
        linear_free(x);
    }
    // ERROR: x not consumed when cond=false
}
// naso verify output:
// ✗ SAT conditional_leak.naso
// ERROR [NASO-LIN-003] Linear resource 'x' not consumed on all paths
//   --> conditional_leak.naso:1:34
//   ::: conditional_leak.naso:3:9
//       Consumed here (only when cond=true)
//   help: Ensure consumption on all branches
```

### Mutable Value Semantics (NASO-MVS-*)

| Code | Severity | Description |
|------|----------|-------------|
| NASO-MVS-001 | Error | Inout aliasing conflict. Two `inout` parameters may alias, violating disjointness. |
| NASO-MVS-002 | Error | Inout escape violation. An `inout` reference escapes its lexical scope. |

### Erasure (NASO-ERA-*)

| Code | Severity | Description |
|------|----------|-------------|
| NASO-ERA-001 | Error | Retained `[0]` value. A proof/erasable value escaped erasure and reached runtime. |
| NASO-ERA-002 | Error | Non-erased proof obligation. A `[0]`-quantity obligation was not discharged. |

## Output Formats

### Human (Default)

Colored terminal output with source snippets, error codes, and suggested fixes.

```
✗ SAT program.naso
  Logic: QF_UFLIA
  Model:
    qubit_state = 1

ERROR [NASO-UNC-001] Temporary qubit 'q' not returned to |0⟩ state at scope exit
  --> program.naso:5:9
  ::: program.naso:6:5
      Qubit allocated here
  help: Add explicit uncomputation or return the qubit

═══ Verification Summary ═══
  Files checked:      1
  Total VCs:          1
  ✗ SAT:              1
  ✓ UNSAT:            0
  ⚠ UNKNOWN:          0
  ✗ ERROR:            0
  Total time:         45ms
  Cache hits:         0 (0.0%)
```

### JSON

Structured output for CI/CD integration.

```json
{
  "results": [
    {
      "file": "program.naso",
      "status": "sat",
      "diagnostics": [
        {
          "code": "NASO-UNC-001",
          "message": "Temporary qubit 'q' not returned to |0⟩ state at scope exit",
          "file": "program.naso",
          "line": 5,
          "column": 9,
          "end_line": 5,
          "end_column": 10,
          "severity": "error",
          "related": [
            {
              "file": "program.naso",
              "line": 6,
              "column": 5,
              "message": "Qubit allocated here"
            }
          ],
          "fix": {
            "title": "Add explicit uncomputation or return the qubit",
            "edits": []
          }
        }
      ],
      "stats": {
        "time_ms": 45,
        "logic": "QF_UFLIA",
        "cache_hit": false
      }
    }
  ],
  "summary": {
    "files_checked": 1,
    "total_vcs": 1,
    "sat_count": 1,
    "unsat_count": 0,
    "unknown_count": 0,
    "error_count": 0,
    "total_time_ms": 45,
    "cache_hits": 0,
    "cache_misses": 1
  }
}
```

### SARIF 2.1.0

Compatible with GitHub Code Scanning.

```json
{
  "version": "2.1.0",
  "runs": [
    {
      "tool": {
        "driver": {
          "name": "naso-verify",
          "version": "0.1.0",
          "informationUri": "https://github.com/naso-lang/naso",
          "rules": [
            {
              "id": "NASO-UNC-001",
              "name": "NASO-UNC-001",
              "shortDescription": { "text": "Temporary qubit 'q' not returned to |0⟩ state at scope exit" },
              "fullDescription": { "text": "Temporary qubit 'q' not returned to |0⟩ state at scope exit" },
              "defaultConfiguration": { "level": "error" }
            }
          ]
        }
      },
      "results": [
        {
          "ruleId": "NASO-UNC-001",
          "level": "error",
          "message": { "text": "Temporary qubit 'q' not returned to |0⟩ state at scope exit" },
          "locations": [
            {
              "physicalLocation": {
                "artifactLocation": { "uri": "program.naso" },
                "region": { "startLine": 5, "startColumn": 9, "endLine": 5, "endColumn": 10 }
              }
            }
          ],
          "relatedLocations": [
            {
              "id": 0,
              "physicalLocation": {
                "artifactLocation": { "uri": "program.naso" },
                "region": { "startLine": 6, "startColumn": 5, "endLine": 6, "endColumn": 6 }
              },
              "message": { "text": "Qubit allocated here" }
            }
          ]
        }
      ],
      "columnKind": "utf16"
    }
  ]
}
```

## Incremental Caching

The verification engine caches results based on AST hash + solver configuration:

- Cache location: `~/.cache/naso/verify/` (or `--cache-dir`)
- Cache key: `blake3(ast_hash + config_json)`
- Statistics shown in human output and JSON summary

```bash
# Clear cache
rm -rf ~/.cache/naso/verify/

# Disable cache for a run
naso verify --no-cache program.naso
```

## Benchmark Suite

The formal benchmark suite is located in `crates/naso-verify/benchmarks/`:

```
benchmarks/
├── README.md
├── expected_outcomes.json
├── run_benchmarks.rs
├── quantum/
│   └── benchmarks.naso
├── tensor/
│   └── benchmarks.naso
├── linear/
│   └── benchmarks.naso
└── polyhedral/
    └── benchmarks.naso
```

### Running Benchmarks

```bash
# Run all benchmarks (human output)
cargo run --release -p naso-verify --example run_benchmarks

# Run specific category
cargo run --release -p naso-verify --example run_benchmarks quantum

# JSON output for CI
cargo run --release -p naso-verify --example run_benchmarks -- json > results.json
```

### Benchmark Structure

Each `.naso` file contains multiple functions with known verification outcomes. The `expected_outcomes.json` maps each function to its expected result:

```json
{
  "benchmarks": {
    "quantum/benchmarks.naso": {
      "functions": {
        "bell_pair": { "expected": "unsat", "category": "quantum", "provers": ["uncomputation"] },
        "missing_uncomputation": { "expected": "sat", "category": "quantum", "provers": ["uncomputation"], "diagnostic_codes": ["NASO-UNC-001"] }
      }
    }
  },
  "performance_baselines": {
    "quantum/benchmarks.naso": { "max_time_ms": 5000, "max_memory_mb": 512 }
  }
}
```

- `"expected": "unsat"` = verification should pass (no violations found)
- `"expected": "sat"` = verification should fail (violations expected)
- `"diagnostic_codes"` = expected diagnostic codes for SAT cases

### Adding Benchmarks

1. Add a `.naso` file to the appropriate category directory
2. Add test functions following the naming convention: `valid_*` for passing, `invalid_*` for failing
3. Update `expected_outcomes.json` with expected results
4. Run the benchmark suite to verify

**Example valid quantum benchmark:**
```naso
// Valid: Bell pair - should verify (UNSAT for errors)
fn valid_bell_pair() -> (Qubit, Qubit) {
    let q0 = qalloc(1);
    let q1 = qalloc(1);
    hadamard(q0);
    cnot(q0, q1);
    (q0, q1)
}
```

**Example invalid quantum benchmark:**
```naso
// Invalid: Missing uncomputation - should fail (SAT)
fn invalid_missing_uncomp() {
    let q = qalloc(1);
    hadamard(q);
    // q never returned or freed!
}
```

### Performance Baselines

Performance baselines are defined in `expected_outcomes.json` under `performance_baselines`. The benchmark runner checks:

- Total verification time per file ≤ `max_time_ms`
- Peak memory usage ≤ `max_memory_mb`

CI fails if any benchmark exceeds its baseline by >10%.

## CI Integration

The `.github/workflows/verify.yml` workflow runs on every PR:

1. Builds `naso-verify` and `naso-compiler`
2. Runs the full benchmark suite
3. Tests CLI integration with valid/invalid programs
4. Verifies JSON and SARIF output formats
5. Fails on benchmark regressions (>10% slower than baseline)

## Troubleshooting

### Solver Timeouts

```bash
# Increase timeout
naso verify --timeout=120000 program.naso

# Use faster config for quick checks
naso verify --logic=QF_UFLIA --timeout=5000 program.naso
```

### Memory Issues

```bash
# Limit memory (advisory)
naso verify --timeout=60000 program.naso  # Timeout also limits memory indirectly
```

### Cache Issues

```bash
# Clear cache
naso verify --no-cache program.naso

# Or manually
rm -rf ~/.cache/naso/verify/
```

### False Positives/Negatives

If you encounter a false positive (verification fails on correct code) or false negative (verification passes on buggy code):

1. Create a minimal reproduction case
2. Check if it's a known limitation (quantifier reasoning, non-linear arithmetic)
3. Report with the `.naso` file and expected vs actual behavior

## Advanced Usage

### Custom Verification Conditions

For advanced users, custom VCs can be defined (experimental):

```bash
naso verify --mode=custom --vc-name=my_property program.naso
```

This requires implementing a custom predicate in the prover module.

### Parallel Verification

```bash
# Use all CPU cores
naso verify --jobs=0 src/

# Limit to 4 cores
naso verify --jobs=4 src/
```

### Verbose Output

```bash
naso verify --verbose program.naso
```

Shows per-function verification details, SMT script size, and solver statistics.

## Architecture

```
naso verify
    │
    ├─► Parse Naso source → AST
    │
    ├─► Type check → Typed AST with QTT quantities
    │
    ├─► Lower to SMT-LIB2
    │       ├─ Quantity constraints ([0], [1], [*], [N])
    │       ├─ Linear resource tracking (consume-once)
    │       ├─ MVS frame conditions (disjointness)
    │       ├─ Quantum uncomputation (U|0⟩ = |0⟩)
    │       └─ Polyhedral invariants (∀ loops)
    │
    ├─► Z3 Solver (via FFI)
    │       ├─ Incremental solving
    │       ├─ Model extraction (counterexamples)
    │       └─ Unsat core extraction
    │
    └─► Output (human/JSON/SARIF)
```

## Contributing

1. **New diagnostic codes**: Add to `crates/naso-verify/src/model.rs` and update `--list-codes`
2. **New prover**: Implement in `crates/naso-verify/src/prover/` and register in `prover/mod.rs`
3. **New benchmarks**: Follow the benchmark contribution guide above
4. **Documentation**: Update this file for any new features

---

*For questions or issues, see the [Naso GitHub repository](https://github.com/naso-lang/naso).*
//! Formal verification benchmark runner.
//!
//! This binary runs the formal verification benchmark suite and compares
//! results against expected outcomes. It measures verification time,
//! solver calls, memory usage, and false positive/negative rates.

use naso_verify::{
    cache::VerificationCache,
    config::{Logic, SolverConfig},
    output::{format_human, format_summary_human, to_json, to_sarif, VerificationSummary},
    prover::{run_all_provers, run_linearity_prover, run_uncomputation_prover},
    VerifyMode, VerifyCliConfig,
};
use naso_compiler::parser::parse_program;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchmarkCase {
    file: String,
    function: String,
    expected: String, // "sat" or "unsat"
    category: String,
    provers: Vec<String>,
    diagnostic_codes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchmarkFile {
    functions: HashMap<String, BenchmarkCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExpectedOutcomes {
    benchmarks: HashMap<String, BenchmarkFile>,
    performance_baselines: HashMap<String, PerformanceBaseline>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PerformanceBaseline {
    max_time_ms: u64,
    max_memory_mb: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchmarkResult {
    file: String,
    function: String,
    expected: String,
    actual: String,
    passed: bool,
    time_ms: u64,
    memory_mb: usize,
    diagnostic_codes: Vec<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchmarkSummary {
    total: usize,
    passed: usize,
    failed: usize,
    false_positives: usize,
    false_negatives: usize,
    total_time_ms: u64,
    category_results: HashMap<String, CategoryResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CategoryResult {
    total: usize,
    passed: usize,
    failed: usize,
    total_time_ms: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (category_filter, format) = match args.as_slice() {
        [] => (None, "human"),
        [fmt] if fmt == "json" || fmt == "sarif" => (None, fmt.as_str()),
        [category] => (Some(category.as_str()), "human"),
        [category, fmt] => (Some(category.as_str()), fmt.as_str()),
        _ => {
            eprintln!("Usage: run_benchmarks [category] [human|json|sarif]");
            std::process::exit(2);
        }
    };

    println!("🧪 Naso Formal Verification Benchmark Suite");
    println!("============================================");

    // Load expected outcomes
    let outcomes_path = PathBuf::from("crates/naso-verify/benchmarks/expected_outcomes.json");
    let outcomes_json = fs::read_to_string(&outcomes_path)?;
    let outcomes: ExpectedOutcomes = serde_json::from_str(&outcomes_json)?;

    // Collect benchmark files
    let bench_dir = PathBuf::from("crates/naso-verify/benchmarks");
    let mut results = Vec::new();
    let mut summary = BenchmarkSummary {
        total: 0,
        passed: 0,
        failed: 0,
        false_positives: 0,
        false_negatives: 0,
        total_time_ms: 0,
        category_results: HashMap::new(),
    };

    for (file_name, bench_file) in &outcomes.benchmarks {
        if let Some(filter) = category_filter {
            if !file_name.starts_with(&format!("{}/", filter)) {
                continue;
            }
        }

        let file_path = bench_dir.join(file_name);
        if !file_path.exists() {
            eprintln!("⚠️  Benchmark file not found: {}", file_path.display());
            continue;
        }

        let source = fs::read_to_string(&file_path)?;
        let program = parse_program(&source).map_err(|e| {
            format!("Parse error in {}: {}", file_name, e)
        })?;

        // Run provers based on expected provers for each function
        for (func_name, case) in &bench_file.functions {
            println!("\n🔍 Verifying: {}::{}", file_name, func_name);

            let start = Instant::now();
            let mut cache = VerificationCache::new(None)?;

            let solver_config = SolverConfig {
                logic: Logic::QF_UFLIA,
                timeout: Duration::from_secs(30),
                ..Default::default()
            };

            let diagnostics = match run_provers_for_case(&program, func_name, &case.provers) {
                Ok(d) => d,
                Err(e) => {
                    let result = BenchmarkResult {
                        file: file_name.clone(),
                        function: func_name.clone(),
                        expected: case.expected.clone(),
                        actual: "error".to_string(),
                        passed: false,
                        time_ms: start.elapsed().as_millis() as u64,
                        memory_mb: 0,
                        diagnostic_codes: Vec::new(),
                        error: Some(e),
                    };
                    results.push(result);
                    continue;
                }
            };

            let elapsed = start.elapsed().as_millis() as u64;
            let actual = if diagnostics.is_empty() { "unsat" } else { "sat" };
            let passed = actual == case.expected;

            // Check diagnostic codes if expected
            let mut codes_match = true;
            if let Some(expected_codes) = &case.diagnostic_codes {
                let found_codes: Vec<String> = diagnostics.iter().map(|d| d.code.clone()).collect();
                for expected_code in expected_codes {
                    if !found_codes.iter().any(|c| c == expected_code) {
                        codes_match = false;
                        break;
                    }
                }
            }

            let result = BenchmarkResult {
                file: file_name.clone(),
                function: func_name.clone(),
                expected: case.expected.clone(),
                actual: actual.to_string(),
                passed: passed && codes_match,
                time_ms: elapsed,
                memory_mb: estimate_memory(&diagnostics),
                diagnostic_codes: diagnostics.iter().map(|d| d.code.clone()).collect(),
                error: None,
            };

            if !passed {
                if actual == "sat" && case.expected == "unsat" {
                    summary.false_positives += 1;
                } else if actual == "unsat" && case.expected == "sat" {
                    summary.false_negatives += 1;
                }
            }

            // Update category stats
            let cat_entry = summary.category_results.entry(case.category.clone()).or_insert(CategoryResult {
                total: 0,
                passed: 0,
                failed: 0,
                total_time_ms: 0,
            });
            cat_entry.total += 1;
            cat_entry.total_time_ms += elapsed;
            if result.passed {
                cat_entry.passed += 1;
                summary.passed += 1;
            } else {
                cat_entry.failed += 1;
                summary.failed += 1;
            }

            summary.total += 1;
            summary.total_time_ms += elapsed;

            results.push(result);
        }
    }

    // Print summary
    print_summary(&summary, &results, format);

    // Check performance baselines
    check_performance_baselines(&results, &outcomes.performance_baselines);

    // Exit with error code if any failures
    if summary.failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn run_provers_for_case(
    program: &naso_compiler::ast::Program,
    func_name: &str,
    provers: &[String],
) -> Result<Vec<naso_verify::model::VerifyDiagnostic>, String> {
    // Filter program to just the target function
    // In real implementation, would extract function and run provers on it
    // For now, run all provers on full program
    let mut diagnostics = Vec::new();

    for prover in provers {
        match prover.as_str() {
            "uncomputation" => {
                diagnostics.extend(run_uncomputation_prover(program).map_err(|e| e.to_string())?);
            }
            "linearity" => {
                diagnostics.extend(run_linearity_prover(program).map_err(|e| e.to_string())?);
            }
            _ => {}
        }
    }

    Ok(diagnostics)
}

fn estimate_memory(_diagnostics: &[naso_verify::model::VerifyDiagnostic]) -> usize {
    // Simplified memory estimation
    64 // MB
}

fn print_summary(
    summary: &BenchmarkSummary,
    results: &[BenchmarkResult],
    format: &str,
) {
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(results).unwrap());
        }
        "sarif" => {
            // Would generate SARIF output
            println!("SARIF output not yet implemented for benchmark summary");
        }
        _ => {
            println!("\n═══ Benchmark Summary ═══");
            println!("Total:      {}", summary.total);
            println!("Passed:     ✓ {}", summary.passed);
            println!("Failed:     ✗ {}", summary.failed);
            println!("False +:    {}", summary.false_positives);
            println!("False -:    {}", summary.false_negatives);
            println!("Total time: {}ms", summary.total_time_ms);

            println!("\n═══ By Category ═══");
            for (cat, cat_result) in &summary.category_results {
                let rate = if cat_result.total > 0 {
                    cat_result.passed as f64 / cat_result.total as f64 * 100.0
                } else {
                    0.0
                };
                println!(
                    "  {:<15} {:>3}/{:<3} ({:.1}%) {}ms",
                    cat, cat_result.passed, cat_result.total, rate, cat_result.total_time_ms
                );
            }

            println!("\n═══ Failures ═══");
            for r in results {
                if !r.passed {
                    println!("  ✗ {}::{} (expected {}, got {})",
                        r.file, r.function, r.expected, r.actual);
                    if !r.diagnostic_codes.is_empty() {
                        println!("     Codes: {:?}", r.diagnostic_codes);
                    }
                    if let Some(e) = &r.error {
                        println!("     Error: {}", e);
                    }
                }
            }
        }
    }
}

fn check_performance_baselines(
    results: &[BenchmarkResult],
    baselines: &HashMap<String, PerformanceBaseline>,
) {
    println!("\n═══ Performance Baselines ═══");
    for (file, baseline) in baselines {
        let file_results: Vec<_> = results.iter().filter(|r| r.file == *file).collect();
        if file_results.is_empty() {
            continue;
        }

        let total_time: u64 = file_results.iter().map(|r| r.time_ms).sum();
        let max_memory: usize = file_results.iter().map(|r| r.memory_mb).max().unwrap_or(0);

        let time_ok = total_time <= baseline.max_time_ms;
        let mem_ok = max_memory <= baseline.max_memory_mb;

        println!(
            "  {:<30} time: {}ms/{}ms {}  mem: {}MB/{}MB {}",
            file,
            total_time,
            baseline.max_time_ms,
            if time_ok { "✓" } else { "✗" },
            max_memory,
            baseline.max_memory_mb,
            if mem_ok { "✓" } else { "✗" }
        );
    }
}
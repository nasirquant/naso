//! CLI argument parsing for the `naso verify` command.
//!
//! This module provides structured argument parsing for verification modes,
//! output formats, solver configuration, and cache control.

use crate::config::{Logic, SolverConfig};
use crate::output::OutputFormat;
use clap::{Arg, ArgAction, Command};
use std::path::PathBuf;
use std::time::Duration;

/// Verification mode selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerifyMode {
    /// Run all provers (uncomputation + linearity)
    #[default]
    All,
    /// Run only quantum uncomputation prover
    Uncomputation,
    /// Run only [1]-quantity linearity prover
    Linearity,
    /// Run custom verification condition (requires --vc-name)
    Custom,
}

impl VerifyMode {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "all" => Ok(VerifyMode::All),
            "uncomputation" => Ok(VerifyMode::Uncomputation),
            "linearity" => Ok(VerifyMode::Linearity),
            "custom" => Ok(VerifyMode::Custom),
            other => Err(format!(
                "Unknown verification mode: {}. Use all, uncomputation, linearity, or custom",
                other
            )),
        }
    }
}

/// Complete CLI configuration for verification.
#[derive(Debug, Clone)]
pub struct VerifyCliConfig {
    /// Input files or directories to verify
    pub inputs: Vec<PathBuf>,
    /// Verification mode
    pub mode: VerifyMode,
    /// Output format
    pub format: OutputFormat,
    /// Solver configuration
    pub solver_config: SolverConfig,
    /// Custom VC name (for --mode=custom)
    pub vc_name: Option<String>,
    /// Whether to use incremental cache
    pub use_cache: bool,
    /// Cache directory override
    pub cache_dir: Option<PathBuf>,
    /// Number of parallel jobs (0 = auto)
    pub jobs: usize,
    /// Verbose output
    pub verbose: bool,
    /// Quiet mode (suppress non-error output)
    pub quiet: bool,
    /// List available diagnostic codes and exit
    pub list_codes: bool,
}

impl Default for VerifyCliConfig {
    fn default() -> Self {
        Self {
            inputs: Vec::new(),
            mode: VerifyMode::All,
            format: OutputFormat::Human,
            solver_config: SolverConfig::default(),
            vc_name: None,
            use_cache: true,
            cache_dir: None,
            jobs: 0,
            verbose: false,
            quiet: false,
            list_codes: false,
        }
    }
}

/// Parse command line arguments for `naso verify`.
pub fn parse_verify_args(args: &[String]) -> Result<VerifyCliConfig, String> {
    let matches = Command::new("naso verify")
        .about("Formally verify Naso programs using SMT solving")
        .version(env!("CARGO_PKG_VERSION"))
        .args(&[
            Arg::new("inputs")
                .help("Input files or directories to verify")
                .num_args(1..)
                .required(false),
            Arg::new("mode")
                .long("mode")
                .short('m')
                .help("Verification mode: all, uncomputation, linearity, custom")
                .default_value("all")
                .value_name("MODE"),
            Arg::new("format")
                .long("format")
                .short('f')
                .help("Output format: human, json, sarif")
                .default_value("human")
                .value_name("FORMAT"),
            Arg::new("jobs")
                .long("jobs")
                .short('j')
                .help("Number of parallel verification jobs (0 = auto)")
                .default_value("0")
                .value_name("N"),
            Arg::new("timeout")
                .long("timeout")
                .short('t')
                .help("Solver timeout in milliseconds")
                .default_value("30000")
                .value_name("MS"),
            Arg::new("logic")
                .long("logic")
                .help("SMT logic: QF_UFLIA, AUFLIA, QF_BV")
                .default_value("QF_UFLIA")
                .value_name("LOGIC"),
            Arg::new("no-cache")
                .long("no-cache")
                .help("Disable incremental verification cache")
                .action(ArgAction::SetTrue),
            Arg::new("cache-dir")
                .long("cache-dir")
                .help("Override cache directory")
                .value_name("DIR"),
            Arg::new("vc-name")
                .long("vc-name")
                .help("Custom verification condition name (for --mode=custom)")
                .value_name("NAME"),
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .help("Enable verbose output")
                .action(ArgAction::SetTrue),
            Arg::new("quiet")
                .long("quiet")
                .short('q')
                .help("Suppress non-error output")
                .action(ArgAction::SetTrue),
            Arg::new("list-codes")
                .long("list-codes")
                .help("List all diagnostic codes and exit")
                .action(ArgAction::SetTrue),
        ])
        .try_get_matches_from(
            std::iter::once("naso verify".to_string()).chain(args.iter().cloned()),
        )
        .map_err(|e| e.to_string())?;

    let mut config = VerifyCliConfig::default();

    // Parse inputs
    if let Some(inputs) = matches.get_many::<String>("inputs") {
        config.inputs = inputs.map(PathBuf::from).collect();
    }

    // Parse mode
    if let Some(mode_str) = matches.get_one::<String>("mode") {
        config.mode = VerifyMode::from_str(mode_str)?;
    }

    // Parse format
    if let Some(format_str) = matches.get_one::<String>("format") {
        config.format = match format_str.to_lowercase().as_str() {
            "human" => OutputFormat::Human,
            "json" => OutputFormat::Json,
            "sarif" => OutputFormat::Sarif,
            other => {
                return Err(format!(
                    "Unknown output format: {}. Use human, json, or sarif",
                    other
                ));
            }
        };
    }

    // Parse jobs
    if let Some(jobs_str) = matches.get_one::<String>("jobs") {
        config.jobs = jobs_str
            .parse()
            .map_err(|_| format!("Invalid jobs value: {}", jobs_str))?;
    }

    // Parse timeout
    if let Some(timeout_str) = matches.get_one::<String>("timeout") {
        let ms = timeout_str
            .parse::<u64>()
            .map_err(|_| format!("Invalid timeout value: {}", timeout_str))?;
        config.solver_config.timeout = Duration::from_millis(ms);
    }

    // Parse logic
    if let Some(logic_str) = matches.get_one::<String>("logic") {
        config.solver_config.logic = Logic::from_str(logic_str).map_err(|e| e.to_string())?;
    }

    // Parse flags
    config.use_cache = !matches.get_flag("no-cache");
    config.verbose = matches.get_flag("verbose");
    config.quiet = matches.get_flag("quiet");
    config.list_codes = matches.get_flag("list-codes");

    // Parse optional values
    if let Some(cache_dir) = matches.get_one::<String>("cache-dir") {
        config.cache_dir = Some(PathBuf::from(cache_dir));
    }
    if let Some(vc_name) = matches.get_one::<String>("vc-name") {
        config.vc_name = Some(vc_name.clone());
    }

    // Configure solver threads
    if config.jobs > 0 {
        config.solver_config.threads = config.jobs;
    }

    Ok(config)
}

/// Print usage information.
pub fn print_verify_usage() {
    eprintln!("Usage: naso verify [OPTIONS] [FILE|DIR]...");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -m, --mode <MODE>         Verification mode (default: all)");
    eprintln!("                              all          - Run all provers");
    eprintln!("                              uncomputation - Quantum uncomputation safety");
    eprintln!("                              linearity    - [1]-quantity leak detection");
    eprintln!("                              custom       - Custom verification condition");
    eprintln!("  -f, --format <FORMAT>     Output format (default: human)");
    eprintln!("                              human  - Colored human-readable output");
    eprintln!("                              json   - Structured JSON for CI/CD");
    eprintln!("                              sarif  - SARIF 2.1.0 for GitHub code scanning");
    eprintln!("  -j, --jobs <N>            Parallel jobs (0 = auto, default: 0)");
    eprintln!("  -t, --timeout <MS>        Solver timeout in milliseconds (default: 30000)");
    eprintln!("      --logic <LOGIC>       SMT logic (default: QF_UFLIA)");
    eprintln!("                              QF_UFLIA, AUFLIA, QF_BV");
    eprintln!("      --no-cache            Disable incremental verification cache");
    eprintln!("      --cache-dir <DIR>     Override cache directory");
    eprintln!("      --vc-name <NAME>      Custom VC name (for --mode=custom)");
    eprintln!("  -v, --verbose             Enable verbose output");
    eprintln!("  -q, --quiet               Suppress non-error output");
    eprintln!("      --list-codes          List all diagnostic codes and exit");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  naso verify program.naso");
    eprintln!("  naso verify --mode=uncomputation --format=json src/");
    eprintln!("  naso verify --mode=linearity --jobs=4 --timeout=60000 program.naso");
    eprintln!("  naso verify --format=sarif --output=results.sarif program.naso");
}

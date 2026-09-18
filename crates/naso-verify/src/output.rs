//! Output formatting for verification results.
//!
//! This module provides human-readable, JSON, and SARIF output formats
//! for the `naso verify` command.

#[cfg(feature = "z3")]
use crate::config::SolverConfig;
#[cfg(feature = "z3")]
use crate::model::{Counterexample, DiagnosticSeverity, Model, UnsatCore, VerifyDiagnostic};
#[cfg(feature = "z3")]
use crate::solver::VerifyResult;
#[cfg(feature = "z3")]
use colored::Colorize;
#[cfg(feature = "z3")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "z3")]
use std::collections::HashMap;
#[cfg(feature = "z3")]
use std::time::Duration;

/// Output format for verification results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Human-readable colored output
    Human,
    /// JSON for CI/CD integration
    Json,
    /// SARIF 2.1.0 for GitHub code scanning
    Sarif,
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Human
    }
}

#[cfg(feature = "z3")]
/// Verification result summary for output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationSummary {
    pub total_functions: usize,
    pub verified: usize,
    pub failed: usize,
    pub unknown: usize,
    pub errors: usize,
    pub total_time: Duration,
    pub diagnostics: Vec<VerifyDiagnostic>,
}

#[cfg(feature = "z3")]
impl VerificationSummary {
    pub fn new() -> Self {
        Self {
            total_functions: 0,
            verified: 0,
            failed: 0,
            unknown: 0,
            errors: 0,
            total_time: Duration::default(),
            diagnostics: Vec::new(),
        }
    }

    pub fn add_result(&mut self, result: &VerifyResult, diagnostics: Vec<VerifyDiagnostic>) {
        self.total_functions += 1;
        match result {
            VerifyResult::Unsat(_) => self.verified += 1,
            VerifyResult::Sat(_) => self.failed += 1,
            VerifyResult::Unknown(_) => self.unknown += 1,
            VerifyResult::Error(_) => self.errors += 1,
        }
        self.diagnostics.extend(diagnostics);
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_functions == 0 {
            0.0
        } else {
            self.verified as f64 / self.total_functions as f64
        }
    }
}

#[cfg(feature = "z3")]
/// Format verification results for human-readable output.
pub fn format_human(summary: &VerificationSummary, config: &SolverConfig) -> String {
    use colored::Colorize;

    let mut out = String::new();

    out.push_str(&format!(
        "{} {}/{} functions verified ({:.1}%)\n",
        "Verification Summary:".bold().cyan(),
        summary.verified,
        summary.total_functions,
        summary.success_rate() * 100.0
    ));

    out.push_str(&format!(
        "  {} verified, {} failed, {} unknown, {} errors\n",
        summary.verified.to_string().green(),
        summary.failed.to_string().red(),
        summary.unknown.to_string().yellow(),
        summary.errors.to_string().red()
    ));

    out.push_str(&format!(
        "  Total time: {:.2}s\n",
        summary.total_time.as_secs_f64()
    ));

    if !summary.diagnostics.is_empty() {
        out.push_str("\n");
        out.push_str(&"Diagnostics:".bold().underline().to_string());
        out.push_str("\n");

        for diag in &summary.diagnostics {
            let severity_color = match diag.severity {
                DiagnosticSeverity::Error => "ERROR".red().bold(),
                DiagnosticSeverity::Warning => "WARN".yellow().bold(),
                DiagnosticSeverity::Info => "INFO".blue().bold(),
                DiagnosticSeverity::Hint => "HINT".cyan().bold(),
            };

            out.push_str(&format!(
                "  [{}] {}: {}\n",
                severity_color,
                diag.code.bold(),
                diag.message
            ));

            out.push_str(&format!(
                "     at <unknown>:{}:{}\n",
                diag.span.line.to_string().cyan(),
                diag.span.column.to_string().cyan()
            ));

            if !diag.related.is_empty() {
                out.push_str("  Related:\n");
                for related in &diag.related {
                    out.push_str(&format!(
                        "    at <unknown>:{}:{} - {}\n",
                        related.span.line.to_string().cyan(),
                        related.span.column.to_string().cyan(),
                        related.message
                    ));
                }
            }

            if let Some(fix) = &diag.fix {
                out.push_str(&format!("  Fix: {}\n", fix.title.bold()));
            }
        }
    }

    out
}

#[cfg(feature = "z3")]
/// Format verification results as JSON.
pub fn format_json(summary: &VerificationSummary) -> Result<String, serde_json::Error> {
    #[derive(Serialize)]
    struct JsonOutput {
        summary: VerificationSummary,
    }

    let output = JsonOutput {
        summary: summary.clone(),
    };
    serde_json::to_string_pretty(&output)
}

#[cfg(feature = "z3")]
/// Format verification results as SARIF 2.1.0.
pub fn format_sarif(summary: &VerificationSummary) -> Result<String, serde_json::Error> {
    #[derive(Serialize)]
    struct SarifOutput {
        version: String,
        runs: Vec<SarifRun>,
    }

    #[derive(Serialize)]
    struct SarifRun {
        tool: SarifTool,
        results: Vec<SarifResult>,
    }

    #[derive(Serialize)]
    struct SarifTool {
        driver: SarifDriver,
    }

    #[derive(Serialize)]
    struct SarifDriver {
        name: String,
        information_uri: String,
        rules: Vec<SarifRule>,
    }

    #[derive(Serialize)]
    struct SarifRule {
        id: String,
        name: String,
        short_description: SarifDescription,
        full_description: SarifDescription,
        default_configuration: SarifConfiguration,
    }

    #[derive(Serialize)]
    struct SarifDescription {
        text: String,
    }

    #[derive(Serialize)]
    struct SarifConfiguration {
        level: String,
    }

    #[derive(Serialize)]
    struct SarifResult {
        rule_id: String,
        level: String,
        message: SarifMessage,
        locations: Vec<SarifLocation>,
    }

    #[derive(Serialize)]
    struct SarifMessage {
        text: String,
    }

    #[derive(Serialize)]
    struct SarifLocation {
        physical_location: SarifPhysicalLocation,
    }

    #[derive(Serialize)]
    struct SarifPhysicalLocation {
        artifact_location: SarifArtifactLocation,
        region: SarifRegion,
    }

    #[derive(Serialize)]
    struct SarifArtifactLocation {
        uri: String,
    }

    #[derive(Serialize)]
    struct SarifRegion {
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
    }

    let mut rules = Vec::new();
    let mut results = Vec::new();

    for diag in &summary.diagnostics {
        // Add rule if not already present
        if !rules.iter().any(|r: &SarifRule| r.id == diag.code) {
            rules.push(SarifRule {
                id: diag.code.clone(),
                name: diag.code.clone(),
                short_description: SarifDescription {
                    text: diag.message.clone(),
                },
                full_description: SarifDescription {
                    text: diag.message.clone(),
                },
                default_configuration: SarifConfiguration {
                    level: match diag.severity {
                        DiagnosticSeverity::Error => "error".to_string(),
                        DiagnosticSeverity::Warning => "warning".to_string(),
                        DiagnosticSeverity::Info => "note".to_string(),
                        DiagnosticSeverity::Hint => "note".to_string(),
                    },
                },
            });
        }

        // Add result
        results.push(SarifResult {
            rule_id: diag.code.clone(),
            level: match diag.severity {
                DiagnosticSeverity::Error => "error".to_string(),
                DiagnosticSeverity::Warning => "warning".to_string(),
                DiagnosticSeverity::Info => "note".to_string(),
                DiagnosticSeverity::Hint => "note".to_string(),
            },
            message: SarifMessage {
                text: diag.message.clone(),
            },
            locations: vec![SarifLocation {
                physical_location: SarifPhysicalLocation {
                    artifact_location: SarifArtifactLocation {
                        uri: "<unknown>".to_string(),
                    },
                    region: SarifRegion {
                        start_line: diag.span.line as usize,
                        start_column: diag.span.column as usize,
                        end_line: diag.span.line as usize,
                        end_column: diag.span.column as usize,
                    },
                },
            }],
        });
    }

    let sarif = SarifOutput {
        version: "2.1.0".to_string(),
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name: "Naso Verify".to_string(),
                    information_uri: "https://github.com/naso-lang/naso".to_string(),
                    rules,
                },
            },
            results,
        }],
    };

    serde_json::to_string_pretty(&sarif)
}

#[cfg(feature = "z3")]
/// Format a single verification result for output.
pub fn format_result(result: &VerifyResult, config: &SolverConfig) -> String {
    use colored::Colorize;

    match result {
        VerifyResult::Sat(model) => {
            let mut out = String::new();
            out.push_str(&"UNSAT (property violated)".red().bold().to_string());
            out.push_str("\n");
            out.push_str(&format!("  Model: {:?}\n", model));
            out
        }
        VerifyResult::Unsat(core) => {
            let mut out = String::new();
            out.push_str(&"SAT (property holds)".green().bold().to_string());
            out.push_str("\n");
            if let Some(core) = core {
                out.push_str(&format!("  Unsat core: {}\n", core.format()));
            }
            out
        }
        VerifyResult::Unknown(reason) => {
            format!("{} {}\n", "UNKNOWN".yellow().bold(), reason)
        }
        VerifyResult::Error(msg) => {
            format!("{} {}\n", "ERROR".red().bold(), msg)
        }
    }
}

#[cfg(feature = "z3")]
/// Print verification results to stdout.
pub fn print_results(summary: &VerificationSummary, format: OutputFormat, config: &SolverConfig) {
    match format {
        OutputFormat::Human => {
            println!("{}", format_human(summary, config));
        }
        OutputFormat::Json => {
            if let Ok(json) = format_json(summary) {
                println!("{}", json);
            }
        }
        OutputFormat::Sarif => {
            if let Ok(sarif) = format_sarif(summary) {
                println!("{}", sarif);
            }
        }
    }
}

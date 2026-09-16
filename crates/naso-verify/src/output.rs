//! Output formatting for verification results.
//!
//! This module provides human-readable, JSON, and SARIF output formats
//! for the `naso verify` command.

use crate::config::SolverConfig;
use crate::model::{Counterexample, DiagnosticSeverity, Model, UnsatCore, VerifyDiagnostic};
use crate::solver::VerifyResult;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

/// Verification summary statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VerificationSummary {
    pub files_checked: usize,
    pub total_vcs: usize,
    pub sat_count: usize,
    pub unsat_count: usize,
    pub unknown_count: usize,
    pub error_count: usize,
    pub total_time_ms: u64,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

impl VerificationSummary {
    pub fn success_rate(&self) -> f64 {
        if self.total_vcs == 0 {
            1.0
        } else {
            (self.unsat_count + self.sat_count) as f64 / self.total_vcs as f64
        }
    }
}

/// Format a verification result for human output.
pub fn format_human(
    result: &VerifyResult,
    file: &str,
    config: &SolverConfig,
    summary: &mut VerificationSummary,
) -> String {
    let mut out = String::new();

    match result {
        VerifyResult::Sat(model) => {
            summary.sat_count += 1;
            out.push_str(&format!(
                "{} {} {}\n",
                "✗".red().bold(),
                "SAT".red().bold(),
                file.cyan()
            ));
            out.push_str(&format!("  Logic: {}\n", config.logic.as_str()));
            out.push_str(&format!("  Model:\n"));
            for (name, value) in &model.assignments {
                out.push_str(&format!("    {} = {}\n", name.yellow(), value));
            }
        }
        VerifyResult::Unsat(core) => {
            summary.unsat_count += 1;
            out.push_str(&format!(
                "{} {} {}\n",
                "✓".green().bold(),
                "UNSAT".green().bold(),
                file.cyan()
            ));
            if let Some(core) = core {
                out.push_str(&format!("  Unsatisfiable core:\n"));
                for a in &core.assertions {
                    out.push_str(&format!("    - {}\n", a));
                }
                out.push_str(&format!("  {}\n", core.explanation));
            }
        }
        VerifyResult::Unknown(reason) => {
            summary.unknown_count += 1;
            out.push_str(&format!(
                "{} {} {}: {}\n",
                "?".yellow().bold(),
                "UNKNOWN".yellow().bold(),
                file.cyan(),
                reason
            ));
        }
        VerifyResult::Error(msg) => {
            summary.error_count += 1;
            out.push_str(&format!(
                "{} {} {}: {}\n",
                "!".red().bold(),
                "ERROR".red().bold(),
                file.cyan(),
                msg
            ));
        }
    }

    summary.total_vcs += 1;
    out
}

/// Format a diagnostic for human output.
pub fn format_diagnostic_human(diag: &VerifyDiagnostic) -> String {
    let severity_color = match diag.severity {
        DiagnosticSeverity::Error => "ERROR".red().bold(),
        DiagnosticSeverity::Warning => "WARNING".yellow().bold(),
        DiagnosticSeverity::Info => "INFO".blue().bold(),
        DiagnosticSeverity::Hint => "HINT".green().bold(),
    };

    let mut out = String::new();
    out.push_str(&format!(
        "{} [{}] {}\n",
        severity_color, diag.code, diag.message
    ));
    out.push_str(&format!(
        "  --> {}:{}:{}\n",
        diag.span.file().cyan(),
        diag.span.line().to_string().cyan(),
        diag.span.column().to_string().cyan()
    ));

    for related in &diag.related {
        out.push_str(&format!(
            "  ::: {}:{}:{}\n",
            related.span.file().cyan(),
            related.span.line().to_string().cyan(),
            related.span.column().to_string().cyan()
        ));
        out.push_str(&format!("      {}\n", related.message));
    }

    if let Some(fix) = &diag.fix {
        out.push_str(&format!("  {}: {}\n", "help".green(), fix.title));
        for edit in &fix.edits {
            out.push_str(&format!(
                "    {}..{}: replace with `{}`\n",
                edit.span.start(),
                edit.span.end(),
                edit.new_text
            ));
        }
    }

    out
}

/// Format verification summary for human output.
pub fn format_summary_human(summary: &VerificationSummary) -> String {
    let mut out = String::new();
    out.push_str("\n");
    out.push_str(&"═══ Verification Summary ═══\n".bold().to_string());
    out.push_str(&format!(
        "  Files checked:      {}\n",
        summary.files_checked
    ));
    out.push_str(&format!("  Total VCs:          {}\n", summary.total_vcs));
    out.push_str(&format!(
        "  {} SAT\n",
        format!("  SAT:              {}", summary.sat_count).red()
    ));
    out.push_str(&format!(
        "  {} UNSAT\n",
        format!("  UNSAT:            {}", summary.unsat_count).green()
    ));
    out.push_str(&format!(
        "  {} UNKNOWN\n",
        format!("  UNKNOWN:          {}", summary.unknown_count).yellow()
    ));
    out.push_str(&format!(
        "  {} ERROR\n",
        format!("  ERROR:            {}", summary.error_count).red()
    ));
    out.push_str(&format!(
        "  Total time:         {}ms\n",
        summary.total_time_ms
    ));
    out.push_str(&format!(
        "  Cache hits:         {} ({:.1}%)\n",
        summary.cache_hits,
        if summary.cache_hits + summary.cache_misses > 0 {
            summary.cache_hits as f64 / (summary.cache_hits + summary.cache_misses) as f64 * 100.0
        } else {
            0.0
        }
    ));
    out
}

/// JSON output structure.
#[derive(Debug, Serialize)]
pub struct JsonOutput {
    pub results: Vec<JsonFileResult>,
    pub summary: VerificationSummary,
}

#[derive(Debug, Serialize)]
pub struct JsonFileResult {
    pub file: String,
    pub status: String, // "sat", "unsat", "unknown", "error"
    pub diagnostics: Vec<JsonDiagnostic>,
    pub stats: JsonStats,
}

#[derive(Debug, Serialize)]
pub struct JsonDiagnostic {
    pub code: String,
    pub message: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub severity: String,
    pub related: Vec<JsonRelated>,
    pub fix: Option<JsonFix>,
}

#[derive(Debug, Serialize)]
pub struct JsonRelated {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct JsonFix {
    pub title: String,
    pub edits: Vec<JsonEdit>,
}

#[derive(Debug, Serialize)]
pub struct JsonEdit {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub new_text: String,
}

#[derive(Debug, Serialize)]
pub struct JsonStats {
    pub time_ms: u64,
    pub logic: String,
    pub cache_hit: bool,
}

/// Convert verification results to JSON output.
pub fn to_json(
    results: Vec<(String, VerifyResult)>,
    config: &SolverConfig,
    summary: &VerificationSummary,
) -> String {
    let mut json_results = Vec::new();

    for (file, result) in results {
        let (status, diagnostics) = match &result {
            VerifyResult::Sat(_) => ("sat", vec![]),
            VerifyResult::Unsat(core) => ("unsat", vec![]),
            VerifyResult::Unknown(reason) => ("unknown", vec![]),
            VerifyResult::Error(msg) => ("error", vec![]),
        };

        json_results.push(JsonFileResult {
            file,
            status: status.to_string(),
            diagnostics,
            stats: JsonStats {
                time_ms: 0, // Would need per-file timing
                logic: config.logic.as_str().to_string(),
                cache_hit: false,
            },
        });
    }

    let output = JsonOutput {
        results: json_results,
        summary: summary.clone(),
    };

    serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
}

/// SARIF 2.1.0 output structure.
#[derive(Debug, Serialize)]
pub struct SarifOutput {
    pub version: String,
    pub runs: Vec<SarifRun>,
}

#[derive(Debug, Serialize)]
pub struct SarifRun {
    pub tool: SarifTool,
    pub results: Vec<SarifResult>,
    pub column_kind: String,
}

#[derive(Debug, Serialize)]
pub struct SarifTool {
    pub driver: SarifDriver,
}

#[derive(Debug, Serialize)]
pub struct SarifDriver {
    pub name: String,
    pub version: String,
    pub information_uri: String,
    pub rules: Vec<SarifRule>,
}

#[derive(Debug, Serialize)]
pub struct SarifRule {
    pub id: String,
    pub name: String,
    pub short_description: SarifMessage,
    pub full_description: SarifMessage,
    pub default_configuration: SarifConfig,
}

#[derive(Debug, Serialize)]
pub struct SarifConfig {
    pub level: String,
}

#[derive(Debug, Serialize)]
pub struct SarifMessage {
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct SarifResult {
    pub rule_id: String,
    pub level: String,
    pub message: SarifMessage,
    pub locations: Vec<SarifLocation>,
    pub related_locations: Option<Vec<SarifRelatedLocation>>,
    pub fixes: Option<Vec<SarifFix>>,
}

#[derive(Debug, Serialize)]
pub struct SarifLocation {
    pub physical_location: SarifPhysicalLocation,
}

#[derive(Debug, Serialize)]
pub struct SarifPhysicalLocation {
    pub artifact_location: SarifArtifactLocation,
    pub region: SarifRegion,
}

#[derive(Debug, Serialize)]
pub struct SarifArtifactLocation {
    pub uri: String,
}

#[derive(Debug, Serialize)]
pub struct SarifRegion {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Serialize)]
pub struct SarifRelatedLocation {
    pub id: usize,
    pub physical_location: SarifPhysicalLocation,
    pub message: SarifMessage,
}

#[derive(Debug, Serialize)]
pub struct SarifFix {
    pub description: SarifMessage,
    pub artifact_changes: Vec<SarifArtifactChange>,
}

#[derive(Debug, Serialize)]
pub struct SarifArtifactChange {
    pub artifact_location: SarifArtifactLocation,
    pub replacements: Vec<SarifReplacement>,
}

#[derive(Debug, Serialize)]
pub struct SarifReplacement {
    pub deleted_region: SarifRegion,
    pub inserted_content: SarifMessage,
}

/// Convert verification results to SARIF output.
pub fn to_sarif(
    results: Vec<(String, VerifyResult, Vec<VerifyDiagnostic>)>,
    config: &SolverConfig,
) -> String {
    let mut rules = Vec::new();
    let mut sarif_results = Vec::new();

    // Collect unique diagnostic codes for rules
    let mut seen_codes = std::collections::HashSet::new();
    for (_, _, diagnostics) in &results {
        for diag in diagnostics {
            if seen_codes.insert(diag.code.clone()) {
                rules.push(SarifRule {
                    id: diag.code.clone(),
                    name: diag.code.clone(),
                    short_description: SarifMessage {
                        text: diag.message.clone(),
                    },
                    full_description: SarifMessage {
                        text: diag.message.clone(),
                    },
                    default_configuration: SarifConfig {
                        level: match diag.severity {
                            DiagnosticSeverity::Error => "error".to_string(),
                            DiagnosticSeverity::Warning => "warning".to_string(),
                            DiagnosticSeverity::Info => "note".to_string(),
                            DiagnosticSeverity::Hint => "none".to_string(),
                        },
                    },
                });
            }
        }
    }

    for (file, result, diagnostics) in results {
        for diag in diagnostics {
            let level = match diag.severity {
                DiagnosticSeverity::Error => "error",
                DiagnosticSeverity::Warning => "warning",
                DiagnosticSeverity::Info => "note",
                DiagnosticSeverity::Hint => "none",
            };

            sarif_results.push(SarifResult {
                rule_id: diag.code.clone(),
                level: level.to_string(),
                message: SarifMessage {
                    text: diag.message.clone(),
                },
                locations: vec![SarifLocation {
                    physical_location: SarifPhysicalLocation {
                        artifact_location: SarifArtifactLocation { uri: file.clone() },
                        region: SarifRegion {
                            start_line: diag.span.line(),
                            start_column: diag.span.column(),
                            end_line: diag.span.end_line().unwrap_or(diag.span.line()),
                            end_column: diag.span.end_column().unwrap_or(diag.span.column()),
                        },
                    },
                }],
                related_locations: if diag.related.is_empty() {
                    None
                } else {
                    Some(
                        diag.related
                            .iter()
                            .enumerate()
                            .map(|(i, r)| SarifRelatedLocation {
                                id: i,
                                physical_location: SarifPhysicalLocation {
                                    artifact_location: SarifArtifactLocation {
                                        uri: r.span.file().to_string(),
                                    },
                                    region: SarifRegion {
                                        start_line: r.span.line(),
                                        start_column: r.span.column(),
                                        end_line: r.span.end_line().unwrap_or(r.span.line()),
                                        end_column: r.span.end_column().unwrap_or(r.span.column()),
                                    },
                                },
                                message: SarifMessage {
                                    text: r.message.clone(),
                                },
                            })
                            .collect(),
                    )
                },
                fixes: diag.fix.as_ref().map(|fix| {
                    vec![SarifFix {
                        description: SarifMessage {
                            text: fix.title.clone(),
                        },
                        artifact_changes: fix
                            .edits
                            .iter()
                            .map(|edit| SarifArtifactChange {
                                artifact_location: SarifArtifactLocation {
                                    uri: edit.span.file().to_string(),
                                },
                                replacements: vec![SarifReplacement {
                                    deleted_region: SarifRegion {
                                        start_line: edit.span.line(),
                                        start_column: edit.span.column(),
                                        end_line: edit.span.end_line().unwrap_or(edit.span.line()),
                                        end_column: edit
                                            .span
                                            .end_column()
                                            .unwrap_or(edit.span.column()),
                                    },
                                    inserted_content: SarifMessage {
                                        text: edit.new_text.clone(),
                                    },
                                }],
                            })
                            .collect(),
                    }]
                }),
            });
        }
    }

    let output = SarifOutput {
        version: "2.1.0".to_string(),
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name: "naso-verify".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    information_uri: "https://github.com/naso-lang/naso".to_string(),
                    rules,
                },
            },
            results: sarif_results,
            column_kind: "utf16".to_string(),
        }],
    };

    serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
}

//! Uncomputation diagnostics for Naso Language Server.
//! Converts compiler TypeError uncomputation failures to LSP Diagnostics.

use crate::compiler_bridge::CompilerBridge;
use crate::diagnostics::codes;
use naso_compiler::typecheck::error::TypeError;
use tower_lsp::lsp_types::*;

/// Convert compiler span to LSP Range
fn span_to_range(_compiler_bridge: &CompilerBridge, span: &naso_compiler::ast::Span) -> Range {
    Range::new(
        Position::new(span.line - 1, span.column - 1),
        Position::new(span.line - 1, span.column - 1 + (span.end - span.start)),
    )
}

/// Convert uncomputation-related TypeError to LSP Diagnostics
pub fn type_error_to_diagnostics(
    compiler_bridge: &CompilerBridge,
    error: &TypeError,
    document_url: &url::Url,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let (message, severity, code, related_ranges) = match error {
        TypeError::MissingUncompute { name, span } => {
            let message = format!(
                "missing uncomputation step for variable `{}` in reversible block",
                name
            );
            let severity = DiagnosticSeverity::ERROR;
            let code = codes::unc::MISSING_UNCOMPUTE.to_string();
            let related = vec![span_to_range(compiler_bridge, span)];
            (message, severity, code, Some(related))
        }
        TypeError::CyclicUncompute { name, span } => {
            let message = format!("cyclic uncomputation dependency involving `{}`", name);
            let severity = DiagnosticSeverity::ERROR;
            let code = codes::unc::CYCLIC_UNCOMPUTE.to_string();
            let related = vec![span_to_range(compiler_bridge, span)];
            (message, severity, code, Some(related))
        }
        TypeError::ImpureInReversible { operation, span } => {
            let message = format!(
                "impure operation in reversible block: `{}` (non-invertible temporary)",
                operation
            );
            let severity = DiagnosticSeverity::ERROR;
            let code = codes::unc::NON_INVERTIBLE_TEMP.to_string();
            let related = vec![span_to_range(compiler_bridge, span)];
            (message, severity, code, Some(related))
        }
        _ => {
            // Not an uncomputation error we handle here
            return Vec::new();
        }
    };

    let fallback_span = naso_compiler::ast::Span::new(0, 0, 0, 0);
    let mut diagnostic = Diagnostic::new_simple(
        span_to_range(
            compiler_bridge,
            &match error {
                TypeError::MissingUncompute { span, .. } => *span,
                TypeError::CyclicUncompute { span, .. } => *span,
                TypeError::ImpureInReversible { span, .. } => *span,
                _ => fallback_span,
            },
        ),
        message,
    );
    diagnostic.severity = Some(severity);
    diagnostic.code = Some(NumberOrString::String(code));
    diagnostic.related_information = related_ranges.map(|ranges| {
        ranges
            .into_iter()
            .map(|range| DiagnosticRelatedInformation {
                location: Location {
                    uri: document_url.clone(),
                    range,
                },
                message: String::new(),
            })
            .collect()
    });

    diagnostics.push(diagnostic);
    diagnostics
}

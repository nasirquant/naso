//! Erasure diagnostics for Naso Language Server.
//! Converts compiler TypeError erasure violations to LSP Diagnostics.

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

/// Convert erasure-related TypeError to LSP Diagnostics
pub fn type_error_to_diagnostics(
    compiler_bridge: &CompilerBridge,
    error: &TypeError,
    document_url: &url::Url,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let (message, severity, code, related_ranges) = match error {
        TypeError::ErasedVariableUsedAtRuntime { name, span } => {
            let message = format!(
                "erased variable `{}` used at runtime (quantity 0 variables cannot appear in runtime positions)",
                name
            );
            let severity = DiagnosticSeverity::ERROR;
            let code = codes::era::RETAINED_AT_RUNTIME.to_string();
            let related = vec![span_to_range(compiler_bridge, span)];
            (message, severity, code, Some(related))
        }
        _ => {
            // Not an erasure error we handle here
            return Vec::new();
        }
    };
    let mut diagnostic = Diagnostic::new_simple(
        span_to_range(
            compiler_bridge,
            &match error {
                TypeError::ErasedVariableUsedAtRuntime { span, .. } => *span,
                _ => naso_compiler::ast::Span::new(0, 0, 1, 1),
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

//! Linearity diagnostics for Naso Language Server.
//! Converts compiler TypeError linearity violations to LSP Diagnostics.

use tower_lsp::lsp_types::*;
use crate::compiler_bridge::CompilerBridge;
use naso_compiler::typecheck::error::TypeError;
use crate::diagnostics::codes;

/// Convert compiler span to LSP Range
fn span_to_range(_compiler_bridge: &CompilerBridge, span: &naso_compiler::ast::Span) -> Range {
    Range::new(
        Position::new(span.line - 1, span.column - 1),
        Position::new(span.line - 1, span.column - 1 + (span.end - span.start) as u32),
    )
}

/// Convert a TypeError to a Vec of LSP Diagnostics
pub fn type_error_to_diagnostics(
    compiler_bridge: &CompilerBridge,
    error: &TypeError,
    document_url: &url::Url,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    
    let (message, severity, code, related_ranges) = match error {
        TypeError::LinearVariableUsedTwice { name, first_use, second_use } => {
            let message = format!("linear variable `{}` used twice (first at {:?}, second at {:?})", name, first_use, second_use);
            let severity = DiagnosticSeverity::ERROR;
            let code = codes::lin::DOUBLE_USE.to_string();
            let mut related = Vec::new();
            related.push(span_to_range(compiler_bridge, first_use));
            related.push(span_to_range(compiler_bridge, second_use));
            (message, severity, code, Some(related))
        }
        TypeError::UnusedLinearVariable { name, defined_at } => {
            let message = format!("unused linear variable `{}` (defined at {:?})", name, defined_at);
            let severity = DiagnosticSeverity::WARNING;
            let code = codes::lin::UNUSED.to_string();
            let related = vec![span_to_range(compiler_bridge, defined_at)];
            (message, severity, code, Some(related))
        }
        TypeError::UseOfMovedValue { name, moved_at, used_at } => {
            let message = format!("use of moved value `{}` (moved at {:?}, used at {:?})", name, moved_at, used_at);
            let severity = DiagnosticSeverity::ERROR;
            let code = codes::lin::USE_OF_MOVED.to_string();
            let mut related = Vec::new();
            related.push(span_to_range(compiler_bridge, moved_at));
            related.push(span_to_range(compiler_bridge, used_at));
            (message, severity, code, Some(related))
        }
        TypeError::QuantityMismatch { expected, found, span } => {
            // Check if this is a linearity quantity mismatch
            if matches!(expected, naso_compiler::Quantity::One) || matches!(found, naso_compiler::Quantity::One) {
                let message = format!("quantity mismatch: expected `{expected}`, found `{found}`");
                let severity = DiagnosticSeverity::ERROR;
                let code = codes::lin::IMPLICIT_DROP.to_string(); // Treat as implicit drop for linearity
                let related = vec![span_to_range(compiler_bridge, span)];
                (message, severity, code, Some(related))
            } else {
                // Not a linearity error, return empty
                return Vec::new();
            }
        }
        // Other errors that might relate to linearity
        TypeError::InOutRequiresUnique { found_qty, span } => {
            if matches!(found_qty, naso_compiler::Quantity::One) {
                // This is actually an MVS error, handle in mvs module
                return Vec::new();
            }
            let message = format!("inout binding requires unique ownership (quantity 1), found `{found_qty}`");
            let severity = DiagnosticSeverity::ERROR;
            let code = codes::mvs::INOUT_REQUIRES_UNIQUE.to_string();
            let related = vec![span_to_range(compiler_bridge, span)];
            (message, severity, code, Some(related))
        }
        _ => {
            // Not a linearity error we handle here
            return Vec::new();
        }
    };
    
    let fallback_span = naso_compiler::ast::Span::new(0, 0, 0, 0);
        let mut diagnostic = Diagnostic::new_simple(
            span_to_range(compiler_bridge, &match error {
                TypeError::LinearVariableUsedTwice { first_use, .. } => *first_use,
                TypeError::UnusedLinearVariable { defined_at, .. } => *defined_at,
                TypeError::UseOfMovedValue { used_at, .. } => *used_at,
                TypeError::QuantityMismatch { span, .. } => *span,
                TypeError::InOutRequiresUnique { span, .. } => *span,
                _ => fallback_span,
            }),
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
                        range: range.clone(),
                    },
                    message: String::new(),
                })
                .collect()
        });
    diagnostics.push(diagnostic);
    diagnostics
}

// Helper to extract the primary span from various error types
fn get_primary_span(error: &TypeError) -> naso_compiler::ast::Span {
    match error {
        TypeError::LinearVariableUsedTwice { first_use, .. } => *first_use,
        TypeError::UnusedLinearVariable { defined_at, .. } => *defined_at,
        TypeError::UseOfMovedValue { used_at, .. } => *used_at,
        TypeError::QuantityMismatch { span, .. } => *span,
        TypeError::InOutRequiresUnique { span, .. } => *span,
        // For other errors, we don't handle them here
        _ => naso_compiler::ast::Span::new(0, 0, 0, 0),
    }
}
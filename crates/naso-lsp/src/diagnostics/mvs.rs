//! MVS (Mutable Value Semantics) diagnostics for Naso Language Server.
//! Converts compiler TypeError inout/aliasing violations to LSP Diagnostics.

use tower_lsp::lsp_types::*;
use crate::compiler_bridge::CompilerBridge;
use naso_compiler::typecheck::error::TypeError;
use crate::diagnostics::codes;

/// Convert compiler span to LSP Range
fn span_to_range(compiler_bridge: &CompilerBridge, span: &naso_compiler::ast::Span) -> Range {
    Range::new(
        Position::new(span.line - 1, span.column - 1),
        Position::new(span.line - 1, span.column - 1 + (span.end - span.start) as u32),
    )
}

/// Convert MVS-related TypeError to LSP Diagnostics
pub fn type_error_to_diagnostics(
    compiler_bridge: &CompilerBridge,
    error: &TypeError,
    document_url: &url::Url,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    
    let (message, severity, code, related_ranges) = match error {
        TypeError::InOutAliasing { var, existing_borrow, existing_span, new_span } => {
            let message = format!(
                "inout aliasing: `{}` aliases with existing inout borrow of `{}`",
                var, existing_borrow
            );
            let severity = DiagnosticSeverity::ERROR;
            let code = codes::mvs::INOUT_ALIASING.to_string();
            let mut related = Vec::new();
            related.push(span_to_range(compiler_bridge, existing_span));
            related.push(span_to_range(compiler_bridge, new_span));
            (message, severity, code, Some(related))
        }
        TypeError::InOutRequiresUnique { found_qty, span } => {
            let message = format!(
                "inout binding requires unique ownership (quantity 1), found `{found_qty}`"
            );
            let severity = DiagnosticSeverity::ERROR;
            let code = codes::mvs::INOUT_REQUIRES_UNIQUE.to_string();
            let related = vec![span_to_range(compiler_bridge, span)];
            (message, severity, code, Some(related))
        }
        // Note: InOutEscape would be a new error variant in TypeError
        // For now we handle what's available
        _ => {
            // Not an MVS error we handle here
            return Vec::new();
        }
    };
    
    let diagnostic = Diagnostic::new_simple(
        span_to_range(compiler_bridge, &match error {
            TypeError::InOutAliasing { new_span, .. } => new_span,
            TypeError::InOutRequiresUnique { span, .. } => span,
            _ => &naso_compiler::ast::Span::new(0, 0, 1, 1),
        }),
        message,
    )
    .with_severity(Some(severity))
    .with_code(Some(code.into()))
    .with_related_information(related_ranges.map(|ranges| {
        ranges
            .into_iter()
            .map(|range| DiagnosticRelatedInformation::new(range.clone(), None))
            .collect()
    }));
    
    diagnostics.push(diagnostic);
    diagnostics
}
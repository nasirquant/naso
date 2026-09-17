//! Diagnostics module - Unified interface for all diagnostic publishers

pub mod codes;
pub mod erasure;
pub mod linearity;
pub mod mvs;
pub mod quick_fixes;
pub mod uncomputation;

use tower_lsp::lsp_types::*;
use crate::compiler_bridge::CompilerBridge;
use naso_compiler::typecheck::error::TypeError;

/// Convert all compiler TypeErrors to LSP Diagnostics
pub fn convert_all_errors(
    compiler_bridge: &CompilerBridge,
    errors: &[TypeError],
    document_url: &url::Url,
) -> Vec<Diagnostic> {
    let mut all_diagnostics = Vec::new();
    
    for error in errors {
        // Try each diagnostic converter
        all_diagnostics.extend(linearity::type_error_to_diagnostics(
            compiler_bridge, error, document_url
        ));
        all_diagnostics.extend(erasure::type_error_to_diagnostics(
            compiler_bridge, error, document_url
        ));
        all_diagnostics.extend(mvs::type_error_to_diagnostics(
            compiler_bridge, error, document_url
        ));
        all_diagnostics.extend(uncomputation::type_error_to_diagnostics(
            compiler_bridge, error, document_url
        ));
    }
    
    all_diagnostics
}

/// Generate quick fixes for a list of diagnostics
pub fn generate_quick_fixes(
    compiler_bridge: &CompilerBridge,
    diagnostics: &[Diagnostic],
    document_content: &str,
    document_uri: &Url,
) -> Vec<CodeActionOrCommand> {
    let mut all_fixes = Vec::new();

    for diagnostic in diagnostics {
        all_fixes.extend(quick_fixes::quick_fix_for_diagnostic(
            compiler_bridge, diagnostic, document_content, document_uri
        ));
    }

    all_fixes
}
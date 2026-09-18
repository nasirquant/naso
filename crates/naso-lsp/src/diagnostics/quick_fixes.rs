//! Quick fixes (code actions) for Naso Language Server diagnostics.
//! Provides automatic fixes for common linearity, MVS, and uncomputation errors.

use crate::compiler_bridge::CompilerBridge;
use crate::diagnostics::codes;
use std::collections::HashMap;
use tower_lsp::lsp_types::*;

/// Generate quick fixes for a given diagnostic
#[allow(dead_code)]
pub fn quick_fix_for_diagnostic(
    _compiler_bridge: &CompilerBridge,
    diagnostic: &Diagnostic,
    _document_content: &str,
    document_uri: &Url,
) -> Vec<CodeActionOrCommand> {
    let mut fixes = Vec::new();

    // Extract the diagnostic code
    let code = match &diagnostic.code {
        Some(NumberOrString::String(code)) => code.as_str(),
        _ => return fixes,
    };

    // Helper to get document URI for WorkspaceEdit
    let uri = document_uri.clone();

    // Generate fixes based on diagnostic code
    match code {
        codes::lin::UNUSED => {
            // Fix: Remove unused [1] binding
            if let Some(range) = get_primary_range(diagnostic) {
                fixes.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Remove unused linear binding".to_string(),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![diagnostic.clone()]),
                    edit: Some(WorkspaceEdit {
                        changes: Some(HashMap::from_iter(vec![(
                            uri.clone(),
                            vec![TextEdit::new(range, String::new())],
                        )])),
                        ..Default::default()
                    }),
                    ..Default::default()
                }));
            }
        }
        codes::lin::IMPLICIT_DROP => {
            // Fix: Add explicit linear_free or uncomputation
            if let Some(range) = get_primary_range(diagnostic) {
                fixes.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Add explicit linear_free call".to_string(),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![diagnostic.clone()]),
                    edit: Some(WorkspaceEdit {
                        changes: Some(HashMap::from_iter(vec![(
                            uri.clone(),
                            vec![TextEdit::new(range, "linear_free(_);".to_string())],
                        )])),
                        ..Default::default()
                    }),
                    ..Default::default()
                }));
            }
        }
        codes::mvs::INOUT_ALIASING => {
            // Fix: Suggest removing one of the conflicting inout bindings
            if let Some(range) = get_primary_range(diagnostic) {
                fixes.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Remove conflicting inout binding".to_string(),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![diagnostic.clone()]),
                    edit: Some(WorkspaceEdit {
                        changes: Some(HashMap::from_iter(vec![(
                            uri.clone(),
                            vec![TextEdit::new(range, String::new())],
                        )])),
                        ..Default::default()
                    }),
                    ..Default::default()
                }));
            }
        }
        codes::unc::MISSING_UNCOMPUTE => {
            // Fix: Add uncomputation block or explicit uncomputation call
            if let Some(range) = get_primary_range(diagnostic) {
                fixes.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Add uncomputation block".to_string(),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![diagnostic.clone()]),
                    edit: Some(WorkspaceEdit {
                        changes: Some(HashMap::from_iter(vec![(
                            uri.clone(),
                            vec![TextEdit::new(
                                range,
                                "revert { /* uncomputation */ };".to_string(),
                            )],
                        )])),
                        ..Default::default()
                    }),
                    ..Default::default()
                }));
            }
        }
        _ => {}
    }

    fixes
}

/// Helper to get the primary range from a diagnostic (first related or the main range)
#[allow(dead_code)]
#[allow(clippy::unnecessary_lazy_evaluations)]
fn get_primary_range(diagnostic: &Diagnostic) -> Option<Range> {
    diagnostic
        .related_information
        .as_ref()
        .and_then(|related| related.first())
        .map(|info| info.location.range)
        .or_else(|| Some(diagnostic.range))
}

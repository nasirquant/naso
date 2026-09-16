//! Quick fixes (code actions) for Naso Language Server diagnostics.
//! Provides automatic fixes for common linearity, MVS, and uncomputation errors.

use tower_lsp::lsp_types::*;
use crate::compiler_bridge::CompilerBridge;
use std::collections::HashMap;
use crate::diagnostics::codes;

/// Generate quick fixes for a given diagnostic
pub fn quick_fix_for_diagnostic(
    compiler_bridge: &CompilerBridge,
    diagnostic: &Diagnostic,
    document_content: &str,
) -> Vec<CodeActionOrCommand> {
    let mut fixes = Vec::new();
    
    // Extract the diagnostic code
    let code = match &diagnostic.code {
        Some(tower_lsp::lsp_types::NumberOrString::String(code)) => code.as_str(),
        _ => return fixes,
    };

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
                            diagnostic.related.as_ref().and_then(|r| r.first())?
                                .unwrap_or(&diagnostic.range)
                                .clone(),
                            vec![TextEdit::new(range.clone(), String::new())],
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
                            range.clone(),
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
                            range.clone(),
                            vec![TextEdit::new(range.clone(), String::new())],
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
                            range.clone(),
                            vec![TextEdit::new(range, "revert { /* uncomputation */ };".to_string())],
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
fn get_primary_range(diagnostic: &Diagnostic) -> Option<Range> {
    diagnostic
        .related_information
        .as_ref()
        .and_then(|related| related.first())
        .map(|info| info.range.clone())
        .or_else(|| Some(diagnostic.range.clone()))
}
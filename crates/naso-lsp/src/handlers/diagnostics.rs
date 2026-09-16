//! Diagnostics Handler - Real-time diagnostic publishing

use tower_lsp::lsp_types::*;
use crate::NasoLanguageServer;

pub async fn handle_diagnostics(
    server: &NasoLanguageServer,
    params: DocumentDiagnosticParams,
) -> Result<DocumentDiagnosticReport, tower_lsp::jsonrpc::Error> {
    let uri = params.text_document.uri;
    
    tracing::debug!("Diagnostics request: {}", uri);
    
    // Get diagnostics from compiler bridge
    let diagnostics = server.compiler_bridge.analyze(&uri).await.unwrap_or_default();
    
    Ok(DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
        related_documents: None,
        full_document_diagnostic_report: FullDocumentDiagnosticReport {
            result_id: None,
            items: diagnostics,
        },
    }))
}
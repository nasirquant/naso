//! Completion Handler - Intelligent code completions

use crate::NasoLanguageServer;
use tower_lsp::lsp_types::*;

pub async fn handle_completion(
    server: &NasoLanguageServer,
    params: CompletionParams,
) -> Result<Option<CompletionResponse>, tower_lsp::jsonrpc::Error> {
    let uri = params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;

    tracing::debug!("Completion request: {} at {:?}", uri, position);

    let completions = server.compiler_bridge.get_completions(&uri, position).await;

    Ok(Some(CompletionResponse::Array(completions)))
}

//! Definition Handler - Go-to-definition support

use crate::NasoLanguageServer;
use tower_lsp::lsp_types::*;

pub async fn handle_definition(
    server: &NasoLanguageServer,
    params: GotoDefinitionParams,
) -> Result<Option<GotoDefinitionResponse>, tower_lsp::jsonrpc::Error> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    tracing::debug!("Definition request: {} at {:?}", uri, position);

    if let Some(location) = server.compiler_bridge.get_definition(&uri, position).await {
        return Ok(Some(GotoDefinitionResponse::Scalar(location)));
    }

    Ok(None)
}

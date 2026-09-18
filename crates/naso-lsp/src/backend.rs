//! Naso Language Server - Core Backend
//!
//! Implements the core LSP server state management and request routing.

use std::sync::Arc;

use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::compiler_bridge::CompilerBridge;
use crate::document_store::DocumentStore;

pub struct NasoLanguageServer {
    client: Client,
    pub document_store: Arc<DocumentStore>,
    pub compiler_bridge: Arc<CompilerBridge>,
    // Configuration settings
    #[allow(dead_code)]
    config: Arc<tokio::sync::RwLock<ServerConfig>>,
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
struct ServerConfig {
    diagnostics_severity: Option<DiagnosticSeverity>,
    completion_detail: bool,
    enable_incremental: bool,
}

impl NasoLanguageServer {
    pub fn new(client: Client) -> Self {
        let document_store = Arc::new(DocumentStore::new());
        let compiler_bridge = Arc::new(CompilerBridge::new(document_store.clone()));

        Self {
            client,
            document_store,
            compiler_bridge,
            config: Arc::new(tokio::sync::RwLock::new(ServerConfig::default())),
        }
    }

    /// Get the client for sending notifications
    #[allow(dead_code)]
    fn client(&self) -> &Client {
        &self.client
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for NasoLanguageServer {
    async fn initialize(
        &self,
        params: InitializeParams,
    ) -> Result<InitializeResult, tower_lsp::jsonrpc::Error> {
        tracing::info!("Initialize request received: {:?}", params.client_info);

        // Store client capabilities if needed
        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::INCREMENTAL,
            )),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(false),
                trigger_characters: Some(vec![".".to_string(), ":".to_string(), "[".to_string()]),
                work_done_progress_options: WorkDoneProgressOptions::default(),
                all_commit_characters: None,
                ..Default::default()
            }),
            definition_provider: Some(OneOf::Left(true)),
            diagnostic_provider: Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
                identifier: Some("naso-diagnostics".to_string()),
                inter_file_dependencies: true,
                workspace_diagnostics: true,
                work_done_progress_options: WorkDoneProgressOptions::default(),
            })),
            workspace: Some(WorkspaceServerCapabilities {
                workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                    supported: Some(true),
                    change_notifications: Some(OneOf::Left(true)),
                }),
                file_operations: None,
            }),
            ..Default::default()
        };

        Ok(InitializeResult {
            capabilities,
            server_info: Some(ServerInfo {
                name: "Naso Language Server".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        tracing::info!("Client initialized");

        // Notify client we're ready
        self.client
            .log_message(MessageType::INFO, "Naso Language Server ready")
            .await;
    }

    async fn shutdown(&self) -> Result<(), tower_lsp::jsonrpc::Error> {
        tracing::info!("Shutdown request received");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        tracing::debug!("Document opened: {}", params.text_document.uri);

        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        let version = params.text_document.version;

        // Store document
        self.document_store.open(uri.clone(), text, version);

        // Trigger analysis
        self.analyze_document(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        tracing::debug!("Document changed: {}", params.text_document.uri);

        let uri = params.text_document.uri.clone();
        let version = params.text_document.version;

        for change in params.content_changes {
            match change.range {
                Some(range) => {
                    self.document_store
                        .update_incremental(&uri, range, change.text, version);
                }
                None => {
                    self.document_store.update_full(&uri, change.text, version);
                }
            }
        }

        // Trigger incremental re-analysis
        self.analyze_document(&uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        tracing::debug!("Document closed: {}", params.text_document.uri);
        self.document_store.close(&params.text_document.uri);
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        tracing::debug!("Document saved: {}", params.text_document.uri);
        // Could trigger full re-analysis on save if needed
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>, tower_lsp::jsonrpc::Error> {
        crate::handlers::hover::handle_hover(self, params).await
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> Result<Option<CompletionResponse>, tower_lsp::jsonrpc::Error> {
        crate::handlers::completion::handle_completion(self, params).await
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>, tower_lsp::jsonrpc::Error> {
        crate::handlers::definition::handle_definition(self, params).await
    }

    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> Result<DocumentDiagnosticReportResult, tower_lsp::jsonrpc::Error> {
        let report = crate::handlers::diagnostics::handle_diagnostics(self, params).await?;
        Ok(DocumentDiagnosticReportResult::Report(report))
    }
}

impl NasoLanguageServer {
    async fn analyze_document(&self, uri: &Url) {
        if let Some(diagnostics) = self.compiler_bridge.analyze(uri).await {
            self.client
                .publish_diagnostics(uri.clone(), diagnostics, None)
                .await;
        }
    }
}

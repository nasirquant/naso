//! Naso Language Server - Main Entry Point
//!
//! This binary implements the Language Server Protocol (LSP) for the Naso programming language,
//! providing IDE support including hover, completion, diagnostics, and go-to-definition.

use std::sync::Arc;

use tokio::io::{stdin, stdout};
use tower_lsp::{LspService, Server};

use crate::backend::NasoLanguageServer;

mod backend;
mod handlers;
mod document_store;
mod compiler_bridge;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting Naso Language Server");

    // Create the LSP service with our backend
    let (service, socket) = LspService::new(|client| {
        NasoLanguageServer::new(client)
    });

    // Run the server on stdin/stdout
    Server::new(stdin(), stdout(), socket).serve(service).await;

    tracing::info!("Naso Language Server shutdown complete");
    Ok(())
}
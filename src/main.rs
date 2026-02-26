mod kraken;
mod server;
mod tools;

use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Log to stderr (stdout is reserved for MCP JSON-RPC)
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting tentactl server");

    let service = server::KrakenMcpServer::new();
    let server = service.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("Failed to start server: {e}");
    })?;

    server.waiting().await?;
    Ok(())
}

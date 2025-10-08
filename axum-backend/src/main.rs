/// Mechaway: Hyperminimalist intelligent systems automation engine
/// 
/// Main entry point for the Mechaway server. Initializes configuration and starts
/// the HTTP server with workflow management and execution capabilities.

use mechaway::{config::Config, server::start_server};

/// Application entry point
/// 
/// Initializes the server with default configuration and starts listening for requests.
/// The server provides:
/// - Workflow management API at /api/workflows/*
/// - Dynamic webhook execution at /webhook/{workflow_id}/*  
/// - Health check at /healthz
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration (defaults to localhost:3004 and SQLite databases)
    let config = Config::default();
    
    // Start the server
    start_server(config).await?;
    
    Ok(())
}
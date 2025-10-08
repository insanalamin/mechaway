/// Server setup and initialization
/// 
/// Wires together all components: storage, registry, execution engine, and HTTP routes.
/// Provides the main application factory function for creating the Axum app.

use crate::{
    api::{
        webhooks::{register_webhook_routes_for_workflows, WebhookAppState},
        workflows::{create_workflow_routes, AppState},
    },
    config::Config,
    project::ProjectDatabaseManager,
    runtime::{engine::ExecutionEngine, executor::NodeExecutor, scheduler::CronSchedulerService},
    workflow::{registry::WorkflowRegistry, storage::WorkflowStorage},
};
use anyhow::Result;
use axum::{
    routing::get,
    Router,
};
// No longer need direct SQLite imports - using ProjectDatabaseManager
use std::sync::Arc;
use tokio::net::TcpListener;

/// Create the main Axum application with all routes and middleware
/// 
/// Initializes all components and wires them together into a complete application.
/// This includes database connections, workflow registry, execution engine, and HTTP routes.
pub async fn create_app(config: Config) -> Result<Router> {
    // Ensure project data directory exists
    tracing::info!("📁 Ensuring project data directory exists: {}", config.database.project_data_dir);
    std::fs::create_dir_all(&config.database.project_data_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create project data directory: {}", e))?;

    // Initialize project database manager for isolated multi-tenant storage
    tracing::info!("🏗️ Initializing project database manager");
    let data_dir = config.database.project_data_dir.clone();
    tracing::debug!("📁 Project data directory: {}", data_dir);
    let project_db_manager = Arc::new(ProjectDatabaseManager::new(data_dir));
    
    // Initialize workflow storage using default project database
    tracing::info!("📋 Initializing workflow storage (default project)");
    let default_project_pool = project_db_manager.get_project_pool("default").await
        .map_err(|e| anyhow::anyhow!("Failed to get default project database: {}", e))?;
    let workflow_storage = WorkflowStorage::new(default_project_pool);

    // Initialize workflow registry and load existing workflows
    tracing::info!("📊 Initializing workflow registry");
    let workflow_registry = Arc::new(WorkflowRegistry::new(workflow_storage.clone()));
    
    tracing::info!("📥 Loading existing workflows from storage");
    workflow_registry.init_from_storage().await
        .map_err(|e| anyhow::anyhow!("Failed to load workflows from storage: {}", e))?;
    
    // Initialize execution components
    tracing::info!("⚙️ Initializing node executor with project isolation");
    let node_executor = NodeExecutor::new(Arc::clone(&project_db_manager))
        .map_err(|e| anyhow::anyhow!("Failed to initialize node executor: {}", e))?;
    
    tracing::info!("🚀 Initializing execution engine");
    let node_executor_arc = Arc::new(node_executor);
    let execution_engine = Arc::new(ExecutionEngine::new(Arc::clone(&node_executor_arc)));

    // Initialize cron scheduler service  
    tracing::info!("⏰ Initializing cron scheduler service");
    let cron_scheduler = Arc::new(
        CronSchedulerService::new(
            Arc::clone(&workflow_registry),
            Arc::clone(&node_executor_arc), 
            Arc::clone(&execution_engine)
        ).await
        .map_err(|e| anyhow::anyhow!("Failed to initialize cron scheduler: {}", e))?
    );

    // Start the cron scheduler in background
    tracing::info!("🚀 Starting cron scheduler");
    let scheduler_clone = Arc::clone(&cron_scheduler);
    tokio::spawn(async move {
        if let Err(e) = scheduler_clone.start().await {
            tracing::error!("❌ Failed to start cron scheduler: {}", e);
        }
    });

    // Create application states
    tracing::info!("🏗️ Creating application states");
    let app_state = AppState {
        storage: workflow_storage,
        registry: workflow_registry.clone(),
        scheduler: Arc::clone(&cron_scheduler),
    };

    let webhook_state = WebhookAppState {
        app_state: app_state.clone(),
        engine: execution_engine,
    };

    // Build webhook routes (dynamically registered based on active workflows)
    tracing::info!("🔗 Registering webhook routes");
    let webhook_routes = register_webhook_routes_for_workflows(&*workflow_registry).await;

    // Create the main application router
    tracing::info!("📡 Creating HTTP router with all endpoints");
    let app = Router::new()
        // Health check endpoint
        .route("/healthz", get(health_check))
        
        // Workflow management API routes
        .merge(create_workflow_routes().with_state(app_state))
        
        // Dynamic webhook execution routes  
        .merge(webhook_routes.with_state(webhook_state));

    tracing::info!("✅ Application initialized successfully");
    
    Ok(app)
}

/// Start the HTTP server with the given configuration
/// 
/// Creates the application and starts the Axum server on the configured address and port.
pub async fn start_server(config: Config) -> Result<()> {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(true)
        .with_level(true)
        .init();

    tracing::info!("Starting Mechaway server...");
    
    // Create the application
    let app = create_app(config.clone()).await?;

    // Bind to the configured address
    let bind_addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&bind_addr).await?;
    
    tracing::info!("Server listening on http://{}", bind_addr);

    // Start the server
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

/// Health check endpoint handler
/// 
/// Simple health check that returns "ok" - same as our original endpoint
async fn health_check() -> &'static str {
    "ok"
}

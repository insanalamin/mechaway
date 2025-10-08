/// Dynamic webhook execution endpoints
/// 
/// Handles webhook triggers that start workflow execution. Routes are registered
/// dynamically based on active workflows with WebhookNode definitions.

use crate::api::workflows::AppState;
use crate::runtime::engine::ExecutionEngine;
use crate::workflow::types::ExecutionContext;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{any, Router},
};
use serde_json::Value;
use std::sync::Arc;

/// Extended application state with execution engine
#[derive(Clone)]
pub struct WebhookAppState {
    /// Base app state with storage and registry
    pub app_state: AppState,
    /// Execution engine for running workflows
    pub engine: Arc<ExecutionEngine>,
}

/// Create webhook routes dynamically based on active workflows
/// 
/// This function should be called whenever workflows are updated to ensure
/// webhook routes stay in sync with active workflow definitions.
pub fn create_webhook_routes() -> Router<WebhookAppState> {
    Router::new()
        // Catch-all route for dynamic webhook paths
        // Format: /webhook/{workflow_id}/{webhook_path}
        .route("/webhook/{workflow_id}/{*path}", any(execute_webhook))
}

/// Execute a workflow via webhook trigger
/// 
/// POST/GET/PUT/DELETE /webhook/{workflow_id}/{webhook_path}
/// Body: JSON payload that becomes the initial execution context data
async fn execute_webhook(
    State(state): State<WebhookAppState>,
    Path((workflow_id, webhook_path)): Path<(String, String)>,
    body: String,
) -> Result<Json<Value>, StatusCode> {
    tracing::info!("📥 Webhook request received: {}/{}", workflow_id, webhook_path);
    tracing::debug!("📄 Request body: {}", body);
    
    // Parse JSON body manually to handle errors gracefully
    let payload: Value = match serde_json::from_str(&body) {
        Ok(json) => {
            tracing::debug!("✅ JSON payload parsed successfully");
            json
        },
        Err(e) => {
            tracing::warn!("❌ Invalid JSON payload for webhook: {}/{} - Error: {}", workflow_id, webhook_path, e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };
    
    // Get the compiled workflow from registry
    tracing::debug!("🔍 Looking up workflow in registry: {}", workflow_id);
    let compiled_workflow = match state.app_state.registry.get_workflow(&workflow_id) {
        Some(workflow) => {
            tracing::debug!("✅ Workflow found: {} ({})", workflow.workflow.id, workflow.workflow.name);
            workflow
        },
        None => {
            tracing::warn!("❌ Webhook called for unknown workflow: {}", workflow_id);
            return Err(StatusCode::NOT_FOUND);
        }
    };

    // Find the webhook node that matches this path
    let webhook_path_normalized = if webhook_path.starts_with('/') {
        webhook_path
    } else {
        format!("/{}", webhook_path)
    };
    
    tracing::debug!("🔍 Searching for webhook node with path: {}", webhook_path_normalized);
    let start_node_id = find_webhook_start_node(&compiled_workflow, &webhook_path_normalized)?;
    tracing::debug!("✅ Found start node: {}", start_node_id);

    // Create execution context from webhook payload
    tracing::debug!("📋 Creating execution context with payload");
    let execution_context = ExecutionContext::from_webhook_data(workflow_id.clone(), payload, "default".to_string());
    tracing::debug!("📊 Execution context created with {} metadata fields", 
        execution_context.metadata.len());

    // Execute the workflow starting from the webhook node
    tracing::info!("🚀 Starting workflow execution for: {} from node: {}", workflow_id, start_node_id);
    let workflow_start_time = std::time::Instant::now();
    
    match state.engine.execute_workflow(&compiled_workflow, &start_node_id, execution_context).await {
        Ok(result) => {
            let workflow_duration = workflow_start_time.elapsed();
            tracing::info!(
                "🎉 Workflow execution completed successfully: {} -> {} in {:?}",
                workflow_id,
                start_node_id,
                workflow_duration
            );
            tracing::debug!("📤 Final result data: {}", 
                serde_json::to_string(&result.data).unwrap_or_else(|_| "invalid_json".to_string()));
            Ok(Json(serde_json::Value::Array(result.data)))
        }
        Err(e) => {
            let workflow_duration = workflow_start_time.elapsed();
            tracing::error!(
                "❌ Workflow execution failed for {} after {:?} - Error: {}",
                workflow_id,
                workflow_duration,
                e
            );
            
            // Log the error chain for debugging
            let error_chain: Vec<String> = std::iter::successors(
                Some(e.as_ref() as &dyn std::error::Error),
                |err| err.source()
            ).skip(1) // Skip the root error (already logged above)
            .map(|err| err.to_string())
            .collect();
            
            if !error_chain.is_empty() {
                tracing::debug!("🔍 Error chain: {}", error_chain.join(" → "));
            }
            
            // Use 422 (Unprocessable Entity) for execution failures
            // vs 500 for system errors  
            Err(StatusCode::UNPROCESSABLE_ENTITY)
        }
    }
}

/// Find the webhook node that matches the requested path
/// 
/// Searches through the workflow nodes to find a WebhookNode with a matching path parameter.
/// Returns the node ID that should be used as the execution start point.
fn find_webhook_start_node(
    compiled_workflow: &crate::workflow::registry::CompiledWorkflow,
    webhook_path: &str,
) -> Result<String, StatusCode> {
    tracing::debug!("🔍 Searching for webhook node with path: '{}'", webhook_path);
    
    let mut webhook_nodes_found = Vec::new();
    
    for node in &compiled_workflow.workflow.nodes {
        if matches!(node.node_type, crate::workflow::NodeType::Webhook) {
            if let Some(path) = node.params.get("path").and_then(|p| p.as_str()) {
                webhook_nodes_found.push(format!("'{}' -> '{}'", node.id, path));
                tracing::debug!("  🔍 Checking webhook node '{}' with path: '{}'", node.id, path);
                if path == webhook_path {
                    tracing::debug!("✅ Found matching webhook node: '{}'", node.id);
                    return Ok(node.id.clone());
                }
            } else {
                tracing::debug!("  ⚠️ Webhook node '{}' has no path parameter", node.id);
                webhook_nodes_found.push(format!("'{}' -> [no path]", node.id));
            }
        }
    }

    tracing::warn!(
        "❌ No webhook node found for path '{}' in workflow '{}'. Available webhook nodes: [{}]",
        webhook_path,
        compiled_workflow.workflow.id,
        webhook_nodes_found.join(", ")
    );
    
    Err(StatusCode::NOT_FOUND)
}

/// Helper function to register webhook routes dynamically
/// 
/// This would be called whenever workflows are updated to rebuild the routing table.
/// For now, we use a catch-all approach, but this could be optimized to pre-register
/// specific routes for better performance.
pub async fn register_webhook_routes_for_workflows(
    registry: &crate::workflow::registry::WorkflowRegistry,
) -> Router<WebhookAppState> {
    let webhook_routes = registry.get_webhook_routes();
    
    tracing::info!(
        "Registered {} dynamic webhook routes",
        webhook_routes.len()
    );
    
    // For now, return the catch-all router
    // Future optimization: register specific routes for each webhook
    create_webhook_routes()
}

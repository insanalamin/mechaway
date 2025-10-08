/// Workflow management REST API endpoints
/// 
/// Provides CRUD operations for workflow definitions with hot-reload support.
/// All changes trigger immediate registry updates for zero-downtime deployments.

use crate::{
    workflow::{
        registry::WorkflowRegistry,
        storage::WorkflowStorage,
        types::Workflow,
    },
    runtime::scheduler::CronSchedulerService,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

/// Application state containing shared resources
#[derive(Clone)]
pub struct AppState {
    /// Workflow storage for persistence
    pub storage: WorkflowStorage,
    /// Hot-reload registry for in-memory workflows
    pub registry: Arc<WorkflowRegistry>,
    /// Cron scheduler service for background job management
    pub scheduler: Arc<CronSchedulerService>,
}

/// Response for workflow creation/update operations
#[derive(Debug, Serialize)]
pub struct WorkflowResponse {
    pub id: String,
    pub message: String,
}

/// Request body for workflow creation
#[derive(Debug, Deserialize)]
pub struct CreateWorkflowRequest {
    pub workflow: Workflow,
}

/// Create workflow management routes
/// 
/// Sets up the REST API endpoints for workflow CRUD operations.
/// All endpoints use the shared application state for storage and registry access.
pub fn create_workflow_routes() -> Router<AppState> {
    Router::new()
        .route("/api/workflows", post(create_workflow))
        .route("/api/workflows", get(list_workflows))
        .route("/api/workflows/{id}", get(get_workflow))
        .route("/api/workflows/{id}", put(update_workflow))
        .route("/api/workflows/{id}", delete(delete_workflow))
}

/// Create a new workflow
/// 
/// POST /api/workflows
/// Body: { "workflow": { "id": "...", "name": "...", "nodes": [...], "edges": [...] } }
async fn create_workflow(
    State(state): State<AppState>,
    Json(payload): Json<CreateWorkflowRequest>,
) -> Result<Json<WorkflowResponse>, StatusCode> {
    let workflow = payload.workflow;

    // Validate workflow structure
    if workflow.id.is_empty() || workflow.name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check if workflow already exists
    match state.storage.get_workflow(&workflow.id).await {
        Ok(Some(_)) => return Err(StatusCode::CONFLICT), // Workflow already exists
        Ok(None) => {} // Good, doesn't exist
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    }

    // Save to persistent storage
    if let Err(e) = state.storage.save_workflow(&workflow).await {
        tracing::error!("Failed to save workflow: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Hot-reload into registry
    if let Err(e) = state.registry.reload_workflow(&workflow.id).await {
        tracing::error!("Failed to reload workflow into registry: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // HOT-RELOAD: Register cron triggers with zero-downtime (Scalable pattern)
    if let Err(e) = state.scheduler.add_or_update_workflow_cron_triggers(&workflow).await {
        tracing::error!("Failed to register cron triggers for workflow {}: {}", workflow.id, e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    tracing::info!("🔥 Created workflow: {} ({}) with cron triggers", workflow.id, workflow.name);

    Ok(Json(WorkflowResponse {
        id: workflow.id.clone(),
        message: format!("Workflow '{}' created successfully", workflow.name),
    }))
}

/// List all workflows
/// 
/// GET /api/workflows
/// Returns: [{ "id": "...", "name": "...", "created_at": "...", "updated_at": "..." }]
async fn list_workflows(
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    match state.storage.list_workflows().await {
        Ok(workflows) => Ok(Json(json!({ "workflows": workflows }))),
        Err(e) => {
            tracing::error!("Failed to list workflows: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get a specific workflow by ID
/// 
/// GET /api/workflows/:id
/// Returns: { "id": "...", "name": "...", "nodes": [...], "edges": [...] }
async fn get_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Workflow>, StatusCode> {
    match state.storage.get_workflow(&id).await {
        Ok(Some(workflow)) => Ok(Json(workflow)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to get workflow {}: {}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Update an existing workflow
/// 
/// PUT /api/workflows/:id
/// Body: { "workflow": { "id": "...", "name": "...", "nodes": [...], "edges": [...] } }
async fn update_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<CreateWorkflowRequest>,
) -> Result<Json<WorkflowResponse>, StatusCode> {
    let mut workflow = payload.workflow;
    
    // Ensure the workflow ID matches the URL parameter
    workflow.id = id.clone();

    // Validate workflow structure
    if workflow.name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check if workflow exists
    match state.storage.get_workflow(&id).await {
        Ok(Some(_)) => {} // Good, exists
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    }

    // Save updated workflow to persistent storage
    if let Err(e) = state.storage.save_workflow(&workflow).await {
        tracing::error!("Failed to update workflow: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Hot-reload into registry
    if let Err(e) = state.registry.reload_workflow(&workflow.id).await {
        tracing::error!("Failed to reload updated workflow into registry: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // HOT-RELOAD: Update cron triggers with zero-downtime (Scalable pattern)
    if let Err(e) = state.scheduler.add_or_update_workflow_cron_triggers(&workflow).await {
        tracing::error!("Failed to hot-reload cron triggers for workflow {}: {}", workflow.id, e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    tracing::info!("🔥 Hot-reloaded workflow: {} ({}) with cron triggers", workflow.id, workflow.name);

    Ok(Json(WorkflowResponse {
        id: workflow.id.clone(),
        message: format!("Workflow '{}' updated successfully", workflow.name),
    }))
}

/// Delete a workflow
/// 
/// DELETE /api/workflows/:id
/// Returns: { "message": "Workflow deleted successfully" }
async fn delete_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    // HOT-RELOAD: Remove cron triggers first (Scalable pattern)
    state.scheduler.remove_workflow_cron_triggers(&id).await;

    // Remove from registry
    if let Err(e) = state.registry.remove_workflow(&id).await {
        tracing::error!("Failed to remove workflow from registry: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Remove from persistent storage
    match state.storage.delete_workflow(&id).await {
        Ok(true) => {
            tracing::info!("Deleted workflow: {} (cron jobs will gracefully skip execution)", id);
            
            // ✅ SCALABLE: No scheduler restart needed! 
            // Cron jobs use lifecycle management and will skip execution for deleted workflows
            // This approach scales to hundreds/thousands of workflows with zero downtime
            
            Ok(Json(json!({ "message": "Workflow deleted successfully" })))
        }
        Ok(false) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to delete workflow: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

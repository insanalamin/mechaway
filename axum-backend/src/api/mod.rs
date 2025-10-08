/// HTTP API Layer
/// 
/// This module provides the REST API endpoints for workflow management
/// and dynamic webhook execution. It handles:
/// - Workflow CRUD operations
/// - Dynamic webhook route registration  
/// - Execution triggering and response handling

// Workflow management endpoints (POST/GET/PUT/DELETE)
pub mod workflows;

// Dynamic webhook execution endpoints
pub mod webhooks;

// Re-export router builders
pub use workflows::create_workflow_routes;
pub use webhooks::create_webhook_routes;

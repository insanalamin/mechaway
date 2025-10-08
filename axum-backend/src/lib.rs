/// Mechaway: Hyperminimalist intelligent systems automation engine
/// 
/// This library provides the core workflow automation engine with hot-reload capabilities,
/// petgraph-based DAG execution, and extensible node system.

// Core configuration and setup
pub mod config;

// Project management layer - multi-tenant project isolation and database management
pub mod project;

// Workflow management layer - handles workflow definitions, storage, and registry
pub mod workflow;

// Runtime execution engine - petgraph DAG execution and node orchestration  
pub mod runtime;

// Logic engines are embedded directly in the runtime executor for simplicity

// HTTP API layer - REST endpoints for workflow management and webhook triggers
pub mod api;

// Server setup and initialization
pub mod server;

// Re-export commonly used types for external consumers
pub use project::{Project, ProjectDatabaseManager};
pub use workflow::{Workflow, Node, NodeType, Edge};
pub use runtime::ExecutionResult;
pub use server::start_server;

/// Workflow Management Layer
/// 
/// This module handles workflow definitions, persistence, and hot-reload registry.
/// It provides the core workflow management functionality including:
/// - Type definitions (Workflow, Node, Edge)  
/// - SQLite persistence with sqlx
/// - Lock-free hot-reload registry using ArcSwap

// Core workflow type definitions
pub mod types;

// SQLite persistence layer for workflow storage
pub mod storage;

// Hot-reload registry using ArcSwap for zero-downtime updates  
pub mod registry;

// Re-export commonly used types
pub use types::{Workflow, Node, NodeType, Edge, ExecutionContext};

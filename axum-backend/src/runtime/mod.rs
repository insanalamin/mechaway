/// Runtime Execution Engine
/// 
/// This module provides the petgraph-based DAG execution engine for workflows.
/// It handles:
/// - Converting workflows to petgraph DAGs
/// - Topological execution of nodes
/// - Async task orchestration with tokio
/// - Data flow between connected nodes

// Core execution engine using petgraph for DAG processing
pub mod engine;

// Individual node execution handlers
pub mod executor;

// Background cron scheduler service for CronTrigger nodes
pub mod scheduler;

// Re-export main types
pub use engine::ExecutionEngine;
pub use executor::ExecutionResult;
pub use scheduler::CronSchedulerService;

/// Hot-reload workflow registry using ArcSwap
/// 
/// Provides lock-free, atomic updates to the in-memory workflow registry.
/// Each workflow update swaps the entire registry pointer, ensuring zero-downtime
/// hot reloads while concurrent executions continue uninterrupted.

use crate::workflow::{storage::WorkflowStorage, types::Workflow};
use anyhow::Result;
use arc_swap::ArcSwap;
use std::{collections::HashMap, sync::Arc};

/// Lock-free workflow registry for hot-reload capabilities
/// 
/// Uses ArcSwap to provide atomic pointer swapping for the workflow map.
/// This allows instant updates without blocking concurrent workflow executions.
/// The registry is the single source of truth for active workflows in memory.
#[derive(Debug)]
pub struct WorkflowRegistry {
    /// Thread-safe atomic pointer to workflow map
    /// Key: workflow_id, Value: compiled workflow definition
    workflows: ArcSwap<HashMap<String, CompiledWorkflow>>,
    
    /// Reference to persistent storage for reload operations
    storage: WorkflowStorage,
}

/// Compiled workflow with execution metadata
/// 
/// Extends the base Workflow with runtime information needed for efficient execution.
/// This includes webhook paths for dynamic route registration and start node lookups.
#[derive(Debug, Clone)]
pub struct CompiledWorkflow {
    /// Base workflow definition
    pub workflow: Workflow,
    
    /// Extracted webhook paths for dynamic routing
    /// Format: ["/grade", "/payment"] - used to register Axum routes
    pub webhook_paths: Vec<String>,
    
    /// Node IDs that are entry points (WebhookNode or CronTrigger types)
    /// Used to start execution when webhook is triggered or cron schedule fires
    pub start_node_ids: Vec<String>,
}

impl WorkflowRegistry {
    /// Create new registry instance with storage backend
    pub fn new(storage: WorkflowStorage) -> Self {
        Self {
            workflows: ArcSwap::new(Arc::new(HashMap::new())),
            storage,
        }
    }

    /// Initialize registry by loading all workflows from storage
    /// 
    /// Called during application startup to populate the in-memory registry.
    /// Compiles each workflow and extracts execution metadata.
    pub async fn init_from_storage(&self) -> Result<()> {
        let stored_workflows = self.storage.load_all_workflows().await?;
        let compiled_workflows = self.compile_workflows(stored_workflows)?;
        
        // Atomic swap of the entire registry
        self.workflows.store(Arc::new(compiled_workflows));
        
        tracing::info!("Initialized workflow registry with {} workflows", 
            self.workflows.load().len());
        
        Ok(())
    }

    /// Hot-reload a single workflow
    /// 
    /// Updates or adds a workflow to the registry using atomic pointer swap.
    /// This operation is lock-free and doesn't block concurrent executions.
    pub async fn reload_workflow(&self, workflow_id: &str) -> Result<()> {
        // Load fresh workflow from storage
        let workflow = self.storage.get_workflow(workflow_id).await?
            .ok_or_else(|| anyhow::anyhow!("Workflow not found: {}", workflow_id))?;
        
        // Compile the workflow
        let compiled = self.compile_single_workflow(workflow)?;
        
        // Clone current registry and update it
        let current = self.workflows.load();
        let mut new_registry = (**current).clone();
        new_registry.insert(workflow_id.to_string(), compiled);
        
        // Atomic swap to new registry
        self.workflows.store(Arc::new(new_registry));
        
        tracing::info!("Hot-reloaded workflow: {}", workflow_id);
        
        Ok(())
    }

    /// Get a workflow by ID (lock-free read)
    /// 
    /// Returns a cloned CompiledWorkflow for execution. The clone is cheap since
    /// the underlying data is Arc-wrapped, so only reference counts are incremented.
    pub fn get_workflow(&self, workflow_id: &str) -> Option<CompiledWorkflow> {
        self.workflows.load().get(workflow_id).cloned()
    }

    /// Get all workflows for processing (used by scheduler)
    pub fn get_all_workflows(&self) -> Vec<Workflow> {
        self.workflows.load()
            .values()
            .map(|compiled| compiled.workflow.clone())
            .collect()
    }

    /// List all active workflow IDs
    pub fn list_workflow_ids(&self) -> Vec<String> {
        self.workflows.load().keys().cloned().collect()
    }

    /// Get all webhook paths for dynamic route registration
    /// 
    /// Returns a map of webhook_path -> workflow_id for Axum route setup.
    /// Used by the API layer to create dynamic webhook endpoints.
    pub fn get_webhook_routes(&self) -> HashMap<String, String> {
        let workflows = self.workflows.load();
        let mut routes = HashMap::new();
        
        for (workflow_id, compiled) in workflows.iter() {
            for path in &compiled.webhook_paths {
                routes.insert(path.clone(), workflow_id.clone());
            }
        }
        
        routes
    }

    /// Remove a workflow from registry
    pub async fn remove_workflow(&self, workflow_id: &str) -> Result<()> {
        let current = self.workflows.load();
        let mut new_registry = (**current).clone();
        
        if new_registry.remove(workflow_id).is_some() {
            self.workflows.store(Arc::new(new_registry));
            tracing::info!("Removed workflow from registry: {}", workflow_id);
        }
        
        Ok(())
    }

    /// Compile multiple workflows into execution-ready format
    fn compile_workflows(&self, workflows: HashMap<String, Workflow>) -> Result<HashMap<String, CompiledWorkflow>> {
        let mut compiled = HashMap::new();
        
        for (id, workflow) in workflows {
            let compiled_workflow = self.compile_single_workflow(workflow)?;
            compiled.insert(id, compiled_workflow);
        }
        
        Ok(compiled)
    }

    /// Compile a single workflow and extract execution metadata
    /// 
    /// Analyzes the workflow to extract:
    /// - Webhook paths from WebhookNode params
    /// - Start node IDs (nodes with WebhookNode or CronTrigger type)
    /// - Validation of node structure
    fn compile_single_workflow(&self, workflow: Workflow) -> Result<CompiledWorkflow> {
        let mut webhook_paths = Vec::new();
        let mut start_node_ids = Vec::new();
        
        // Extract metadata from nodes
        for node in &workflow.nodes {
            match node.node_type {
                crate::workflow::NodeType::Webhook => {
                    start_node_ids.push(node.id.clone());
                    
                    // Extract webhook path from params
                    if let Some(path) = node.params.get("path").and_then(|p| p.as_str()) {
                        webhook_paths.push(path.to_string());
                    }
                }
                crate::workflow::NodeType::CronTrigger => {
                    start_node_ids.push(node.id.clone());
                    // CronTrigger nodes are also valid start nodes (background triggers)
                }
                _ => {}
            }
        }
        
        // Validate that workflow has at least one start node (Webhook or CronTrigger)
        if start_node_ids.is_empty() {
            return Err(anyhow::anyhow!("Workflow must have at least one start node (WebhookNode or CronTrigger)"));
        }
        
        Ok(CompiledWorkflow {
            workflow,
            webhook_paths,
            start_node_ids,
        })
    }
}

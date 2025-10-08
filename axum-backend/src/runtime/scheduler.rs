/// Background cron scheduler service
/// 
/// Manages scheduled workflows using tokio-cron-scheduler. Automatically
/// registers CronTrigger nodes from workflows and executes them at scheduled times.

use crate::{
    runtime::{engine::ExecutionEngine, executor::NodeExecutor},
    workflow::{
        types::{ExecutionContext, Node, NodeType, Workflow},
        registry::WorkflowRegistry,
    },
};
use anyhow::Result;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{oneshot, RwLock};
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

/// Industrial-grade hot-reload cron scheduler service
/// 
/// Uses Scalable pattern for zero-downtime job updates with cancellation map.
/// Scales to thousands of workflows with instant schedule changes.
pub struct CronSchedulerService {
    scheduler: Arc<RwLock<JobScheduler>>,
    job_uuid_map: Arc<RwLock<HashMap<String, Uuid>>>, // Track job UUIDs for proper removal
    registry: Arc<WorkflowRegistry>,
    executor: Arc<NodeExecutor>,
    engine: Arc<ExecutionEngine>,
}

impl CronSchedulerService {
    /// Create new hot-reload cron scheduler service
    pub async fn new(
        registry: Arc<WorkflowRegistry>,
        executor: Arc<NodeExecutor>,
        engine: Arc<ExecutionEngine>,
    ) -> Result<Self> {
        let scheduler = JobScheduler::new().await?;
        
        Ok(Self {
            scheduler: Arc::new(RwLock::new(scheduler)),
            job_uuid_map: Arc::new(RwLock::new(HashMap::new())),
            registry,
            executor,
            engine,
        })
    }

    /// Start the hot-reload scheduler and register all cron triggers from workflows
    pub async fn start(&self) -> Result<()> {
        tracing::info!("⏰ Starting hot-reload cron scheduler service");
        
        // Register existing workflows using hot-reload pattern
        self.register_all_cron_triggers().await?;
        
        // Start the scheduler
        {
            let scheduler = self.scheduler.read().await;
            scheduler.start().await?;
        }
        
        tracing::info!("✅ Hot-reload cron scheduler started successfully");
        Ok(())
    }

    /// Stop the hot-reload scheduler
    pub async fn stop(&self) -> Result<()> {
        tracing::info!("⏹️ Stopping hot-reload cron scheduler service");
        
        // Clear job UUID map
        {
            let mut job_uuid_map = self.job_uuid_map.write().await;
            job_uuid_map.clear();
            tracing::debug!("🧹 Cleared job UUID map during shutdown");
        }
        
        // Shutdown scheduler
        {
            let mut scheduler = self.scheduler.write().await;
            scheduler.shutdown().await?;
        }
        
        tracing::info!("✅ Hot-reload cron scheduler stopped");
        Ok(())
    }

    /// DEPRECATED: Restart scheduler (not needed with hot-reload pattern)
    /// Hot-reload pattern eliminates the need for scheduler restarts!
    #[deprecated(note = "Use hot-reload pattern instead - no restart needed")]
    pub async fn restart_scheduler(&self) -> Result<()> {
        tracing::warn!("⚠️ restart_scheduler() is deprecated - hot-reload pattern eliminates need for restarts");
        Ok(())
    }

    /// HOT-RELOAD: Add or update workflow cron triggers (scalable pattern)
    /// This is the industrial-grade solution for zero-downtime updates
    pub async fn add_or_update_workflow_cron_triggers(&self, workflow: &Workflow) -> Result<()> {
        tracing::info!("🔥 Hot-reloading cron triggers for workflow: {}", workflow.id);
        
        let cron_nodes: Vec<&Node> = workflow.nodes.iter()
            .filter(|node| matches!(node.node_type, NodeType::CronTrigger))
            .collect();

        if cron_nodes.is_empty() {
            tracing::debug!("📋 No cron triggers found in workflow: {}", workflow.id);
            // Remove any existing triggers for this workflow
            self.remove_workflow_cron_triggers(&workflow.id).await;
            return Ok(());
        }

        let cron_count = cron_nodes.len();
        for cron_node in cron_nodes {
            self.add_or_update_cron_job(&workflow.id, cron_node).await?;
        }

        tracing::info!("🔥 Hot-reloaded {} cron triggers for workflow: {}", 
            cron_count, workflow.id);
        Ok(())
    }

    /// HOT-RELOAD: Remove all cron triggers for a workflow
    pub async fn remove_workflow_cron_triggers(&self, workflow_id: &str) {
        tracing::info!("🗑️ Removing all cron triggers for workflow: {}", workflow_id);
        
        // Remove all jobs for this workflow from tokio-cron-scheduler
        let mut job_uuid_map = self.job_uuid_map.write().await;
        let keys_to_remove: Vec<String> = job_uuid_map.keys()
            .filter(|key| key.starts_with(workflow_id))
            .cloned()
            .collect();
        
        for key in keys_to_remove {
            if let Some(job_uuid) = job_uuid_map.remove(&key) {
                // CRITICAL FIX: Actually remove the job from tokio-cron-scheduler
                let scheduler = self.scheduler.read().await;
                if let Err(e) = scheduler.remove(&job_uuid).await {
                    tracing::warn!("⚠️ Failed to remove job {} from scheduler: {}", key, e);
                } else {
                    tracing::debug!("🛑 Removed cron job from scheduler: {}", key);
                }
            }
        }
        
        tracing::info!("✅ Removed all cron triggers for workflow: {}", workflow_id);
    }

    /// HOT-RELOAD: Core add/update job function (scalable industrial pattern)
    async fn add_or_update_cron_job(&self, workflow_id: &str, cron_node: &Node) -> Result<()> {
        let schedule = cron_node.params.get("schedule")
            .and_then(|s| s.as_str())
            .ok_or_else(|| anyhow::anyhow!("CronTrigger missing 'schedule' parameter"))?;

        let job_id = format!("{}:{}", workflow_id, cron_node.id);
        
        tracing::info!("🔥 Hot-reloading cron job: {} - schedule: {}", job_id, schedule);

        // STEP 1: CRITICAL FIX - Remove existing job from tokio-cron-scheduler
        {
            let mut job_uuid_map = self.job_uuid_map.write().await;
            if let Some(old_job_uuid) = job_uuid_map.remove(&job_id) {
                // Actually remove the old job from the scheduler
                let scheduler = self.scheduler.read().await;
                if let Err(e) = scheduler.remove(&old_job_uuid).await {
                    tracing::warn!("⚠️ Failed to remove old job {} from scheduler: {}", job_id, e);
                } else {
                    tracing::debug!("🛑 Removed old cron job from scheduler: {}", job_id);
                }
            }
        }

        // STEP 2: Clone data for the job closure
        let workflow_id_owned = workflow_id.to_string();
        let cron_node_id = cron_node.id.clone();
        let registry = Arc::clone(&self.registry);
        let engine = Arc::clone(&self.engine);

        // STEP 3: Create the hot-reloadable job (simplified without oneshot for now)
        let job = Job::new_async(schedule, move |_uuid, _l| {
            let workflow_id = workflow_id_owned.clone();
            let cron_node_id = cron_node_id.clone();
            let registry = Arc::clone(&registry);
            let engine = Arc::clone(&engine);

            Box::pin(async move {
                tracing::debug!("🔔 Cron trigger activated: {} in workflow {}", cron_node_id, workflow_id);
                
                // Check if workflow still exists (scalable pattern)
                if let Some(workflow) = registry.get_workflow(&workflow_id) {
                    tracing::info!("🚀 Executing cron workflow: {}", workflow_id);
                    
                    // Create execution context from cron trigger
                    let context = ExecutionContext::from_cron_trigger(workflow_id.clone(), cron_node_id.clone(), "default".to_string());
                    
                    // Execute the workflow starting from the cron trigger
                    match engine.execute_workflow(&workflow, &cron_node_id, context).await {
                        Ok(result) => {
                            tracing::info!("✅ Cron-triggered workflow completed: {} (continue: {})", 
                                workflow_id, result.should_continue);
                        }
                        Err(e) => {
                            tracing::error!("❌ Cron-triggered workflow failed: {} - Error: {}", 
                                workflow_id, e);
                        }
                    }
                } else {
                    // Workflow was deleted - job gracefully skips execution
                    tracing::debug!("⏭️ Skipping cron trigger for deleted workflow: {}", workflow_id);
                }
            })
        })?;

        // STEP 4: Add job to scheduler and get UUID
        let new_job_uuid = {
            let scheduler = self.scheduler.write().await;
            scheduler.add(job).await?
        };

        // STEP 5: CRITICAL FIX - Track job UUID for proper removal
        {
            let mut job_uuid_map = self.job_uuid_map.write().await;
            job_uuid_map.insert(job_id.clone(), new_job_uuid);
            tracing::debug!("📝 Tracked job UUID for: {}", job_id);
        }

        tracing::info!("🔥 Hot-reloaded cron job: {} ({})", job_id, schedule);
        Ok(())
    }

    /// Register all cron triggers from all workflows (startup only)
    async fn register_all_cron_triggers(&self) -> Result<()> {
        let workflows = self.registry.get_all_workflows();
        let workflow_count = workflows.len();
        let mut total_triggers = 0;
        
        for workflow in workflows {
            let trigger_count = workflow.nodes.iter()
                .filter(|node| matches!(node.node_type, NodeType::CronTrigger))
                .count();
            
            if trigger_count > 0 {
                self.add_or_update_workflow_cron_triggers(&workflow).await?;
                total_triggers += trigger_count;
            }
        }

        tracing::info!("📊 Registered {} total cron triggers from {} workflows", 
            total_triggers, workflow_count);
        Ok(())
    }

    /// Register a single cron trigger node
    async fn register_cron_trigger(&self, workflow_id: &str, cron_node: &Node) -> Result<()> {
        let schedule = cron_node.params.get("schedule")
            .and_then(|s| s.as_str())
            .ok_or_else(|| anyhow::anyhow!("CronTrigger missing 'schedule' parameter"))?;

        let timezone = cron_node.params.get("timezone")
            .and_then(|tz| tz.as_str())
            .unwrap_or("UTC");

        tracing::debug!("⏰ Registering cron job: {} - schedule: {} ({})", 
            cron_node.id, schedule, timezone);

        // Clone data for the job closure
        let workflow_id_owned = workflow_id.to_string();
        let cron_node_id = cron_node.id.clone();
        let registry = Arc::clone(&self.registry);
        let engine = Arc::clone(&self.engine);

        // Create the cron job with lifecycle management (scales to hundreds of workflows!)
        let job = Job::new_async(schedule, move |_uuid, _l| {
            let workflow_id = workflow_id_owned.clone();
            let cron_node_id = cron_node_id.clone();
            let registry = Arc::clone(&registry);
            let engine = Arc::clone(&engine);

            Box::pin(async move {
                tracing::debug!("🔔 Cron trigger activated: {} in workflow {}", cron_node_id, workflow_id);
                
                // ✅ SCALABLE: Check if workflow still exists (zero downtime for other workflows)
                if let Some(workflow) = registry.get_workflow(&workflow_id) {
                    tracing::info!("🚀 Executing cron workflow: {}", workflow_id);
                    
                    // Create execution context from cron trigger
                    let context = ExecutionContext::from_cron_trigger(workflow_id.clone(), cron_node_id.clone(), "default".to_string());
                    
                    // Execute the workflow starting from the cron trigger
                    match engine.execute_workflow(&workflow, &cron_node_id, context).await {
                        Ok(result) => {
                            tracing::info!("✅ Cron-triggered workflow completed: {} (continue: {})", 
                                workflow_id, result.should_continue);
                        }
                        Err(e) => {
                            tracing::error!("❌ Cron-triggered workflow failed: {} - Error: {}", 
                                workflow_id, e);
                        }
                    }
                } else {
                    // Workflow was deleted - job gracefully skips execution (no restart needed!)
                    tracing::debug!("⏭️ Skipping cron trigger for deleted workflow: {}", workflow_id);
                }
            })
        })?;

        // Add job to scheduler with write lock
        {
            let scheduler = self.scheduler.write().await;
            scheduler.add(job).await?;
        }

        tracing::debug!("✅ Cron job registered: {} ({}) for workflow: {}", 
            cron_node.id, schedule, workflow_id);
        Ok(())
    }
}

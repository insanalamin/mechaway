/// Petgraph-based DAG execution engine
/// 
/// Converts workflows into directed acyclic graphs (DAGs) and executes them
/// using topological sorting for deterministic, parallel execution.

use crate::runtime::executor::{ExecutionResult, NodeExecutor};
use crate::workflow::registry::CompiledWorkflow;
use crate::workflow::types::{ExecutionContext, Node};
use anyhow::Result;
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use std::{collections::HashMap, sync::Arc};

/// DAG execution engine using petgraph for workflow orchestration
/// 
/// Builds a directed graph from workflow definitions and executes nodes
/// in topological order, respecting dependencies and data flow.
#[derive(Debug)]
pub struct ExecutionEngine {
    /// Node executor for handling individual node execution
    executor: Arc<NodeExecutor>,
}

/// Internal representation of a workflow as a petgraph DAG
#[derive(Debug)]
struct WorkflowGraph {
    /// The petgraph DiGraph structure
    graph: DiGraph<Node, ()>,
    /// Mapping from node ID to graph node index
    node_id_to_index: HashMap<String, NodeIndex>,
    /// Mapping from graph node index to node ID
    index_to_node_id: HashMap<NodeIndex, String>,
}

impl ExecutionEngine {
    /// Create new execution engine with node executor
    pub fn new(executor: Arc<NodeExecutor>) -> Self {
        Self { executor }
    }
    
    /// Find all nodes reachable from the starting node using DFS
    fn find_reachable_nodes(&self, graph: &petgraph::Graph<Node, ()>, start_index: petgraph::graph::NodeIndex) -> std::collections::HashSet<petgraph::graph::NodeIndex> {
        use std::collections::{HashSet, VecDeque};
        
        let mut reachable = HashSet::new();
        let mut queue = VecDeque::new();
        
        queue.push_back(start_index);
        reachable.insert(start_index);
        
        while let Some(current) = queue.pop_front() {
            // Add all neighbors (nodes this one points to)
            let mut neighbors = graph.neighbors(current).detach();
            while let Some(target) = neighbors.next_node(&graph) {
                if !reachable.contains(&target) {
                    reachable.insert(target);
                    queue.push_back(target);
                }
            }
        }
        
        reachable
    }

    /// Execute a workflow starting from a webhook trigger
    /// 
    /// Takes the compiled workflow and initial execution context,
    /// builds a DAG, and executes nodes in topological order.
    /// Returns the final execution result after all nodes complete.
    pub async fn execute_workflow(
        &self,
        workflow: &CompiledWorkflow,
        start_node_id: &str,
        mut context: ExecutionContext,
    ) -> Result<ExecutionResult> {
        let workflow_start_time = std::time::Instant::now();
        
        tracing::info!("🚀 Starting workflow execution: {} from node: {}", 
            workflow.workflow.id, start_node_id);
        
        // Build petgraph DAG from workflow definition
        tracing::debug!("📊 Building workflow DAG with {} nodes and {} edges", 
            workflow.workflow.nodes.len(), workflow.workflow.edges.len());
        let graph = self.build_workflow_graph(&workflow.workflow)?;
        
        // Find the start node index
        let start_index = graph.node_id_to_index.get(start_node_id)
            .ok_or_else(|| anyhow::anyhow!("Start node not found: {}", start_node_id))?;

        // Get topological order for execution
        tracing::debug!("🔄 Computing topological execution order");
        let topo_order = toposort(&graph.graph, None)
            .map_err(|_| anyhow::anyhow!("Workflow contains cycles - must be a DAG"))?;
        
        let unknown_name = "unknown".to_string();
        let node_order: Vec<String> = topo_order.iter()
            .map(|&idx| graph.index_to_node_id.get(&idx).unwrap_or(&unknown_name).clone())
            .collect();
        tracing::debug!("📋 Execution order: {:?}", node_order);

        // Find position of start node in topological order  
        let start_pos = topo_order.iter().position(|&idx| idx == *start_index)
            .ok_or_else(|| anyhow::anyhow!("Start node not found in topological order"))?;
        
        // If the start node is a WebhookNode or CronTrigger, we need to start from the next position
        // because these are just entry points and don't process data
        let execution_start_pos = if matches!(graph.graph[*start_index].node_type, 
                crate::workflow::NodeType::Webhook | crate::workflow::NodeType::CronTrigger) {
            tracing::debug!("🎯 Start node is WebhookNode/CronTrigger, beginning execution from next connected node");
            let next_pos = start_pos + 1;
            if next_pos >= topo_order.len() {
                return Err(anyhow::anyhow!("Start node has no connected processing nodes"));
            }
            next_pos
        } else {
            start_pos
        };
        
        tracing::debug!("🎯 Starting execution from position {} in topology", execution_start_pos);

        // Find nodes reachable from the start node (not all nodes!)
        let reachable_nodes = self.find_reachable_nodes(&graph.graph, *start_index);
        
        // Filter topological order to only include reachable nodes (excluding webhooks and cron triggers)
        let nodes_to_execute: Vec<petgraph::graph::NodeIndex> = topo_order.iter()
            .filter(|&&idx| reachable_nodes.contains(&idx) && 
                   !matches!(graph.graph[idx].node_type, 
                           crate::workflow::NodeType::Webhook | crate::workflow::NodeType::CronTrigger))
            .cloned()
            .collect();
            
        tracing::info!("🔄 Executing {} nodes reachable from {}", nodes_to_execute.len(), start_node_id);
        
        // Execute the filtered nodes
        let mut current_result = ExecutionResult {
            data: context.data.clone(),
            metadata: context.metadata.clone(),
            should_continue: true,
        };

        for (step_num, &node_index) in nodes_to_execute.iter().enumerate() {
            if !current_result.should_continue {
                tracing::warn!("⏸️ Workflow execution stopped at step {} - should_continue = false", step_num);
                break;
            }

            let node = &graph.graph[node_index];
            let unknown_name = "unknown".to_string();
            let node_name = graph.index_to_node_id.get(&node_index).unwrap_or(&unknown_name);
            
            tracing::info!("📍 Step {}/{}: Executing node '{}' (type: {:?})", 
                step_num + 1, nodes_to_execute.len(), node_name, node.node_type);
            
            // Update execution context with current result
            context.data = current_result.data.clone();
            context.metadata = current_result.metadata.clone();
            
            // Skip any remaining webhook nodes during execution (they shouldn't be in processing flow)
            if matches!(node.node_type, crate::workflow::NodeType::Webhook) {
                tracing::debug!("⏭️ Skipping webhook node '{}' during execution", node_name);
                continue;
            }

            // Execute the current node
            let node_start_time = std::time::Instant::now();
            
            current_result = self.executor.execute_node(node, context.clone()).await
                .map_err(|e| anyhow::anyhow!("Node execution failed for '{}': {}", node.id, e))?;
            
            let node_duration = node_start_time.elapsed();
            tracing::info!("✅ Node '{}' completed in {:?}", node_name, node_duration);
        }
        
        let workflow_duration = workflow_start_time.elapsed();
        tracing::info!("🎉 Workflow '{}' execution completed successfully in {:?}", 
            workflow.workflow.id, workflow_duration);

        Ok(current_result)
    }

    /// Build a petgraph DiGraph from workflow definition
    /// 
    /// Creates nodes and edges in the graph while maintaining bidirectional
    /// mappings between node IDs and graph indices for efficient lookups.
    fn build_workflow_graph(&self, workflow: &crate::workflow::Workflow) -> Result<WorkflowGraph> {
        tracing::debug!("🏗️ Building workflow graph for '{}'", workflow.id);
        
        let mut graph = DiGraph::new();
        let mut node_id_to_index = HashMap::new();
        let mut index_to_node_id = HashMap::new();

        // Add all nodes to the graph
        tracing::debug!("📦 Adding {} nodes to graph", workflow.nodes.len());
        for node in &workflow.nodes {
            let node_index = graph.add_node(node.clone());
            node_id_to_index.insert(node.id.clone(), node_index);
            index_to_node_id.insert(node_index, node.id.clone());
            tracing::debug!("  ➕ Added node: '{}' (type: {:?})", node.id, node.node_type);
        }

        // Add all edges to the graph
        tracing::debug!("🔗 Adding {} edges to graph", workflow.edges.len());
        for edge in &workflow.edges {
            let from_index = node_id_to_index.get(&edge.from)
                .ok_or_else(|| anyhow::anyhow!("Edge references unknown node: {}", edge.from))?;
            let to_index = node_id_to_index.get(&edge.to)
                .ok_or_else(|| anyhow::anyhow!("Edge references unknown node: {}", edge.to))?;
            
            graph.add_edge(*from_index, *to_index, ());
            tracing::debug!("  🔗 Added edge: '{}' → '{}'", edge.from, edge.to);
        }

        // Validate that the graph is a DAG (no cycles)
        tracing::debug!("🔍 Validating DAG structure (checking for cycles)");
        if let Err(_) = toposort(&graph, None) {
            tracing::error!("❌ Workflow contains cycles - must be a DAG");
            return Err(anyhow::anyhow!("Workflow contains cycles - must be a DAG"));
        }
        
        tracing::debug!("✅ DAG validation successful - no cycles detected");

        Ok(WorkflowGraph {
            graph,
            node_id_to_index,
            index_to_node_id,
        })
    }
}

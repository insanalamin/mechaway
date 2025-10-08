/// Core workflow type definitions
/// 
/// Defines the fundamental structures for workflows, nodes, and edges as specified
/// in the README. These types are serialized/deserialized from JSON for persistence.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A complete workflow definition containing nodes and their connections
/// 
/// Workflows are stored as JSON in SQLite and compiled into petgraph DAGs 
/// for execution. Each workflow can have multiple entry points (webhook nodes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Unique workflow identifier (e.g., "wf-grading")
    pub id: String,
    /// Human-readable workflow name  
    pub name: String,
    /// List of nodes in this workflow
    pub nodes: Vec<Node>,
    /// List of edges connecting nodes
    pub edges: Vec<Edge>,
}

/// A single node in the workflow DAG
/// 
/// Nodes represent discrete processing units (webhooks, transforms, database ops, etc).
/// Each node has a type that determines its behavior and a params object for configuration.
/// Optional input/output pins enable precise data selection and transformation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Unique node identifier within the workflow (e.g., "n1", "webhook-start")
    pub id: String,
    /// The type of node which determines execution behavior
    pub node_type: NodeType,
    /// Node-specific configuration parameters as flexible JSON
    pub params: Value,
    /// Optional input pin expressions for data selection (n8n-style)
    /// If None, uses entire context.data array as-is (backwards compatible)
    /// If Some, evaluates expressions against context.data to build input data
    pub inputs: Option<Vec<String>>,
    /// Optional output pin expressions for data transformation  
    /// If None, passes through node result as-is (backwards compatible)
    /// If Some, evaluates expressions against node result to build output data
    pub outputs: Option<Vec<String>>,
    /// Optional secret pin expressions for secure credential access (n8n-style)
    /// If None, node doesn't require secrets (backwards compatible)
    /// If Some, evaluates expressions like ["$secret.postgres_main"] to get credentials
    pub secrets: Option<Vec<String>>,
}

/// Available node types for the mechaway engine
/// 
/// Core nodes for proof of concept:
/// - WebhookNode: HTTP trigger entry point
/// - FunLogicNode: Embedded Lua script execution  
/// - SimpleTableWriterNode: Write data to SQLite data store
/// - SimpleTableReaderNode: Read data from SQLite data store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    /// HTTP webhook trigger node - creates dynamic endpoints
    /// Expected params: { "path": "/grade", "method": "POST" }
    Webhook,
    
    /// Embedded Lua script execution node  
    /// Expected params: { "script": "return {result = data.score * 2}" }
    FunLogic,
    
    /// Simple table writer to data SQLite database
    /// Expected params: { "table": "grades", "columns": ["id", "score", "result"] }
    SimpleTableWriter,
    
    /// Simple table reader from data SQLite database
    /// Expected params: { "table": "grades", "limit": 100, "where": "score > 70" }
    SimpleTableReader,
    
    /// Simple table query with input pins and bind parameters
    /// Expected params: { "table": "posts", "query": "SELECT * FROM posts WHERE slug = ?" }
    /// Expected inputs: ["$json.slug"] for bind parameters
    SimpleTableQuery,
    
    /// Background cron trigger for scheduled workflows
    /// Expected params: { "schedule": "0 */1 * * * *", "timezone": "UTC" }
    /// Starts workflow execution based on cron schedule
    CronTrigger,
    
    /// HTTP client for external API calls
    /// Expected params: { "url": "https://api.example.com/data", "method": "GET", "headers": {...} }
    /// Expected inputs: ["$json.payload"] for request body/query params
    HTTPClient,
    
    /// PostgreSQL query execution node (MANDATORY secret required)
    /// Expected params: { "query": "SELECT * FROM users WHERE id = ? AND status = ?" }
    /// Expected inputs: ["$json.user_id", "$json.status"] for bind parameters
    /// Expected secrets: ["$secret.postgres_main"] - MANDATORY, no fallbacks!
    PGQuery,
}

/// Connection between two nodes in the workflow DAG
/// 
/// Edges define the data flow direction from one node to another.
/// The execution engine uses these to build the dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// Source node ID 
    pub from: String,
    /// Target node ID
    pub to: String,
}

/// Runtime execution context passed between nodes
/// 
/// Contains the data payload and metadata for workflow execution.
/// Uses array-based processing like n8n for batch operations and consistent interface.
/// Even single items are wrapped in arrays: [item] for uniform processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Array of data items (n8n-style batch processing)
    /// Single webhook request becomes [request_data]
    /// Multiple items from previous nodes remain as [item1, item2, ...]
    pub data: Vec<Value>,
    /// Execution metadata (workflow_id, node_id, timestamps, etc)
    pub metadata: HashMap<String, Value>,
    /// Project slug for database isolation (e.g., "default", "ecommerce", "analytics")
    /// Determines which project.db and simpletable.db files to use
    pub project_slug: String,
}

impl ExecutionContext {
    /// Create a new execution context from initial webhook data
    /// Wraps single webhook request in array for consistent n8n-style processing
    pub fn from_webhook_data(workflow_id: String, data: Value, project_slug: String) -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("workflow_id".to_string(), Value::String(workflow_id));
        metadata.insert("started_at".to_string(), 
            Value::String(chrono::Utc::now().to_rfc3339()));
        
        // Wrap single webhook data in array for batch processing
        let data_array = vec![data];
        
        Self { data: data_array, metadata, project_slug }
    }
    
    /// Create execution context from array of items (for batch processing)
    pub fn from_array_data(workflow_id: String, data: Vec<Value>, project_slug: String) -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("workflow_id".to_string(), Value::String(workflow_id));
        metadata.insert("started_at".to_string(), 
            Value::String(chrono::Utc::now().to_rfc3339()));
        
        Self { data, metadata, project_slug }
    }
    
    /// Create execution context from cron trigger (scheduled execution)
    /// Provides timestamp and trigger info as data payload
    pub fn from_cron_trigger(workflow_id: String, trigger_node_id: String, project_slug: String) -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("workflow_id".to_string(), Value::String(workflow_id.clone()));
        metadata.insert("trigger_node_id".to_string(), Value::String(trigger_node_id));
        metadata.insert("trigger_type".to_string(), Value::String("cron".to_string()));
        metadata.insert("started_at".to_string(), 
            Value::String(chrono::Utc::now().to_rfc3339()));
        
        // Create trigger data payload with timestamp
        let trigger_data = serde_json::json!({
            "trigger_type": "cron",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "workflow_id": workflow_id,
            "project_slug": project_slug.clone()
        });
        
        Self { data: vec![trigger_data], metadata, project_slug }
    }
}

/// Project type definitions for multi-tenant architecture
/// 
/// Defines project structure with slug-based database isolation like n8n.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A project container for workflows, secrets, and LLM knowledge
/// 
/// Projects provide complete isolation with dedicated directories:
/// - {slug}/project.db: workflows, secrets, metadata
/// - {slug}/simpletable.db: SimpleTable data storage
/// - {slug}/uploads/: file uploads (future)
/// - {slug}/ai-models/: AI models (future)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Unique project identifier (e.g., "proj-ecommerce")
    pub id: String,
    /// URL-safe project slug for directory naming (e.g., "ecommerce", "analytics")
    /// Used to create isolated project directories: {slug}/project.db, {slug}/simpletable.db
    pub slug: String,
    /// Human-readable project name (e.g., "E-commerce Automation")
    pub name: String,
    /// Project description for documentation
    pub description: String,
    /// LLM knowledge base for AI workflow generation
    /// Contains project context, business rules, common patterns
    pub llm_context: String,
    /// Project-specific settings and configuration
    pub settings: Value,
}

impl Project {
    /// Get the project directory path
    pub fn project_dir(&self) -> String {
        self.slug.clone()
    }
    
    /// Get the project database path
    pub fn project_db_path(&self) -> String {
        format!("{}/project.db", self.slug)
    }
    
    /// Get the simpletable database path  
    pub fn simpletable_db_path(&self) -> String {
        format!("{}/simpletable.db", self.slug)
    }
    
    /// Create default project for backwards compatibility
    pub fn default() -> Self {
        Self {
            id: "default".to_string(),
            slug: "default".to_string(),
            name: "Default Project".to_string(),
            description: "Default project for existing workflows".to_string(),
            llm_context: "General-purpose workflow automation project".to_string(),
            settings: serde_json::json!({}),
        }
    }
}

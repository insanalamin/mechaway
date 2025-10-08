/// SQLite persistence layer for workflow storage
/// 
/// Handles workflow CRUD operations in the main SQLite database.
/// Workflows are stored as JSON for flexibility while maintaining structured queries.

use crate::workflow::types::Workflow;
use anyhow::Result;
use sqlx::{sqlite::SqlitePool, Row};
use std::collections::HashMap;

/// SQLite-based workflow storage manager
/// 
/// Provides persistent storage for workflow definitions with optimized queries.
/// Uses JSON column for workflow data while maintaining indexed lookup fields.
#[derive(Debug, Clone)]
pub struct WorkflowStorage {
    /// SQLite connection pool for workflow database
    pool: SqlitePool,
}

impl WorkflowStorage {
    /// Create new storage instance with database connection
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Initialize the workflow storage schema
    /// 
    /// Creates the workflows table with JSON storage and necessary indexes.
    /// Safe to call multiple times (uses IF NOT EXISTS).
    pub async fn init_schema(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workflows (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                definition JSON NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create index on name for fast lookups
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_workflows_name 
            ON workflows(name)
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Store a new workflow or update existing one
    /// 
    /// Uses UPSERT to handle both create and update operations atomically.
    /// Updates the updated_at timestamp automatically.
    pub async fn save_workflow(&self, workflow: &Workflow) -> Result<()> {
        let definition_json = serde_json::to_string(workflow)?;

        sqlx::query(
            r#"
            INSERT INTO workflows (id, name, definition, updated_at)
            VALUES (?, ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                definition = excluded.definition,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(&workflow.id)
        .bind(&workflow.name)  
        .bind(&definition_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Retrieve a workflow by ID
    pub async fn get_workflow(&self, id: &str) -> Result<Option<Workflow>> {
        let row = sqlx::query("SELECT definition FROM workflows WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => {
                let definition_json: String = row.get("definition");
                let workflow: Workflow = serde_json::from_str(&definition_json)?;
                Ok(Some(workflow))
            }
            None => Ok(None),
        }
    }

    /// List all workflows with basic metadata
    pub async fn list_workflows(&self) -> Result<Vec<WorkflowMetadata>> {
        let rows = sqlx::query(
            "SELECT id, name, created_at, updated_at FROM workflows ORDER BY updated_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut workflows = Vec::new();
        for row in rows {
            workflows.push(WorkflowMetadata {
                id: row.get("id"),
                name: row.get("name"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(workflows)
    }

    /// Load all workflows for registry initialization
    /// 
    /// Returns a map of workflow_id -> Workflow for efficient registry loading.
    /// Used during startup and hot-reload operations.
    pub async fn load_all_workflows(&self) -> Result<HashMap<String, Workflow>> {
        let rows = sqlx::query("SELECT id, definition FROM workflows")
            .fetch_all(&self.pool)
            .await?;

        let mut workflows = HashMap::new();
        for row in rows {
            let id: String = row.get("id");
            let definition_json: String = row.get("definition");
            let workflow: Workflow = serde_json::from_str(&definition_json)?;
            workflows.insert(id, workflow);
        }

        Ok(workflows)
    }

    /// Delete a workflow by ID
    pub async fn delete_workflow(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM workflows WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}

/// Basic workflow metadata for listing operations
#[derive(Debug, serde::Serialize)]
pub struct WorkflowMetadata {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

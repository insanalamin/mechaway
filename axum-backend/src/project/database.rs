/// Project database manager for isolated multi-tenant storage
/// 
/// Manages separate SQLite databases per project:
/// - {slug}_project.db: workflows, secrets, project metadata
/// - {slug}_simpletable.db: SimpleTable node data storage
/// 
/// INDUSTRIAL-GRADE: Connection pooling, lazy loading, zero cross-project data leaks

use crate::project::types::Project;
use anyhow::Result;
use sqlx::{sqlite::{SqlitePool, SqliteConnectOptions}, SqliteConnection};
use std::collections::HashMap;
use std::path::Path;
use tokio::sync::RwLock;

/// Project database manager with isolated connection pools
/// 
/// HYPERMINIMALIST: Lazy-loaded pools, minimal memory footprint
/// SCALABLE: Handles thousands of projects with efficient resource management
#[derive(Debug)]
pub struct ProjectDatabaseManager {
    /// Connection pools for project databases ({slug}_project.db)
    project_pools: RwLock<HashMap<String, SqlitePool>>,
    /// Connection pools for simpletable databases ({slug}_simpletable.db)  
    simpletable_pools: RwLock<HashMap<String, SqlitePool>>,
    /// Base directory for database files
    data_dir: String,
}

impl ProjectDatabaseManager {
    /// Create new project database manager
    pub fn new(data_dir: String) -> Self {
        Self {
            project_pools: RwLock::new(HashMap::new()),
            simpletable_pools: RwLock::new(HashMap::new()),
            data_dir,
        }
    }
    
    /// Get or create project database pool ({slug}_project.db)
    /// 
    /// LAZY LOADING: Creates pool only when first accessed
    /// THREAD-SAFE: Uses RwLock for concurrent access
    pub async fn get_project_pool(&self, project_slug: &str) -> Result<SqlitePool> {
        // Try read lock first (fast path for existing pools)
        {
            let pools = self.project_pools.read().await;
            if let Some(pool) = pools.get(project_slug) {
                return Ok(pool.clone());
            }
        }
        
        // Create new pool (slow path)
        let mut pools = self.project_pools.write().await;
        
        // Double-check pattern (another thread might have created it)
        if let Some(pool) = pools.get(project_slug) {
            return Ok(pool.clone());
        }
        
        // Create project directory and database file path
        let project_dir = Path::new(&self.data_dir).join(project_slug);
        std::fs::create_dir_all(&project_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create project directory '{}': {}", project_dir.display(), e))?;
        let db_path = project_dir.join("project.db");
        
        tracing::info!("🗄️ Creating project database pool: {}", db_path.display());
        
        // Create connection pool with auto-create option
        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = SqlitePool::connect_with(options).await?;
        
        // Initialize project database schema
        self.init_project_schema(&pool).await?;
        
        // Cache the pool
        pools.insert(project_slug.to_string(), pool.clone());
        
        tracing::info!("✅ Project database pool created: {}/project.db", project_slug);
        
        Ok(pool)
    }
    
    /// Get or create simpletable database pool ({slug}_simpletable.db)
    /// 
    /// LAZY LOADING: Creates pool only when first accessed by SimpleTable nodes
    pub async fn get_simpletable_pool(&self, project_slug: &str) -> Result<SqlitePool> {
        // Try read lock first (fast path for existing pools)
        {
            let pools = self.simpletable_pools.read().await;
            if let Some(pool) = pools.get(project_slug) {
                return Ok(pool.clone());
            }
        }
        
        // Create new pool (slow path)
        let mut pools = self.simpletable_pools.write().await;
        
        // Double-check pattern
        if let Some(pool) = pools.get(project_slug) {
            return Ok(pool.clone());
        }
        
        // Create project directory and database file path
        let project_dir = Path::new(&self.data_dir).join(project_slug);
        std::fs::create_dir_all(&project_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create project directory '{}': {}", project_dir.display(), e))?;
        let db_path = project_dir.join("simpletable.db");
        
        tracing::info!("🗄️ Creating simpletable database pool: {}", db_path.display());
        
        // Create connection pool with auto-create option
        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = SqlitePool::connect_with(options).await?;
        
        // Cache the pool (no schema init needed - tables created dynamically)
        pools.insert(project_slug.to_string(), pool.clone());
        
        tracing::info!("✅ Simpletable database pool created: {}/simpletable.db", project_slug);
        
        Ok(pool)
    }
    
    /// Initialize project database schema
    /// 
    /// Creates tables for workflows, secrets, and project metadata
    async fn init_project_schema(&self, pool: &SqlitePool) -> Result<()> {
        // Workflows table (project-scoped)
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
        .execute(pool)
        .await?;
        
        // Project secrets table (encrypted storage)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS project_secrets (
                id TEXT PRIMARY KEY,
                key TEXT NOT NULL UNIQUE,
                encrypted_value TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(pool)
        .await?;
        
        // Project metadata table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS project_metadata (
                key TEXT PRIMARY KEY,
                value JSON NOT NULL,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(pool)
        .await?;
        
        // Create indexes for performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_workflows_name ON workflows(name)")
            .execute(pool)
            .await?;
            
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_secrets_key ON project_secrets(key)")
            .execute(pool)
            .await?;
        
        Ok(())
    }
    
    /// Get pool statistics for monitoring
    pub async fn get_pool_stats(&self) -> (usize, usize) {
        let project_count = self.project_pools.read().await.len();
        let simpletable_count = self.simpletable_pools.read().await.len();
        (project_count, simpletable_count)
    }
}

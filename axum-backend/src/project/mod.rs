/// Project management module
/// 
/// Handles project-level organization, database isolation, and multi-tenancy.
/// Each project gets isolated databases: {slug}_project.db and {slug}_simpletable.db

pub mod database;
pub mod types;

pub use database::ProjectDatabaseManager;
pub use types::Project;

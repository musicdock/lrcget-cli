//! Core functionality modules
//!
//! This module contains all core business logic organized into logical layers:
//! - `data`: Database operations and data persistence
//! - `services`: External API integrations and service clients
//! - `files`: File operations and content processing
//! - `infrastructure`: Cross-cutting concerns (cache, hooks, templates)

pub mod data;
pub mod services;
pub mod files;
pub mod infrastructure;

// Re-export commonly used types for convenience
pub use data::Database;

// Legacy re-exports for backward compatibility

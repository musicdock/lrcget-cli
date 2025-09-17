//! Management and configuration commands
//!
//! This module contains commands for managing the application:
//! configuration, cache, data export, hooks, and templates.

pub mod config;
pub mod cache;
pub mod export;
pub mod hooks;
pub mod templates;

// Re-export for convenience
pub use config::*;
pub use cache::*;
pub use export::*;
pub use hooks::*;
pub use templates::*;
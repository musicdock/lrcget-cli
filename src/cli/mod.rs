//! Command Line Interface module
//!
//! This module contains all CLI commands organized into logical submodules:
//! - `core`: Essential operations (init, scan, download, search)
//! - `operations`: Advanced operations (fetch, watch, batch)
//! - `management`: Configuration and maintenance (config, cache, export, hooks, templates)

pub mod core;
pub mod operations;
pub mod management;

// Re-export all commands for backward compatibility and convenience
pub use core::*;
pub use operations::*;
pub use management::*;
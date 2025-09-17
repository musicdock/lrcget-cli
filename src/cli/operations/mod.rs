//! Advanced operations for power users
//!
//! This module contains advanced commands for complex workflows:
//! fetching specific lyrics, monitoring file changes, and batch processing.

pub mod fetch;
pub mod watch;
pub mod batch;

// Re-export for convenience
pub use fetch::*;
pub use watch::*;
pub use batch::*;
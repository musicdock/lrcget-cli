//! Core CLI commands for essential operations
//!
//! This module contains the fundamental commands that users need for basic
//! lyrics management: initializing libraries, scanning music files,
//! downloading lyrics, and searching for lyrics.

pub mod init;
pub mod scan;
pub mod download;
pub mod search;

// Re-export for convenience
pub use init::*;
pub use scan::*;
pub use download::*;
pub use search::*;
//! File operations and content processing
//!
//! This module contains functionality for working with files:
//! - Music file scanning and metadata extraction
//! - Lyrics file processing and validation

pub mod scanner;
pub mod lyrics;

// Re-export main types
pub use scanner::{Scanner, Track};
pub use lyrics::LyricsManager;
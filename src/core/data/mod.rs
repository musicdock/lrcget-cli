//! Data layer modules
//!
//! This module contains all database-related functionality:
//! - SQLite database operations for local music library
//! - LRCLIB database operations for cached lyrics

pub mod database;
pub mod lrclib_db;

// Re-export main types
pub use database::Database;
pub use lrclib_db::{LrclibDatabase, LrclibTrack};
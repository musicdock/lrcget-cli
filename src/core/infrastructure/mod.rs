//! Infrastructure and cross-cutting concerns
//!
//! This module contains infrastructure components:
//! - Caching layer for performance optimization
//! - Hook system for extensibility
//! - Template engine for output formatting

pub mod cache;
pub mod hooks;
pub mod templates;

// Re-export main types
pub use cache::{LyricsCache, LyricsCacheInterface, CacheService};
pub use hooks::{HookManager, HookEvent};
pub use templates::TemplateEngine;
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
pub use data::{Database, LrclibDatabase, LrclibTrack};
pub use services::{LrclibClient, LyricsDownloader, LyricsResponse, SearchResult};
pub use files::{Scanner, Track, LyricsManager};
pub use infrastructure::{LyricsCache, LyricsCacheInterface, CacheService, HookManager, HookEvent, TemplateEngine};

// Legacy re-exports for backward compatibility
pub use services::LrclibClient as LrcLibClient;
pub use services::LyricsDownloader as Downloader;
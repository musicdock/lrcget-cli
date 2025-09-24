//! Service layer for dependency injection and service management
//!
//! This module provides service abstractions and dependency injection patterns:
//! - `ServiceFactory`: Factory pattern for creating service instances
//! - `SimpleServices`: Lightweight service container
//! - `DatabaseService`: Database service abstraction
//! - `ServiceProvider`: Service provider trait for dependency injection

use std::sync::Arc;
use crate::config::Config;
use crate::core::services::lrclib::{LrclibClient as LrcLibClient, LyricsDownloader as Downloader};
use crate::core::files::scanner::Scanner;
use crate::core::infrastructure::cache::CacheService;
use crate::error::Result;

// pub mod container;
pub mod database;
// pub mod client_factory;
pub mod simple_container;
pub mod factory;

// pub use container::ServiceContainer;
pub use database::DatabaseService;
// pub use client_factory::ClientFactory;
pub use simple_container::SimpleServices;
pub use factory::ServiceFactory;

#[async_trait::async_trait]
pub trait ServiceProvider {
    async fn database(&self) -> Result<Arc<dyn DatabaseService>>;
    async fn lrclib_client(&self) -> Result<Arc<LrcLibClient>>;
    async fn cache_service(&self) -> Result<Arc<dyn CacheService>>;
    async fn downloader(&self) -> Result<Arc<Downloader>>;
    async fn scanner(&self) -> Result<Arc<Scanner>>;
    fn config(&self) -> Arc<Config>;
}

// Temporarily disabled complex Services
// pub struct Services {
//     container: Arc<ServiceContainer>,
// }

// Temporarily disabled ServiceProvider implementation
//! Error handling for the lrcget-cli application
//!
//! This module provides a hierarchical error system with proper error handling
//! and user-friendly error messages. All errors are typed and can be handled
//! appropriately by different parts of the application.

use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LrcGetError {
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    #[error("Network error: {0}")]
    Network(#[from] NetworkError),

    #[error("File system error: {0}")]
    FileSystem(#[from] FileSystemError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Audio metadata error: {0}")]
    AudioMetadata(#[from] AudioMetadataError),

    #[error("Cache error: {0}")]
    Cache(#[from] CacheError),

    #[error("Lyrics processing error: {0}")]
    Lyrics(#[from] LyricsError),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Operation cancelled by user")]
    Cancelled,

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Connection failed: {0}")]
    Connection(#[source] rusqlite::Error),

    #[error("Query failed: {0}")]
    Query(#[source] rusqlite::Error),

    #[error("Migration failed: {0}")]
    Migration(String),

    #[error("Transaction failed: {0}")]
    Transaction(#[source] rusqlite::Error),

    #[error("Database file not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Database corruption detected")]
    Corruption,
}

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API rate limit exceeded")]
    RateLimit,

    #[error("API response invalid: {reason}")]
    InvalidResponse { reason: String },

    #[error("Authentication failed")]
    Authentication,

    #[error("Service unavailable")]
    ServiceUnavailable,

    #[error("Timeout exceeded")]
    Timeout,
}

#[derive(Error, Debug)]
pub enum FileSystemError {
    #[error("IO error: {0}")]
    Io(std::io::Error),

    #[error("Path not found: {path}")]
    PathNotFound { path: PathBuf },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    #[error("Invalid file format: {path}")]
    InvalidFormat { path: PathBuf },

    #[error("File already exists: {path}")]
    AlreadyExists { path: PathBuf },
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Invalid config format: {0}")]
    InvalidFormat(#[from] toml::de::Error),

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid value for {field}: {value}")]
    InvalidValue { field: String, value: String },

    #[error("Environment variable error: {0}")]
    Environment(#[from] std::env::VarError),
}

#[derive(Error, Debug)]
pub enum AudioMetadataError {
    #[error("Failed to read metadata: {0}")]
    ReadFailed(#[from] lofty::error::LoftyError),

    #[error("Unsupported audio format: {path}")]
    UnsupportedFormat { path: PathBuf },

    #[error("Missing required metadata: {field}")]
    MissingMetadata { field: String },

    #[error("Invalid duration")]
    InvalidDuration,
}

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Redis connection failed: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("File cache error: {0}")]
    FileCache(#[from] std::io::Error),

    #[error("Serialization failed: {0}")]
    Serialization(serde_json::Error),

    #[error("Cache key not found: {key}")]
    KeyNotFound { key: String },

    #[error("Cache expired: {key}")]
    Expired { key: String },
}

#[derive(Error, Debug)]
pub enum LyricsError {
    #[error("Failed to parse LRC: {reason}")]
    ParseFailed { reason: String },

    #[error("No lyrics found for track")]
    NotFound,

    #[error("Lyrics already exist: {path}")]
    AlreadyExists { path: PathBuf },

    #[error("Invalid lyrics format")]
    InvalidFormat,

    #[error("Failed to embed lyrics: {reason}")]
    EmbedFailed { reason: String },
}

pub type Result<T> = std::result::Result<T, LrcGetError>;

impl From<rusqlite::Error> for DatabaseError {
    fn from(err: rusqlite::Error) -> Self {
        match err {
            rusqlite::Error::SqliteFailure(ffi::Error { code: ffi::ErrorCode::DatabaseCorrupt, .. }, _) => {
                DatabaseError::Corruption
            }
            _ => DatabaseError::Query(err),
        }
    }
}

use rusqlite::ffi;

impl From<std::io::Error> for FileSystemError {
    fn from(err: std::io::Error) -> Self {
        use std::io::ErrorKind;
        match err.kind() {
            ErrorKind::NotFound => FileSystemError::Io(err),
            ErrorKind::PermissionDenied => FileSystemError::Io(err),
            _ => FileSystemError::Io(err),
        }
    }
}

impl From<serde_json::Error> for CacheError {
    fn from(err: serde_json::Error) -> Self {
        CacheError::Serialization(err)
    }
}

impl From<std::io::Error> for LrcGetError {
    fn from(err: std::io::Error) -> Self {
        LrcGetError::FileSystem(FileSystemError::Io(err))
    }
}

impl From<serde_json::Error> for LrcGetError {
    fn from(err: serde_json::Error) -> Self {
        LrcGetError::Cache(CacheError::Serialization(err))
    }
}

impl From<toml::de::Error> for LrcGetError {
    fn from(err: toml::de::Error) -> Self {
        LrcGetError::Config(ConfigError::InvalidFormat(err))
    }
}

impl From<tokio::task::JoinError> for LrcGetError {
    fn from(err: tokio::task::JoinError) -> Self {
        LrcGetError::Internal(err.into())
    }
}
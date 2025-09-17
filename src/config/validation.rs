use std::path::Path;
use url::Url;
use crate::error::{Result, LrcGetError};

/// Centralized configuration validation utilities
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate a URL string
    pub fn validate_url(url: &str, field_name: &str) -> Result<()> {
        Url::parse(url).map_err(|e| {
            LrcGetError::Validation(format!("Invalid {} URL '{}': {}", field_name, url, e))
        })?;
        Ok(())
    }

    /// Validate a path exists or can be created
    pub fn validate_path(path: &Path, field_name: &str, should_exist: bool) -> Result<()> {
        if should_exist && !path.exists() {
            return Err(LrcGetError::Validation(format!(
                "{} path does not exist: {}",
                field_name,
                path.display()
            )));
        }

        // Check if parent directory exists or can be created
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    LrcGetError::Validation(format!(
                        "Cannot create parent directory for {}: {}",
                        field_name, e
                    ))
                })?;
            }
        }

        Ok(())
    }

    /// Validate numeric range
    pub fn validate_range<T>(value: T, min: T, max: T, field_name: &str) -> Result<()>
    where
        T: PartialOrd + std::fmt::Display + Copy,
    {
        if value < min || value > max {
            return Err(LrcGetError::Validation(format!(
                "{} must be between {} and {}, got {}",
                field_name, min, max, value
            )));
        }
        Ok(())
    }

    /// Validate Redis URL format
    pub fn validate_redis_url(url: &str) -> Result<()> {
        if !url.starts_with("redis://") && !url.starts_with("rediss://") {
            return Err(LrcGetError::Validation(format!(
                "Redis URL must start with 'redis://' or 'rediss://', got: {}",
                url
            )));
        }

        Self::validate_url(url, "Redis")?;
        Ok(())
    }

    /// Validate database file extension
    pub fn validate_db_path(path: &Path) -> Result<()> {
        if let Some(ext) = path.extension() {
            if ext != "db" && ext != "sqlite" && ext != "sqlite3" {
                return Err(LrcGetError::Validation(format!(
                    "Database file should have .db, .sqlite, or .sqlite3 extension, got: {}",
                    path.display()
                )));
            }
        } else {
            return Err(LrcGetError::Validation(format!(
                "Database file should have an extension (.db, .sqlite, .sqlite3), got: {}",
                path.display()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_validate_url() {
        assert!(ConfigValidator::validate_url("https://lrclib.net", "LRCLIB").is_ok());
        assert!(ConfigValidator::validate_url("not-a-url", "LRCLIB").is_err());
    }

    #[test]
    fn test_validate_range() {
        assert!(ConfigValidator::validate_range(5u64, 1u64, 10u64, "test").is_ok());
        assert!(ConfigValidator::validate_range(15u64, 1u64, 10u64, "test").is_err());
        assert!(ConfigValidator::validate_range(0u64, 1u64, 10u64, "test").is_err());
    }

    #[test]
    fn test_validate_redis_url() {
        assert!(ConfigValidator::validate_redis_url("redis://localhost:6379").is_ok());
        assert!(ConfigValidator::validate_redis_url("rediss://localhost:6380").is_ok());
        assert!(ConfigValidator::validate_redis_url("http://localhost:6379").is_err());
    }

    #[test]
    fn test_validate_db_path() {
        assert!(ConfigValidator::validate_db_path(&PathBuf::from("test.db")).is_ok());
        assert!(ConfigValidator::validate_db_path(&PathBuf::from("test.sqlite")).is_ok());
        assert!(ConfigValidator::validate_db_path(&PathBuf::from("test.sqlite3")).is_ok());
        assert!(ConfigValidator::validate_db_path(&PathBuf::from("test.txt")).is_err());
        assert!(ConfigValidator::validate_db_path(&PathBuf::from("test")).is_err());
    }
}
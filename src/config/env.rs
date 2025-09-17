use std::env;
use std::path::PathBuf;
use crate::error::{Result, LrcGetError};

/// Environment variable configuration constants
pub struct EnvVars;

impl EnvVars {
    pub const DATABASE_PATH: &'static str = "LRCGET_DATABASE_PATH";
    pub const LRCLIB_INSTANCE: &'static str = "LRCGET_LRCLIB_INSTANCE";
    pub const LRCLIB_DATABASE_PATH: &'static str = "LRCGET_LRCLIB_DATABASE_PATH";
    pub const SKIP_TRACKS_WITH_SYNCED_LYRICS: &'static str = "LRCGET_SKIP_TRACKS_WITH_SYNCED_LYRICS";
    pub const SKIP_TRACKS_WITH_PLAIN_LYRICS: &'static str = "LRCGET_SKIP_TRACKS_WITH_PLAIN_LYRICS";
    pub const TRY_EMBED_LYRICS: &'static str = "LRCGET_TRY_EMBED_LYRICS";
    pub const SHOW_LINE_COUNT: &'static str = "LRCGET_SHOW_LINE_COUNT";
    pub const WATCH_DEBOUNCE_SECONDS: &'static str = "LRCGET_WATCH_DEBOUNCE_SECONDS";
    pub const WATCH_BATCH_SIZE: &'static str = "LRCGET_WATCH_BATCH_SIZE";
    pub const REDIS_URL: &'static str = "LRCGET_REDIS_URL";

    // Special environment variables
    pub const DOCKER: &'static str = "DOCKER";
    pub const CI: &'static str = "CI";
    pub const FORCE_API_ONLY: &'static str = "FORCE_API_ONLY";
    pub const FORCE_TERMINAL_UI: &'static str = "LRCGET_FORCE_TERMINAL_UI";
}

/// Environment variable parsing utilities with validation
pub struct EnvParser;

impl EnvParser {
    /// Parse environment variable as string with validation
    pub fn parse_string(var_name: &str, validator: Option<fn(&str) -> Result<()>>) -> Result<Option<String>> {
        match env::var(var_name) {
            Ok(value) => {
                let trimmed = value.trim().to_string();
                if trimmed.is_empty() {
                    return Ok(None);
                }

                if let Some(validate_fn) = validator {
                    validate_fn(&trimmed)?;
                }

                Ok(Some(trimmed))
            }
            Err(env::VarError::NotPresent) => Ok(None),
            Err(env::VarError::NotUnicode(_)) => {
                Err(LrcGetError::Validation(format!(
                    "Environment variable {} contains invalid UTF-8",
                    var_name
                )))
            }
        }
    }

    /// Parse environment variable as PathBuf with validation
    pub fn parse_path(var_name: &str, should_exist: bool) -> Result<Option<PathBuf>> {
        if let Some(path_str) = Self::parse_string(var_name, None)? {
            let path = PathBuf::from(path_str);

            if should_exist && !path.exists() {
                return Err(LrcGetError::Validation(format!(
                    "Path specified in {} does not exist: {}",
                    var_name,
                    path.display()
                )));
            }

            Ok(Some(path))
        } else {
            Ok(None)
        }
    }

    /// Parse environment variable as boolean with validation
    pub fn parse_bool(var_name: &str) -> Result<Option<bool>> {
        if let Some(value_str) = Self::parse_string(var_name, None)? {
            match value_str.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Ok(Some(true)),
                "false" | "0" | "no" | "off" => Ok(Some(false)),
                _ => Err(LrcGetError::Validation(format!(
                    "Invalid boolean value in {}: '{}'. Use: true/false, 1/0, yes/no, on/off",
                    var_name, value_str
                )))
            }
        } else {
            Ok(None)
        }
    }

    /// Parse environment variable as u64 with range validation
    pub fn parse_u64(var_name: &str, min: u64, max: u64) -> Result<Option<u64>> {
        if let Some(value_str) = Self::parse_string(var_name, None)? {
            let value = value_str.parse::<u64>().map_err(|_| {
                LrcGetError::Validation(format!(
                    "Invalid number in {}: '{}'. Must be a positive integer",
                    var_name, value_str
                ))
            })?;

            if value < min || value > max {
                return Err(LrcGetError::Validation(format!(
                    "Value in {} must be between {} and {}, got {}",
                    var_name, min, max, value
                )));
            }

            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Parse environment variable as usize with range validation
    pub fn parse_usize(var_name: &str, min: usize, max: usize) -> Result<Option<usize>> {
        if let Some(value_str) = Self::parse_string(var_name, None)? {
            let value = value_str.parse::<usize>().map_err(|_| {
                LrcGetError::Validation(format!(
                    "Invalid number in {}: '{}'. Must be a positive integer",
                    var_name, value_str
                ))
            })?;

            if value < min || value > max {
                return Err(LrcGetError::Validation(format!(
                    "Value in {} must be between {} and {}, got {}",
                    var_name, min, max, value
                )));
            }

            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Check if environment variable is present (for boolean flags)
    pub fn is_present(var_name: &str) -> bool {
        env::var(var_name).is_ok()
    }

    /// Get all LRCGET environment variables for debugging
    pub fn get_all_lrcget_vars() -> Vec<(String, String)> {
        env::vars()
            .filter(|(key, _)| key.starts_with("LRCGET_"))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_parse_bool() {
        env::set_var("TEST_BOOL_TRUE", "true");
        env::set_var("TEST_BOOL_FALSE", "false");
        env::set_var("TEST_BOOL_INVALID", "maybe");

        assert_eq!(EnvParser::parse_bool("TEST_BOOL_TRUE").unwrap(), Some(true));
        assert_eq!(EnvParser::parse_bool("TEST_BOOL_FALSE").unwrap(), Some(false));
        assert!(EnvParser::parse_bool("TEST_BOOL_INVALID").is_err());
        assert_eq!(EnvParser::parse_bool("TEST_BOOL_NOT_SET").unwrap(), None);

        env::remove_var("TEST_BOOL_TRUE");
        env::remove_var("TEST_BOOL_FALSE");
        env::remove_var("TEST_BOOL_INVALID");
    }

    #[test]
    fn test_parse_u64() {
        env::set_var("TEST_U64_VALID", "42");
        env::set_var("TEST_U64_OUT_OF_RANGE", "150");
        env::set_var("TEST_U64_INVALID", "not_a_number");

        assert_eq!(EnvParser::parse_u64("TEST_U64_VALID", 1, 100).unwrap(), Some(42));
        assert!(EnvParser::parse_u64("TEST_U64_OUT_OF_RANGE", 1, 100).is_err());
        assert!(EnvParser::parse_u64("TEST_U64_INVALID", 1, 100).is_err());
        assert_eq!(EnvParser::parse_u64("TEST_U64_NOT_SET", 1, 100).unwrap(), None);

        env::remove_var("TEST_U64_VALID");
        env::remove_var("TEST_U64_OUT_OF_RANGE");
        env::remove_var("TEST_U64_INVALID");
    }
}
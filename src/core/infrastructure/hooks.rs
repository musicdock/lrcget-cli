use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tokio::process::Command as AsyncCommand;
use tracing::{debug, warn, info};

use crate::core::data::database::DatabaseTrack;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum HookEvent {
    PreScan,
    PostScan,
    PreDownload,
    PostDownload,
    PreTrackDownload,
    PostTrackDownload,
    PreSearch,
    PostSearch,
    LyricsFound,
    LyricsNotFound,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: Option<PathBuf>,
    pub timeout_seconds: Option<u64>,
    pub enabled: bool,
    pub async_execution: bool,
}

#[derive(Debug, Serialize)]
pub struct HookContext {
    pub event: HookEvent,
    pub track: Option<DatabaseTrack>,
    pub metadata: HashMap<String, serde_json::Value>,
}

pub struct HookManager {
    hooks: HashMap<HookEvent, Vec<Hook>>,
}

impl HookManager {
    pub fn new() -> Self {
        Self {
            hooks: HashMap::new(),
        }
    }

    pub fn register_hook(&mut self, event: HookEvent, hook: Hook) {
        self.hooks.entry(event).or_insert_with(Vec::new).push(hook);
    }

    pub fn load_from_config(&mut self, config_path: &PathBuf) -> Result<()> {
        if !config_path.exists() {
            debug!("Hook configuration file not found: {}", config_path.display());
            return Ok(());
        }

        let content = std::fs::read_to_string(config_path)?;
        let hook_config: HookConfig = toml::from_str(&content)?;

        for (event_name, hooks) in hook_config.hooks {
            if let Some(event) = self.parse_event(&event_name) {
                for hook in hooks {
                    if hook.enabled {
                        self.register_hook(event.clone(), hook);
                    }
                }
            } else {
                warn!("Unknown hook event: {}", event_name);
            }
        }

        info!("Loaded {} hook events from configuration", self.hooks.len());
        Ok(())
    }

    pub async fn execute_hooks(&self, event: HookEvent, context: HookContext) -> Result<()> {
        if let Some(hooks) = self.hooks.get(&event) {
            info!("Executing {} hooks for event: {:?}", hooks.len(), event);
            
            for hook in hooks {
                if hook.enabled {
                    self.execute_hook(hook, &context).await?;
                }
            }
        }
        Ok(())
    }

    async fn execute_hook(&self, hook: &Hook, context: &HookContext) -> Result<()> {
        debug!("Executing hook: {} for event: {:?}", hook.name, context.event);

        let context_json = serde_json::to_string(context)?;
        
        if hook.async_execution {
            // Fire and forget
            let hook = hook.clone();
            let context_json = context_json.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::run_hook_command(&hook, &context_json).await {
                    warn!("Async hook {} failed: {}", hook.name, e);
                }
            });
        } else {
            // Wait for completion
            Self::run_hook_command(hook, &context_json).await?;
        }

        Ok(())
    }

    async fn run_hook_command(hook: &Hook, context_json: &str) -> Result<()> {
        let mut cmd = AsyncCommand::new(&hook.command);
        
        // Set arguments
        cmd.args(&hook.args);
        
        // Set working directory
        if let Some(ref working_dir) = hook.working_dir {
            cmd.current_dir(working_dir);
        }
        
        // Pass context via stdin
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        // Set environment variables
        cmd.env("LRCGET_HOOK_CONTEXT", context_json);
        
        let mut child = cmd.spawn()?;
        
        // Write context to stdin
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(context_json.as_bytes()).await?;
            stdin.shutdown().await?;
        }
        
        // Wait for completion with timeout
        let result = if let Some(timeout) = hook.timeout_seconds {
            tokio::time::timeout(
                std::time::Duration::from_secs(timeout),
                child.wait()
            ).await??
        } else {
            child.wait().await?
        };
        
        if result.success() {
            debug!("Hook {} completed successfully", hook.name);
        } else {
            warn!("Hook {} exited with code: {:?}", hook.name, result.code());
        }
        
        Ok(())
    }

    fn parse_event(&self, event_name: &str) -> Option<HookEvent> {
        match event_name.to_lowercase().as_str() {
            "pre_scan" | "prescan" => Some(HookEvent::PreScan),
            "post_scan" | "postscan" => Some(HookEvent::PostScan),
            "pre_download" | "predownload" => Some(HookEvent::PreDownload),
            "post_download" | "postdownload" => Some(HookEvent::PostDownload),
            "pre_track_download" | "pretrackdownload" => Some(HookEvent::PreTrackDownload),
            "post_track_download" | "posttrackdownload" => Some(HookEvent::PostTrackDownload),
            "pre_search" | "presearch" => Some(HookEvent::PreSearch),
            "post_search" | "postsearch" => Some(HookEvent::PostSearch),
            "lyrics_found" | "lyricsfound" => Some(HookEvent::LyricsFound),
            "lyrics_not_found" | "lyricsnotfound" => Some(HookEvent::LyricsNotFound),
            "error" => Some(HookEvent::Error),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct HookConfig {
    hooks: HashMap<String, Vec<Hook>>,
}

impl Default for HookManager {
    fn default() -> Self {
        Self::new()
    }
}

// Helper to create sample hook configuration
pub fn create_sample_hook_config(config_path: &PathBuf) -> Result<()> {
    let sample_config = r#"
# Sample hooks configuration for lrcget-cli
# Hooks allow you to run custom scripts at various points in the process

[hooks]

# Hooks that run before scanning starts
pre_scan = [
    { name = "backup", command = "rsync", args = ["-av", "/music/", "/backup/music/"], enabled = false, async_execution = true, timeout_seconds = 300 }
]

# Hooks that run after scanning completes
post_scan = [
    { name = "notify_scan_complete", command = "notify-send", args = ["LRCGET", "Music scan completed"], enabled = false, async_execution = true }
]

# Hooks that run before download batch starts
pre_download = [
    { name = "log_start", command = "echo", args = ["Starting lyrics download batch"], enabled = false, async_execution = false }
]

# Hooks that run after download batch completes
post_download = [
    { name = "update_music_player", command = "/usr/local/bin/update_music_db.sh", args = [], enabled = false, async_execution = true, timeout_seconds = 60 },
    { name = "notify_complete", command = "notify-send", args = ["LRCGET", "Lyrics download completed"], enabled = false, async_execution = true }
]

# Hooks that run before each individual track download
pre_track_download = []

# Hooks that run after each individual track download
post_track_download = [
    { name = "log_track", command = "logger", args = ["-t", "lrcget", "Downloaded lyrics for track"], enabled = false, async_execution = true }
]

# Hooks that run when lyrics are found for a track
lyrics_found = [
    { name = "celebrate", command = "echo", args = ["🎉 Found lyrics!"], enabled = false, async_execution = false }
]

# Hooks that run when lyrics are not found for a track
lyrics_not_found = [
    { name = "log_missing", command = "logger", args = ["-t", "lrcget", "Missing lyrics"], enabled = false, async_execution = true }
]

# Hooks that run when errors occur
error = [
    { name = "error_notification", command = "notify-send", args = ["-u", "critical", "LRCGET Error", "An error occurred"], enabled = false, async_execution = true }
]
"#;

    std::fs::write(config_path, sample_config)?;
    info!("Sample hook configuration created at: {}", config_path.display());
    Ok(())
}
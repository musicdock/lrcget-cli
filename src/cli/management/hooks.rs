use clap::{Args, Subcommand};
use anyhow::Result;

use crate::config::Config;
use crate::core::infrastructure::hooks::{HookManager, create_sample_hook_config};

#[derive(Args)]
pub struct HooksArgs {
    #[command(subcommand)]
    command: HooksCommands,
}

#[derive(Subcommand)]
enum HooksCommands {
    /// List all configured hooks
    List,
    
    /// Create sample hooks configuration
    Init,
    
    /// Test hook execution with sample data
    Test {
        /// Hook event to test
        #[arg(value_enum)]
        event: TestEvent,
    },
    
    /// Show hooks configuration path
    Path,
}

#[derive(clap::ValueEnum, Clone)]
enum TestEvent {
    PreScan,
    PostScan,
    PreDownload,
    PostDownload,
    LyricsFound,
    LyricsNotFound,
}

pub async fn execute(args: HooksArgs, config: &Config) -> Result<()> {
    let hooks_config_path = config.database_path.parent()
        .unwrap_or(&config.database_path)
        .join("hooks.toml");

    match args.command {
        HooksCommands::List => {
            if !hooks_config_path.exists() {
                println!("❌ No hooks configuration found.");
                println!("💡 Use 'lrcget hooks init' to create a sample configuration.");
                return Ok(());
            }

            let mut hook_manager = HookManager::new();
            hook_manager.load_from_config(&hooks_config_path)?;

            println!("🔗 Configured Hooks");
            println!("═══════════════════");
            println!("📄 Config file: {}", hooks_config_path.display());
            
            // This would require exposing hooks from HookManager
            // For now, just show the config file exists
            println!("✅ Hooks configuration loaded successfully");
            println!("💡 Edit {} to configure your hooks", hooks_config_path.display());
        },

        HooksCommands::Init => {
            if hooks_config_path.exists() {
                println!("⚠️  Hooks configuration already exists at: {}", hooks_config_path.display());
                println!("💡 Delete the file first if you want to recreate it.");
                return Ok(());
            }

            create_sample_hook_config(&hooks_config_path)?;
            println!("✅ Sample hooks configuration created!");
            println!("📄 Location: {}", hooks_config_path.display());
            println!("\n💡 Next steps:");
            println!("  1. Edit the configuration file to enable and customize hooks");
            println!("  2. Use 'lrcget hooks list' to view configured hooks");
            println!("  3. Use 'lrcget hooks test <event>' to test hook execution");
        },

        HooksCommands::Test { event } => {
            if !hooks_config_path.exists() {
                println!("❌ No hooks configuration found.");
                println!("💡 Use 'lrcget hooks init' to create a sample configuration first.");
                return Ok(());
            }

            let mut hook_manager = HookManager::new();
            hook_manager.load_from_config(&hooks_config_path)?;

            use crate::core::infrastructure::hooks::{HookEvent, HookContext};
            use std::collections::HashMap;

            let hook_event = match event {
                TestEvent::PreScan => HookEvent::PreScan,
                TestEvent::PostScan => HookEvent::PostScan,
                TestEvent::PreDownload => HookEvent::PreDownload,
                TestEvent::PostDownload => HookEvent::PostDownload,
                TestEvent::LyricsFound => HookEvent::LyricsFound,
                TestEvent::LyricsNotFound => HookEvent::LyricsNotFound,
            };

            let mut metadata = HashMap::new();
            metadata.insert("test".to_string(), serde_json::Value::Bool(true));
            metadata.insert("timestamp".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

            let context = HookContext {
                event: hook_event.clone(),
                track: None,
                metadata,
            };

            println!("🧪 Testing hooks for event: {:?}", hook_event);
            hook_manager.execute_hooks(hook_event, context).await?;
            println!("✅ Hook test completed");
        },

        HooksCommands::Path => {
            println!("📄 Hooks configuration path:");
            println!("{}", hooks_config_path.display());
            
            if hooks_config_path.exists() {
                println!("✅ Configuration file exists");
            } else {
                println!("❌ Configuration file not found");
                println!("💡 Use 'lrcget hooks init' to create it");
            }
        },
    }

    Ok(())
}
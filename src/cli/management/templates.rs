use clap::{Args, Subcommand};
use anyhow::Result;
use std::fs;

use crate::config::Config;
use crate::core::data::database::Database;
use crate::core::infrastructure::templates::{TemplateEngine, create_sample_template_config, build_context};

#[derive(Args)]
pub struct TemplatesArgs {
    #[command(subcommand)]
    command: TemplatesCommands,
}

#[derive(Subcommand)]
enum TemplatesCommands {
    /// List available templates
    List,
    
    /// Create sample template configuration
    Init,
    
    /// Render a template with library data
    Render {
        /// Template name to render
        template_name: String,
        
        /// Output file path (optional)
        #[arg(short, long)]
        output: Option<String>,
        
        /// Filter by artist
        #[arg(long)]
        artist: Option<String>,
        
        /// Filter by album
        #[arg(long)]
        album: Option<String>,
        
        /// Include only tracks missing lyrics
        #[arg(long)]
        missing_only: bool,
    },
    
    /// Show template configuration path
    Path,

    /// Test template rendering with sample data
    Test {
        /// Template name to test
        template_name: String,
    },
}

pub async fn execute(args: TemplatesArgs, config: &Config) -> Result<()> {
    let templates_config_path = config.database_path.parent()
        .unwrap_or(&config.database_path)
        .join("templates.toml");

    match args.command {
        TemplatesCommands::List => {
            if !templates_config_path.exists() {
                println!("❌ No template configuration found.");
                println!("💡 Use 'lrcget templates init' to create sample templates.");
                return Ok(());
            }

            let mut engine = TemplateEngine::new();
            engine.load_templates_from_config(&templates_config_path)?;

            let templates = engine.list_templates();
            
            if templates.is_empty() {
                println!("📝 No templates configured.");
            } else {
                println!("📝 Available Templates");
                println!("══════════════════════");
                println!("📄 Config: {}", templates_config_path.display());
                println!();

                for template in templates {
                    println!("🎨 {} ({})", template.name, template.description);
                    println!("   Format: {:?}", template.output_format);
                    println!("   Status: {}", if template.enabled { "✅ Enabled" } else { "❌ Disabled" });
                    println!();
                }
            }

            println!("💡 Usage:");
            println!("  lrcget templates render <template_name>");
            println!("  lrcget templates render library_summary --output report.txt");
        },

        TemplatesCommands::Init => {
            if templates_config_path.exists() {
                println!("⚠️  Template configuration already exists at: {}", templates_config_path.display());
                println!("💡 Delete the file first if you want to recreate it.");
                return Ok(());
            }

            create_sample_template_config(&templates_config_path)?;
            println!("✅ Sample template configuration created!");
            println!("📄 Location: {}", templates_config_path.display());
            println!("\n💡 Next steps:");
            println!("  1. Use 'lrcget templates list' to see available templates");
            println!("  2. Use 'lrcget templates render <name>' to generate output");
            println!("  3. Edit the config file to customize templates");
        },

        TemplatesCommands::Render { template_name, output, artist, album, missing_only } => {
            if !templates_config_path.exists() {
                println!("❌ No template configuration found.");
                println!("💡 Use 'lrcget templates init' first.");
                return Ok(());
            }

            // Load templates
            let mut engine = TemplateEngine::new();
            engine.load_templates_from_config(&templates_config_path)?;

            // Check if template exists
            if engine.get_template(&template_name).is_none() {
                println!("❌ Template '{}' not found.", template_name);
                println!("💡 Use 'lrcget templates list' to see available templates.");
                return Ok(());
            }

            // Load tracks from database
            let db = Database::new(&config.database_path).await?;
            let mut tracks = db.get_all_tracks().await?;

            // Apply filters
            if let Some(artist_filter) = &artist {
                tracks.retain(|t| t.artist_name.to_lowercase().contains(&artist_filter.to_lowercase()));
            }

            if let Some(album_filter) = &album {
                tracks.retain(|t| t.album_name.to_lowercase().contains(&album_filter.to_lowercase()));
            }

            if missing_only {
                tracks.retain(|t| t.lrc_lyrics.is_none() && t.txt_lyrics.is_none());
            }

            // Build context and render
            let context = build_context(tracks, None);
            let rendered = engine.render(&template_name, &context)?;

            // Output result
            if let Some(output_path) = output {
                fs::write(&output_path, &rendered)?;
                println!("✅ Template rendered to: {}", output_path);
                println!("📊 Processed {} tracks", context.tracks.len());
            } else {
                println!("{}", rendered);
            }
        },

        TemplatesCommands::Path => {
            println!("📄 Template configuration path:");
            println!("{}", templates_config_path.display());
            
            if templates_config_path.exists() {
                println!("✅ Configuration file exists");
            } else {
                println!("❌ Configuration file not found");
                println!("💡 Use 'lrcget templates init' to create it");
            }
        },

        TemplatesCommands::Test { template_name } => {
            if !templates_config_path.exists() {
                println!("❌ No template configuration found.");
                println!("💡 Use 'lrcget templates init' first.");
                return Ok(());
            }

            // Load templates
            let mut engine = TemplateEngine::new();
            engine.load_templates_from_config(&templates_config_path)?;

            // Check if template exists
            if engine.get_template(&template_name).is_none() {
                println!("❌ Template '{}' not found.", template_name);
                return Ok(());
            }

            // Create sample data for testing
            use crate::core::data::database::DatabaseTrack;
            let sample_tracks = vec![
                DatabaseTrack {
                    id: 1,
                    file_path: "/music/Artist1/Album1/Track1.mp3".to_string(),
                    file_name: "Track1.mp3".to_string(),
                    title: "Sample Song 1".to_string(),
                    album_name: "Test Album".to_string(),
                    artist_name: "Test Artist".to_string(),
                    album_artist: "Test Artist".to_string(),
                    duration: 180.5,
                    track_number: Some(1),
                    txt_lyrics: Some("Sample plain lyrics".to_string()),
                    lrc_lyrics: None,
                },
                DatabaseTrack {
                    id: 2,
                    file_path: "/music/Artist1/Album1/Track2.mp3".to_string(),
                    file_name: "Track2.mp3".to_string(),
                    title: "Sample Song 2".to_string(),
                    album_name: "Test Album".to_string(),
                    artist_name: "Test Artist".to_string(),
                    album_artist: "Test Artist".to_string(),
                    duration: 225.0,
                    track_number: Some(2),
                    txt_lyrics: None,
                    lrc_lyrics: Some("[00:10.00]Sample synced lyrics".to_string()),
                },
            ];

            let context = build_context(sample_tracks, None);
            let rendered = engine.render(&template_name, &context)?;

            println!("🧪 Template Test: {}", template_name);
            println!("═══════════════════════════════");
            println!("{}", rendered);
            println!("═══════════════════════════════");
            println!("✅ Template test completed with {} sample tracks", context.tracks.len());
        },
    }

    Ok(())
}
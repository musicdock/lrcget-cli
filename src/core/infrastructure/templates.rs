use anyhow::Result;
use handlebars::{Handlebars, Helper, Context, RenderContext, Output, HelperResult, RenderError, RenderErrorReason};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info};

use crate::core::data::database::DatabaseTrack;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub name: String,
    pub description: String,
    pub template: String,
    pub output_format: OutputFormat,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    Text,
    Json,
    Html,
    Markdown,
    Csv,
    Custom(String),
}

#[derive(Debug, Serialize)]
pub struct TemplateContext {
    pub tracks: Vec<DatabaseTrack>,
    pub metadata: HashMap<String, Value>,
    pub stats: TemplateStats,
    pub timestamp: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct TemplateStats {
    pub total_tracks: usize,
    pub tracks_with_synced_lyrics: usize,
    pub tracks_with_plain_lyrics: usize,
    pub tracks_with_any_lyrics: usize,
    pub tracks_missing_lyrics: usize,
    pub coverage_percentage: f64,
    pub unique_artists: usize,
    pub unique_albums: usize,
}

pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
    templates: HashMap<String, Template>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(false);
        
        // Register custom helpers
        handlebars.register_helper("format_duration", Box::new(format_duration));
        handlebars.register_helper("format_percentage", Box::new(format_percentage));
        handlebars.register_helper("truncate", Box::new(truncate));
        handlebars.register_helper("capitalize", Box::new(capitalize));
        handlebars.register_helper("escape_csv", Box::new(escape_csv));
        handlebars.register_helper("format_date", Box::new(format_date));
        
        Self {
            handlebars,
            templates: HashMap::new(),
        }
    }

    pub fn load_templates_from_config(&mut self, config_path: &PathBuf) -> Result<()> {
        if !config_path.exists() {
            debug!("Template configuration file not found: {}", config_path.display());
            return Ok(());
        }

        let content = std::fs::read_to_string(config_path)?;
        let template_config: TemplateConfig = toml::from_str(&content)?;

        for template in template_config.templates {
            if template.enabled {
                self.register_template(template)?;
            }
        }

        info!("Loaded {} templates from configuration", self.templates.len());
        Ok(())
    }

    pub fn register_template(&mut self, template: Template) -> Result<()> {
        self.handlebars.register_template_string(&template.name, &template.template)?;
        self.templates.insert(template.name.clone(), template);
        debug!("Registered template: {}", self.templates.len());
        Ok(())
    }

    pub fn render(&self, template_name: &str, context: &TemplateContext) -> Result<String> {
        match self.templates.get(template_name) {
            Some(_template) => {
                let output = self.handlebars.render(template_name, context)?;
                Ok(output)
            },
            None => {
                anyhow::bail!("Template not found: {}", template_name)
            }
        }
    }

    pub fn list_templates(&self) -> Vec<&Template> {
        self.templates.values().collect()
    }

    pub fn get_template(&self, name: &str) -> Option<&Template> {
        self.templates.get(name)
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

// Custom Handlebars helpers
fn format_duration(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let duration = h.param(0)
        .and_then(|v| v.value().as_f64())
        .ok_or_else(|| RenderError::from(RenderErrorReason::Other("Duration parameter required".to_string())))?;

    let hours = (duration as u64) / 3600;
    let minutes = ((duration as u64) % 3600) / 60;
    let seconds = (duration as u64) % 60;

    let formatted = if hours > 0 {
        format!("{}h {:02}m {:02}s", hours, minutes, seconds)
    } else {
        format!("{}m {:02}s", minutes, seconds)
    };

    out.write(&formatted)?;
    Ok(())
}

fn format_percentage(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let percentage = h.param(0)
        .and_then(|v| v.value().as_f64())
        .ok_or_else(|| RenderError::from(RenderErrorReason::Other("Percentage parameter required".to_string())))?;

    let precision = h.param(1)
        .and_then(|v| v.value().as_u64())
        .unwrap_or(1) as usize;

    out.write(&format!("{:.prec$}%", percentage, prec = precision))?;
    Ok(())
}

fn truncate(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let text = h.param(0)
        .and_then(|v| v.value().as_str())
        .ok_or_else(|| RenderError::from(RenderErrorReason::Other("Text parameter required".to_string())))?;

    let max_length = h.param(1)
        .and_then(|v| v.value().as_u64())
        .ok_or_else(|| RenderError::from(RenderErrorReason::Other("Max length parameter required".to_string())))? as usize;

    let truncated = if text.len() > max_length {
        format!("{}...", &text[..max_length.saturating_sub(3)])
    } else {
        text.to_string()
    };

    out.write(&truncated)?;
    Ok(())
}

fn capitalize(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let text = h.param(0)
        .and_then(|v| v.value().as_str())
        .ok_or_else(|| RenderError::from(RenderErrorReason::Other("Text parameter required".to_string())))?;

    let capitalized = text.chars()
        .enumerate()
        .map(|(i, c)| if i == 0 { c.to_uppercase().collect::<String>() } else { c.to_string() })
        .collect::<String>();

    out.write(&capitalized)?;
    Ok(())
}

fn escape_csv(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let text = h.param(0)
        .and_then(|v| v.value().as_str())
        .ok_or_else(|| RenderError::from(RenderErrorReason::Other("Text parameter required".to_string())))?;

    let escaped = if text.contains(',') || text.contains('"') || text.contains('\n') {
        format!("\"{}\"", text.replace("\"", "\"\""))
    } else {
        text.to_string()
    };

    out.write(&escaped)?;
    Ok(())
}

fn format_date(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let timestamp = h.param(0)
        .and_then(|v| v.value().as_str())
        .ok_or_else(|| RenderError::from(RenderErrorReason::Other("Timestamp parameter required".to_string())))?;

    let format = h.param(1)
        .and_then(|v| v.value().as_str())
        .unwrap_or("%Y-%m-%d %H:%M:%S");

    let parsed_date = chrono::DateTime::parse_from_rfc3339(timestamp);
    
    let formatted = match parsed_date {
        Ok(dt) => dt.format(format).to_string(),
        Err(_) => timestamp.to_string(), // Fallback to original if parsing fails
    };

    out.write(&formatted)?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct TemplateConfig {
    templates: Vec<Template>,
}

// Helper function to create sample template configuration
pub fn create_sample_template_config(config_path: &PathBuf) -> Result<()> {
    let sample_templates = vec![
        Template {
            name: "library_summary".to_string(),
            description: "Comprehensive library summary with statistics".to_string(),
            template: r#"📊 Music Library Summary
═══════════════════════════════════════

🎵 Library Statistics
─────────────────────
Total Tracks:     {{stats.total_tracks}}
Unique Artists:   {{stats.unique_artists}}
Unique Albums:    {{stats.unique_albums}}
Generated:        {{format_date timestamp "%Y-%m-%d %H:%M:%S UTC"}}
LRCGET Version:   {{version}}

📝 Lyrics Coverage
──────────────────
Synced Lyrics:    {{stats.tracks_with_synced_lyrics}} tracks
Plain Lyrics:     {{stats.tracks_with_plain_lyrics}} tracks
Any Lyrics:       {{stats.tracks_with_any_lyrics}} tracks
Missing Lyrics:   {{stats.tracks_missing_lyrics}} tracks
Coverage Rate:    {{format_percentage stats.coverage_percentage 1}}

💡 Recommendations
──────────────────
{{#if (lt stats.coverage_percentage 80)}}
🔍 Consider running: lrcget download --missing-lyrics
{{/if}}
{{#if (gt stats.tracks_missing_lyrics 100)}}
📋 For bulk processing: lrcget batch download missing_lyrics.json
{{/if}}
{{#if (eq stats.coverage_percentage 100)}}
🎉 Perfect! All your tracks have lyrics!
{{/if}}"#.to_string(),
            output_format: OutputFormat::Text,
            enabled: true,
        },
        Template {
            name: "track_list".to_string(),
            description: "Simple list of all tracks".to_string(),
            template: r#"🎵 Music Library Track List
═══════════════════════════

{{#each tracks}}
{{@index}} - {{artist_name}} - {{title}}
   Album: {{album_name}}
   Duration: {{format_duration duration}}
   Lyrics: {{#if lrc_lyrics}}🎼{{else}}  {{/if}} {{#if txt_lyrics}}📝{{else}}  {{/if}}
   {{#unless lrc_lyrics}}{{#unless txt_lyrics}}❌ Missing{{/unless}}{{/unless}}

{{/each}}

Total: {{stats.total_tracks}} tracks
Coverage: {{format_percentage stats.coverage_percentage 1}}"#.to_string(),
            output_format: OutputFormat::Text,
            enabled: true,
        },
        Template {
            name: "csv_export".to_string(),
            description: "CSV export format".to_string(),
            template: r#"Artist,Title,Album,Duration,Has Synced Lyrics,Has Plain Lyrics,File Path
{{#each tracks}}{{escape_csv artist_name}},{{escape_csv title}},{{escape_csv album_name}},{{duration}},{{#if lrc_lyrics}}true{{else}}false{{/if}},{{#if txt_lyrics}}true{{else}}false{{/if}},{{escape_csv file_path}}
{{/each}}"#.to_string(),
            output_format: OutputFormat::Csv,
            enabled: true,
        },
    ];

    let config = TemplateConfig { templates: sample_templates };
    let toml_content = toml::to_string_pretty(&config)?;
    std::fs::write(config_path, toml_content)?;
    
    info!("Sample template configuration created at: {}", config_path.display());
    Ok(())
}

// Helper to build template context from tracks
pub fn build_context(
    tracks: Vec<DatabaseTrack>, 
    metadata: Option<HashMap<String, Value>>
) -> TemplateContext {
    let stats = calculate_stats(&tracks);
    
    TemplateContext {
        tracks,
        metadata: metadata.unwrap_or_default(),
        stats,
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

fn calculate_stats(tracks: &[DatabaseTrack]) -> TemplateStats {
    let total = tracks.len();
    let synced_count = tracks.iter().filter(|t| t.lrc_lyrics.is_some()).count();
    let plain_count = tracks.iter().filter(|t| t.txt_lyrics.is_some()).count();
    let any_lyrics_count = tracks.iter()
        .filter(|t| t.lrc_lyrics.is_some() || t.txt_lyrics.is_some())
        .count();
    let missing_count = total - any_lyrics_count;
    
    let coverage = if total > 0 {
        (any_lyrics_count as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    let unique_artists = tracks.iter()
        .map(|t| &t.artist_name)
        .collect::<std::collections::HashSet<_>>()
        .len();
        
    let unique_albums = tracks.iter()
        .map(|t| &t.album_name)
        .collect::<std::collections::HashSet<_>>()
        .len();

    TemplateStats {
        total_tracks: total,
        tracks_with_synced_lyrics: synced_count,
        tracks_with_plain_lyrics: plain_count,
        tracks_with_any_lyrics: any_lyrics_count,
        tracks_missing_lyrics: missing_count,
        coverage_percentage: coverage,
        unique_artists,
        unique_albums,
    }
}
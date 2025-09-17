use anyhow::Result;
use std::fs::{remove_file, write};
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

use crate::core::database::DatabaseTrack;

pub struct LyricsManager;

impl LyricsManager {
    pub fn new() -> Self {
        LyricsManager
    }

    pub async fn save_lyrics_for_track(
        &self,
        track: &DatabaseTrack,
        plain_lyrics: Option<&str>,
        synced_lyrics: Option<&str>,
        is_instrumental: bool,
    ) -> Result<()> {
        let track_path = &track.file_path;
        
        if is_instrumental {
            self.save_instrumental(track_path)?;
        } else {
            if let Some(synced) = synced_lyrics {
                self.save_synced_lyrics(track_path, synced)?;
            } else if let Some(plain) = plain_lyrics {
                self.save_plain_lyrics(track_path, plain)?;
            }
        }

        debug!("Saved lyrics for: {}", track.title);
        Ok(())
    }

    pub async fn save_lyrics_for_file(
        &self,
        file_path: &str,
        plain_lyrics: Option<&str>,
        synced_lyrics: Option<&str>,
        is_instrumental: bool,
    ) -> Result<()> {
        if is_instrumental {
            self.save_instrumental(file_path)?;
        } else {
            if let Some(synced) = synced_lyrics {
                self.save_synced_lyrics(file_path, synced)?;
            } else if let Some(plain) = plain_lyrics {
                self.save_plain_lyrics(file_path, plain)?;
            }
        }

        debug!("Saved lyrics for file: {}", file_path);
        Ok(())
    }

    fn save_plain_lyrics(&self, track_path: &str, lyrics: &str) -> Result<()> {
        let txt_path = self.build_txt_path(track_path)?;
        let lrc_path = self.build_lrc_path(track_path)?;

        // Remove any existing LRC file
        let _ = remove_file(lrc_path);

        if lyrics.is_empty() {
            let _ = remove_file(txt_path);
        } else {
            write(txt_path, lyrics)?;
        }
        Ok(())
    }

    fn save_synced_lyrics(&self, track_path: &str, lyrics: &str) -> Result<()> {
        let txt_path = self.build_txt_path(track_path)?;
        let lrc_path = self.build_lrc_path(track_path)?;
        
        if lyrics.is_empty() {
            let _ = remove_file(lrc_path);
        } else {
            // Remove any existing TXT file when we have synced lyrics
            let _ = remove_file(txt_path);
            write(lrc_path, lyrics)?;
        }
        Ok(())
    }

    pub fn save_instrumental(&self, track_path: &str) -> Result<()> {
        let txt_path = self.build_txt_path(track_path)?;
        let lrc_path = self.build_lrc_path(track_path)?;

        let _ = remove_file(&txt_path);
        let _ = remove_file(&lrc_path);

        write(lrc_path, "[au: instrumental]")?;
        Ok(())
    }

    fn build_txt_path(&self, track_path: &str) -> Result<PathBuf> {
        let path = Path::new(track_path);
        let parent_path = path.parent().ok_or_else(|| anyhow::anyhow!(
            "Invalid track path (no parent directory): {}", track_path
        ))?;
        let stem = path.file_stem().ok_or_else(|| anyhow::anyhow!(
            "Invalid track path (no file name): {}", track_path
        ))?;
        let file_name_without_extension = stem.to_str().ok_or_else(|| anyhow::anyhow!(
            "Track path is not valid UTF-8: {}", track_path
        ))?;
        let txt_path =
            Path::new(parent_path).join(format!("{}.{}", file_name_without_extension, "txt"));

        Ok(txt_path)
    }

    fn build_lrc_path(&self, track_path: &str) -> Result<PathBuf> {
        let path = Path::new(track_path);
        let parent_path = path.parent().ok_or_else(|| anyhow::anyhow!(
            "Invalid track path (no parent directory): {}", track_path
        ))?;
        let stem = path.file_stem().ok_or_else(|| anyhow::anyhow!(
            "Invalid track path (no file name): {}", track_path
        ))?;
        let file_name_without_extension = stem.to_str().ok_or_else(|| anyhow::anyhow!(
            "Track path is not valid UTF-8: {}", track_path
        ))?;
        let lrc_path =
            Path::new(parent_path).join(format!("{}.{}", file_name_without_extension, "lrc"));

        Ok(lrc_path)
    }

    // Future: Add embedding functionality for MP3/FLAC files
    pub fn embed_lyrics(&self, track_path: &str, plain_lyrics: &str, synced_lyrics: &str) -> Result<()> {
        if track_path.to_lowercase().ends_with(".mp3") {
            self.embed_lyrics_mp3(track_path, plain_lyrics, synced_lyrics)
        } else if track_path.to_lowercase().ends_with(".flac") {
            self.embed_lyrics_flac(track_path, plain_lyrics, synced_lyrics)
        } else {
            warn!("Embedding not supported for file type: {}", track_path);
            Ok(())
        }
    }

    fn embed_lyrics_mp3(&self, _track_path: &str, _plain_lyrics: &str, _synced_lyrics: &str) -> Result<()> {
        // TODO: Implement MP3 ID3v2 embedding
        warn!("MP3 embedding not yet implemented");
        Ok(())
    }

    fn embed_lyrics_flac(&self, _track_path: &str, _plain_lyrics: &str, _synced_lyrics: &str) -> Result<()> {
        // TODO: Implement FLAC Vorbis comments embedding
        warn!("FLAC embedding not yet implemented");
        Ok(())
    }
}
use anyhow::Result;
use globwalk::{glob, DirEntry};
use lofty::error::LoftyError;
use lofty::file::AudioFile;
use lofty::file::TaggedFileExt;
use lofty::properties::FileProperties;
use lofty::read_from_path;
use lofty::tag::Accessor;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;
use tracing::{debug, warn};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Track {
    pub file_path: String,
    pub file_name: String,
    pub title: String,
    pub album: String,
    pub artist: String,
    pub album_artist: String,
    pub duration: f64,
    pub txt_lyrics: Option<String>,
    pub lrc_lyrics: Option<String>,
    pub track_number: Option<u32>,
}

#[derive(Error, Debug)]
pub enum TrackError {
    #[error("Cannot parse the tag info from track: `{0}`. Error: `{1}`")]
    ParseFailed(String, LoftyError),
    #[error("No title was found from track: `{0}`")]
    TitleNotFound(String),
    #[error("No album name was found from track: `{0}`")]
    AlbumNotFound(String),
    #[error("No artist name was found from track: `{0}`")]
    ArtistNotFound(String),
    #[error("No primary tag was found from track: `{0}`")]
    PrimaryTagNotFound(String),
}

impl Track {
    fn new(
        file_path: String,
        file_name: String,
        title: String,
        album: String,
        artist: String,
        album_artist: String,
        duration: f64,
        txt_lyrics: Option<String>,
        lrc_lyrics: Option<String>,
        track_number: Option<u32>,
    ) -> Track {
        Track {
            file_path,
            file_name,
            title,
            album,
            artist,
            album_artist,
            duration,
            txt_lyrics,
            lrc_lyrics,
            track_number,
        }
    }

    pub fn new_from_path(path: &Path) -> Result<Track> {
        let file_path = path.display().to_string();
        // Avoid panics on non-UTF8 or edge-case paths by using lossy conversion and a safe fallback
        let file_name = path
            .file_name()
            .map(|os| os.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        let tagged_file = read_from_path(&file_path)
            .map_err(|err| TrackError::ParseFailed(file_path.to_owned(), err))?;
        let tag = tagged_file
            .primary_tag()
            .ok_or(TrackError::PrimaryTagNotFound(file_path.to_owned()))?
            .to_owned();
        let properties = tagged_file.properties();
        let title = tag
            .title()
            .ok_or(TrackError::TitleNotFound(file_path.to_owned()))?
            .to_string();
        let album = tag
            .album()
            .ok_or(TrackError::AlbumNotFound(file_path.to_owned()))?
            .to_string();
        let artist = tag
            .artist()
            .ok_or(TrackError::ArtistNotFound(file_path.to_owned()))?
            .to_string();
        let album_artist = tag
            .get_string(&lofty::tag::ItemKey::AlbumArtist)
            .map(|s| s.to_string())
            .unwrap_or_else(|| artist.clone());
        let duration = extract_duration(&file_path, properties);
        let track_number = tag.track();

        let mut track = Track::new(
            file_path,
            file_name,
            title,
            album,
            artist,
            album_artist,
            duration,
            None,
            None,
            track_number,
        );
        track.txt_lyrics = track.get_txt_lyrics();
        track.lrc_lyrics = track.get_lrc_lyrics();

        Ok(track)
    }

    fn get_txt_path(&self) -> String {
        // Build path safely without unwraps; just change extension to .txt
        let mut path = PathBuf::from(&self.file_path);
        path.set_extension("txt");
        path.to_string_lossy().into_owned()
    }

    fn get_txt_lyrics(&self) -> Option<String> {
        let txt_file_path = self.get_txt_path();
        match std::fs::read_to_string(txt_file_path) {
            Ok(content) => Some(content),
            Err(_) => None,
        }
    }

    fn get_lrc_path(&self) -> String {
        // Build path safely without unwraps; just change extension to .lrc
        let mut path = PathBuf::from(&self.file_path);
        path.set_extension("lrc");
        path.to_string_lossy().into_owned()
    }

    fn get_lrc_lyrics(&self) -> Option<String> {
        let lrc_file_path = self.get_lrc_path();
        match std::fs::read_to_string(lrc_file_path) {
            Ok(content) => Some(content),
            Err(_) => None,
        }
    }
}

pub struct Scanner;

impl Scanner {
    pub fn new() -> Self {
        Scanner
    }

    pub async fn scan_directory(&self, directory: &Path, extensions: &Option<Vec<String>>) -> Result<Vec<Track>> {
        let directory_str = directory.to_string_lossy();
        debug!("Scanning directory: {}", directory_str);

        let extensions_pattern = match extensions {
            Some(exts) => {
                let ext_list: Vec<String> = exts.iter()
                    .flat_map(|ext| vec![ext.to_lowercase(), ext.to_uppercase()])
                    .collect();
                ext_list.join(",")
            }
            None => "mp3,m4a,flac,ogg,opus,wav,MP3,M4A,FLAC,OGG,OPUS,WAV".to_string(),
        };

        let pattern = format!(
            "{}/**/*.{{{}}}",
            directory_str, extensions_pattern
        );

        let globwalker = glob(&pattern)?;
        let entries: Vec<DirEntry> = globwalker.collect::<Result<Vec<_>, _>>()?;

        debug!("Found {} audio files", entries.len());

        let tracks = self.load_tracks_from_entries(&entries)?;
        Ok(tracks)
    }

    fn load_tracks_from_entries(&self, entries: &[DirEntry]) -> Result<Vec<Track>> {
        let track_results: Vec<Result<Track>> = entries
            .par_iter()
            .map(|file| Track::new_from_path(file.path()))
            .collect();

        let mut tracks: Vec<Track> = vec![];

        for track_result in track_results {
            match track_result {
                Ok(track) => {
                    tracks.push(track);
                }
                Err(error) => {
                    warn!("Failed to process track: {}", error);
                }
            }
        }

        Ok(tracks)
    }

    pub async fn scan_file(&self, file_path: &Path) -> Result<Option<Track>> {
        if !file_path.exists() || !file_path.is_file() {
            return Ok(None);
        }

        match Track::new_from_path(file_path) {
            Ok(track) => Ok(Some(track)),
            Err(e) => {
                debug!("Failed to scan file {}: {}", file_path.display(), e);
                Ok(None)
            }
        }
    }
}

fn extract_duration(file_path: &str, properties: &FileProperties) -> f64 {
    // First try the standard lofty approach
    let lofty_duration = properties.duration().as_secs_f64();

    // If lofty gives us a reasonable duration (> 0), use it
    if lofty_duration > 0.0 {
        debug!("Extracted duration using lofty: {:.3}s", lofty_duration);
        return lofty_duration;
    }

    // If lofty fails (returns 0 or negative), try alternative methods
    warn!("lofty failed to extract duration (got {:.3}s), trying alternative methods for: {}", lofty_duration, file_path);

    // Try using ffprobe as fallback
    if let Some(ffprobe_duration) = extract_duration_with_ffprobe(file_path) {
        debug!("Extracted duration using ffprobe: {:.3}s", ffprobe_duration);
        return ffprobe_duration;
    }

    // Try using MediaInfo as another fallback
    if let Some(mediainfo_duration) = extract_duration_with_mediainfo(file_path) {
        debug!("Extracted duration using mediainfo: {:.3}s", mediainfo_duration);
        return mediainfo_duration;
    }

    // If all methods fail, warn and return the original lofty value
    warn!("All duration extraction methods failed for: {}, using fallback value: {:.3}s", file_path, lofty_duration);
    lofty_duration
}

fn extract_duration_with_ffprobe(file_path: &str) -> Option<f64> {
    match Command::new("ffprobe")
        .arg("-i")
        .arg(file_path)
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-v")
        .arg("quiet")
        .arg("-of")
        .arg("csv=p=0")
        .output()
    {
        Ok(output) if output.status.success() => {
            let duration_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            match duration_str.parse::<f64>() {
                Ok(duration) if duration > 0.0 => {
                    debug!("ffprobe extracted duration: {:.3}s from {}", duration, file_path);
                    Some(duration)
                }
                Ok(duration) => {
                    warn!("ffprobe returned invalid duration: {:.3}s for {}", duration, file_path);
                    None
                }
                Err(e) => {
                    warn!("Failed to parse ffprobe duration output '{}' for {}: {}", duration_str, file_path, e);
                    None
                }
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            debug!("ffprobe failed for {}: {}", file_path, stderr);
            None
        }
        Err(e) => {
            debug!("ffprobe command not available or failed for {}: {}", file_path, e);
            None
        }
    }
}

fn extract_duration_with_mediainfo(file_path: &str) -> Option<f64> {
    match Command::new("mediainfo")
        .arg("--Inform=General;%Duration%")
        .arg(file_path)
        .output()
    {
        Ok(output) if output.status.success() => {
            let duration_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            match duration_str.parse::<u64>() {
                Ok(duration_ms) if duration_ms > 0 => {
                    let duration_seconds = duration_ms as f64 / 1000.0;
                    debug!("mediainfo extracted duration: {:.3}s from {}", duration_seconds, file_path);
                    Some(duration_seconds)
                }
                Ok(duration_ms) => {
                    warn!("mediainfo returned invalid duration: {}ms for {}", duration_ms, file_path);
                    None
                }
                Err(e) => {
                    warn!("Failed to parse mediainfo duration output '{}' for {}: {}", duration_str, file_path, e);
                    None
                }
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            debug!("mediainfo failed for {}: {}", file_path, stderr);
            None
        }
        Err(e) => {
            debug!("mediainfo command not available or failed for {}: {}", file_path, e);
            None
        }
    }
}

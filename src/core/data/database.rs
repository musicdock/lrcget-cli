//! SQLite database operations for local music library
//!
//! This module handles all database operations for storing and retrieving
//! music track information, including metadata and lyrics storage paths.

use anyhow::Result;
use rusqlite::{Connection, params};
use serde::Serialize;
use std::path::Path;
use tracing::{debug, info};

use crate::core::files::scanner::Track;

const CURRENT_DB_VERSION: u32 = 1;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub async fn new(db_path: &Path) -> Result<Self> {
        info!("Opening database at: {}", db_path.display());
        
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut conn = Connection::open(db_path)?;
        
        // Enable WAL mode for better concurrent access
        conn.pragma_update(None, "journal_mode", "WAL")?;
        
        // Check and upgrade database if needed
        let mut user_pragma = conn.prepare("PRAGMA user_version")?;
        let existing_user_version: u32 = user_pragma.query_row([], |row| Ok(row.get(0)?))?;
        drop(user_pragma);

        if existing_user_version < CURRENT_DB_VERSION {
            Self::upgrade_database(&mut conn, existing_user_version)?;
        }

        Ok(Database { conn })
    }

    fn upgrade_database(conn: &mut Connection, existing_version: u32) -> Result<()> {
        debug!("Upgrading database from version {} to {}", existing_version, CURRENT_DB_VERSION);

        if existing_version == 0 {
            let tx = conn.transaction()?;

            tx.pragma_update(None, "user_version", CURRENT_DB_VERSION)?;

            tx.execute_batch(r#"
                CREATE TABLE directories (
                    id INTEGER PRIMARY KEY,
                    path TEXT UNIQUE
                );

                CREATE TABLE config (
                    id INTEGER PRIMARY KEY,
                    initialized BOOLEAN DEFAULT FALSE
                );

                CREATE TABLE tracks (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    file_path TEXT UNIQUE,
                    file_name TEXT,
                    title TEXT,
                    album TEXT,
                    artist TEXT,
                    album_artist TEXT,
                    duration REAL,
                    track_number INTEGER,
                    txt_lyrics TEXT,
                    lrc_lyrics TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
                );

                CREATE INDEX idx_tracks_artist ON tracks(artist);
                CREATE INDEX idx_tracks_album ON tracks(album);
                CREATE INDEX idx_tracks_title ON tracks(title);

                INSERT INTO config (initialized) VALUES (FALSE);
            "#)?;

            tx.commit()?;
        }

        info!("Database upgraded successfully");
        Ok(())
    }

    pub async fn add_directory(&mut self, directory: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO directories (path) VALUES (?1)",
            params![directory],
        )?;
        Ok(())
    }

    pub async fn get_directories(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT path FROM directories ORDER BY path")?;
        let dirs = stmt.query_map([], |row| {
            Ok(row.get::<_, String>(0)?)
        })?
        .collect::<Result<Vec<String>, _>>()?;
        
        Ok(dirs)
    }

    pub async fn initialize_library(&mut self) -> Result<()> {
        self.conn.execute(
            "UPDATE config SET initialized = TRUE WHERE id = 1",
            [],
        )?;
        Ok(())
    }

    pub async fn clear_tracks(&mut self) -> Result<()> {
        self.conn.execute("DELETE FROM tracks", [])?;
        Ok(())
    }

    pub async fn add_track(&mut self, track: &Track) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO tracks 
            (file_path, file_name, title, album, artist, album_artist, duration, track_number, txt_lyrics, lrc_lyrics, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, CURRENT_TIMESTAMP)
            "#,
            params![
                track.file_path,
                track.file_name,
                track.title,
                track.album,
                track.artist,
                track.album_artist,
                track.duration,
                track.track_number,
                track.txt_lyrics,
                track.lrc_lyrics,
            ],
        )?;
        Ok(())
    }

    pub async fn get_track(&self, id: i64) -> Result<DatabaseTrack> {
        let mut stmt = self.conn.prepare(r#"
            SELECT id, file_path, file_name, title, album, artist, album_artist, 
                   duration, track_number, txt_lyrics, lrc_lyrics
            FROM tracks WHERE id = ?1
        "#)?;
        
        let track = stmt.query_row(params![id], |row| {
            Ok(DatabaseTrack {
                id: row.get(0)?,
                file_path: row.get(1)?,
                file_name: row.get(2)?,
                title: row.get(3)?,
                album_name: row.get(4)?,
                artist_name: row.get(5)?,
                album_artist: row.get(6)?,
                duration: row.get(7)?,
                track_number: row.get(8)?,
                txt_lyrics: row.get(9)?,
                lrc_lyrics: row.get(10)?,
            })
        })?;

        Ok(track)
    }

    pub async fn get_all_tracks(&self) -> Result<Vec<DatabaseTrack>> {
        let mut stmt = self.conn.prepare(r#"
            SELECT id, file_path, file_name, title, album, artist, album_artist,
                   duration, track_number, txt_lyrics, lrc_lyrics
            FROM tracks 
            ORDER BY artist, album, track_number, title
        "#)?;

        let tracks = stmt.query_map([], |row| {
            Ok(DatabaseTrack {
                id: row.get(0)?,
                file_path: row.get(1)?,
                file_name: row.get(2)?,
                title: row.get(3)?,
                album_name: row.get(4)?,
                artist_name: row.get(5)?,
                album_artist: row.get(6)?,
                duration: row.get(7)?,
                track_number: row.get(8)?,
                txt_lyrics: row.get(9)?,
                lrc_lyrics: row.get(10)?,
            })
        })?
        .collect::<Result<Vec<DatabaseTrack>, _>>()?;

        Ok(tracks)
    }

    pub async fn track_exists(&self, file_path: &std::path::Path) -> Result<bool> {
        let mut stmt = self.conn.prepare("SELECT 1 FROM tracks WHERE file_path = ?1")?;
        let result = stmt.query_row(params![file_path.to_string_lossy()], |_| Ok(()));
        
        match result {
            Ok(_) => Ok(true),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn get_track_by_path(&self, file_path: &str) -> Result<DatabaseTrack> {
        let mut stmt = self.conn.prepare(r#"
            SELECT id, file_path, file_name, title, album, artist, album_artist,
                   duration, track_number, txt_lyrics, lrc_lyrics
            FROM tracks 
            WHERE file_path = ?1
        "#)?;
        
        let track = stmt.query_row(params![file_path], |row| {
            Ok(DatabaseTrack {
                id: row.get(0)?,
                file_path: row.get(1)?,
                file_name: row.get(2)?,
                title: row.get(3)?,
                album_name: row.get(4)?,
                artist_name: row.get(5)?,
                album_artist: row.get(6)?,
                duration: row.get(7)?,
                track_number: row.get(8)?,
                txt_lyrics: row.get(9)?,
                lrc_lyrics: row.get(10)?,
            })
        })?;

        Ok(track)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DatabaseTrack {
    pub id: i64,
    pub file_path: String,
    pub file_name: String,
    pub title: String,
    pub album_name: String,
    pub artist_name: String,
    pub album_artist: String,
    pub duration: f64,
    pub track_number: Option<i64>,
    pub txt_lyrics: Option<String>,
    pub lrc_lyrics: Option<String>,
}
use anyhow::Result;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, info};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use crate::core::services::lrclib::SearchResult;

#[derive(Debug)]
pub struct LrclibDatabase {
    conn: Connection,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LrclibTrack {
    pub id: i64,
    pub name: Option<String>,
    pub artist_name: Option<String>,
    pub album_name: Option<String>,
    pub duration: Option<f64>,
    pub plain_lyrics: Option<String>,
    pub synced_lyrics: Option<String>,
    pub instrumental: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct DatabaseStats {
    pub total_tracks: i64,
    pub unique_artists: i64,
    pub unique_albums: i64,
    pub synced_tracks: i64,
    pub plain_tracks: i64,
    pub instrumental_tracks: i64,
}

impl LrclibDatabase {
    pub async fn new(db_path: &Path) -> Result<Self> {
        debug!("Creating database connection for {:?}", db_path);
        let conn = Connection::open(db_path)?;

        // Enable WAL mode for better performance - allows multiple concurrent readers
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "cache_size", 10000)?;
        conn.pragma_update(None, "temp_store", "MEMORY")?;

        // Enable shared cache to improve performance across connections
        conn.pragma_update(None, "cache", "shared")?;

        Ok(LrclibDatabase { conn })
    }
    
    pub async fn create_schema(&self) -> Result<()> {
        info!("Creating LRCLIB database schema...");
        
        // Use the exact schema from initial.sql
        self.conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS tracks (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              name TEXT,
              name_lower TEXT,
              artist_name TEXT,
              artist_name_lower TEXT,
              album_name TEXT,
              album_name_lower TEXT,
              duration FLOAT,
              last_lyrics_id INTEGER,
              created_at DATETIME,
              updated_at DATETIME,
              FOREIGN KEY (last_lyrics_id) REFERENCES lyrics (id),
              UNIQUE(name_lower, artist_name_lower, album_name_lower, duration)
            );

            CREATE TABLE IF NOT EXISTS missing_tracks (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              name TEXT,
              name_lower TEXT,
              artist_name TEXT,
              artist_name_lower TEXT,
              album_name TEXT,
              album_name_lower TEXT,
              duration FLOAT,
              created_at DATETIME,
              updated_at DATETIME,
              UNIQUE(name_lower, artist_name_lower, album_name_lower, duration)
            );

            CREATE TABLE IF NOT EXISTS lyrics (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              plain_lyrics TEXT,
              synced_lyrics TEXT,
              track_id INTEGER,
              has_plain_lyrics BOOLEAN,
              has_synced_lyrics BOOLEAN,
              instrumental BOOLEAN,
              source TEXT,
              created_at DATETIME,
              updated_at DATETIME,
              FOREIGN KEY (track_id) REFERENCES tracks (id)
            );

            CREATE TABLE IF NOT EXISTS flags (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              lyrics_id INTEGER,
              content TEXT,
              created_at DATETIME,
              FOREIGN KEY (lyrics_id) REFERENCES lyrics (id)
            );

            -- Indexes for fast searching
            CREATE INDEX IF NOT EXISTS idx_missing_tracks_name_lower ON missing_tracks (name_lower);
            CREATE INDEX IF NOT EXISTS idx_missing_tracks_artist_name_lower ON missing_tracks (artist_name_lower);
            CREATE INDEX IF NOT EXISTS idx_missing_tracks_album_name_lower ON missing_tracks (album_name_lower);
            CREATE INDEX IF NOT EXISTS idx_missing_tracks_duration ON missing_tracks (duration);
            CREATE INDEX IF NOT EXISTS idx_missing_tracks_created_at ON missing_tracks (created_at);

            CREATE INDEX IF NOT EXISTS idx_lyrics_created_at ON lyrics (created_at);
            CREATE INDEX IF NOT EXISTS idx_tracks_name_lower ON tracks (name_lower);
            CREATE INDEX IF NOT EXISTS idx_tracks_artist_name_lower ON tracks (artist_name_lower);
            CREATE INDEX IF NOT EXISTS idx_tracks_album_name_lower ON tracks (album_name_lower);
            CREATE INDEX IF NOT EXISTS idx_tracks_duration ON tracks (duration);
            CREATE INDEX IF NOT EXISTS idx_tracks_last_lyrics_id ON tracks (last_lyrics_id);
            CREATE INDEX IF NOT EXISTS idx_lyrics_track_id ON lyrics (track_id);
            CREATE INDEX IF NOT EXISTS idx_lyrics_has_plain_lyrics ON lyrics (has_plain_lyrics);
            CREATE INDEX IF NOT EXISTS idx_lyrics_has_synced_lyrics ON lyrics (has_synced_lyrics);
            CREATE INDEX IF NOT EXISTS idx_lyrics_source ON lyrics (source);

            -- Full-text search
            CREATE VIRTUAL TABLE IF NOT EXISTS tracks_fts USING fts5(
              name_lower,
              album_name_lower,
              artist_name_lower,
              content='tracks',
              content_rowid='id'
            );

            -- Triggers for automatic last_lyrics_id update
            CREATE TRIGGER IF NOT EXISTS set_tracks_last_lyrics_id
            AFTER INSERT ON lyrics
            BEGIN
              UPDATE tracks SET last_lyrics_id = NEW.id WHERE tracks.id = NEW.track_id;
            END;

            -- FTS triggers
            CREATE TRIGGER IF NOT EXISTS tracks_ai AFTER INSERT ON tracks
            BEGIN
              INSERT INTO tracks_fts (rowid, name_lower, album_name_lower, artist_name_lower)
              VALUES (new.id, new.name_lower, new.album_name_lower, new.artist_name_lower);
            END;

            CREATE TRIGGER IF NOT EXISTS tracks_au AFTER UPDATE ON tracks
            BEGIN
              INSERT INTO tracks_fts(tracks_fts, rowid, name_lower, album_name_lower, artist_name_lower)
              VALUES('delete', old.id, old.name_lower, old.album_name_lower, old.artist_name_lower);
              INSERT INTO tracks_fts (rowid, name_lower, album_name_lower, artist_name_lower)
              VALUES (new.id, new.name_lower, new.album_name_lower, new.artist_name_lower);
            END;

            CREATE TRIGGER IF NOT EXISTS tracks_ad AFTER DELETE ON tracks
            BEGIN
              INSERT INTO tracks_fts(tracks_fts, rowid, name_lower, album_name_lower, artist_name_lower)
              VALUES('delete', old.id, old.name_lower, old.album_name_lower, old.artist_name_lower);
            END;
            
            -- Metadata table for versioning
            CREATE TABLE IF NOT EXISTS lrclib_metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
        "#)?;
        
        // Insert metadata about the database
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR REPLACE INTO lrclib_metadata (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params!["schema_version", "1", now],
        )?;
        
        Ok(())
    }
    
    pub async fn search_exact(
        &self,
        title: &str,
        artist: &str,
        album: &str,
        duration: f64,
    ) -> Result<Option<SearchResult>> {
        debug!("Searching local LRCLIB database for exact match: {} - {} ({})", artist, title, album);
        
        // Try exact match first using pre-computed lowercase columns for better performance
        let mut stmt = self.conn.prepare(r#"
            SELECT
                t.id, t.name, t.artist_name, t.album_name, t.duration,
                l.plain_lyrics, l.synced_lyrics, l.instrumental
            FROM tracks t
            LEFT JOIN lyrics l ON t.last_lyrics_id = l.id
            WHERE t.name_lower = LOWER(?1)
              AND t.artist_name_lower = LOWER(?2)
              AND t.album_name_lower = LOWER(?3)
              AND ABS(t.duration - ?4) <= 5.0
            ORDER BY ABS(t.duration - ?4)
            LIMIT 1
        "#)?;
        
        let result = stmt.query_row(params![title, artist, album, duration], |row| {
            Ok(SearchResult {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                artist_name: row.get(2)?,
                album_name: row.get(3)?,
                duration: row.get(4)?,
                plain_lyrics: row.get(5)?,
                synced_lyrics: row.get(6)?,
                instrumental: row.get::<_, Option<bool>>(7)?.unwrap_or(false),
                source: crate::core::services::lrclib::SearchResultSource::LocalDb,
            })
        });
        
        match result {
            Ok(track) => {
                debug!("Found exact match in local database");
                Ok(Some(track))
            },
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                debug!("No exact match found in local database");
                Ok(None)
            },
            Err(e) => Err(e.into()),
        }
    }
    
    pub async fn search(
        &self,
        title: &str,
        artist: &str,
        album: &str,
        query: &str,
    ) -> Result<Vec<SearchResult>> {
        debug!("Searching local LRCLIB database: title='{}', artist='{}', album='{}', query='{}'", 
               title, artist, album, query);
        
        let mut conditions = Vec::new();
        let mut params_vec = Vec::new();
        
        if !title.is_empty() {
            conditions.push("t.name_lower LIKE LOWER(?)");
            params_vec.push(format!("%{}%", title.to_lowercase()));
        }

        if !artist.is_empty() {
            conditions.push("t.artist_name_lower LIKE LOWER(?)");
            params_vec.push(format!("%{}%", artist.to_lowercase()));
        }

        if !album.is_empty() {
            conditions.push("t.album_name_lower LIKE LOWER(?)");
            params_vec.push(format!("%{}%", album.to_lowercase()));
        }

        if !query.is_empty() {
            let query_lower = query.to_lowercase();
            conditions.push("(t.name_lower LIKE ? OR t.artist_name_lower LIKE ? OR t.album_name_lower LIKE ?)");
            params_vec.push(format!("%{}%", query_lower));
            params_vec.push(format!("%{}%", query_lower));
            params_vec.push(format!("%{}%", query_lower));
        }
        
        if conditions.is_empty() {
            return Ok(Vec::new());
        }
        
        let where_clause = conditions.join(" AND ");
        let title_lower = title.to_lowercase().replace("'", "''");
        let artist_lower = artist.to_lowercase().replace("'", "''");

        let sql = format!(r#"
            SELECT
                t.id, t.name, t.artist_name, t.album_name, t.duration,
                l.plain_lyrics, l.synced_lyrics, l.instrumental
            FROM tracks t
            LEFT JOIN lyrics l ON t.last_lyrics_id = l.id
            WHERE {}
            ORDER BY
                -- Exact title match first (using pre-computed lowercase)
                CASE WHEN t.name_lower = '{}' THEN 0 ELSE 1 END,
                -- Exact artist match second (using pre-computed lowercase)
                CASE WHEN t.artist_name_lower = '{}' THEN 0 ELSE 1 END,
                -- Then by popularity (using ID as proxy)
                t.id DESC
            LIMIT 50
        "#, where_clause, title_lower, artist_lower);
        
        let mut stmt = self.conn.prepare(&sql)?;
        
        // Convert Vec<String> to Vec<&dyn rusqlite::ToSql>
        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();
        
        let tracks = stmt.query_map(params_refs.as_slice(), |row| {
            Ok(SearchResult {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                artist_name: row.get(2)?,
                album_name: row.get(3)?,
                duration: row.get(4)?,
                plain_lyrics: row.get(5)?,
                synced_lyrics: row.get(6)?,
                instrumental: row.get::<_, Option<bool>>(7)?.unwrap_or(false),
                source: crate::core::services::lrclib::SearchResultSource::LocalDb,
            })
        })?
        .collect::<Result<Vec<SearchResult>, _>>()?;
        
        debug!("Found {} exact results in local database", tracks.len());
        
        // If no exact results found, try fuzzy search
        if tracks.is_empty() && (!title.is_empty() || !artist.is_empty() || !album.is_empty() || !query.is_empty()) {
            debug!("No exact matches found, trying fuzzy search");
            return self.fuzzy_search(title, artist, album, query, Some(20)).await;
        }
        
        Ok(tracks)
    }
    
    pub async fn insert_track(&self, track: &LrclibTrack) -> Result<()> {
        // Start a transaction for consistent data insertion
        let tx = self.conn.unchecked_transaction()?;
        
        // Insert or update track record
        let track_id = tx.query_row(r#"
            INSERT INTO tracks 
            (name, name_lower, artist_name, artist_name_lower, album_name, album_name_lower, duration, created_at, updated_at)
            VALUES (?1, LOWER(?1), ?2, LOWER(?2), ?3, LOWER(?3), ?4, ?5, ?6)
            ON CONFLICT(name_lower, artist_name_lower, album_name_lower, duration) 
            DO UPDATE SET updated_at = ?6
            RETURNING id
        "#, params![
            track.name,
            track.artist_name,
            track.album_name,
            track.duration,
            track.created_at,
            track.updated_at,
        ], |row| row.get::<_, i64>(0))?;
        
        // Insert lyrics if we have any
        if track.plain_lyrics.is_some() || track.synced_lyrics.is_some() || track.instrumental {
            tx.execute(r#"
                INSERT INTO lyrics 
                (plain_lyrics, synced_lyrics, track_id, has_plain_lyrics, has_synced_lyrics, instrumental, source, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'lrclib', ?7, ?8)
            "#, params![
                track.plain_lyrics,
                track.synced_lyrics,
                track_id,
                track.plain_lyrics.is_some(),
                track.synced_lyrics.is_some(),
                track.instrumental,
                track.created_at,
                track.updated_at,
            ])?;
        }
        
        tx.commit()?;
        Ok(())
    }
    
    pub async fn fuzzy_search(
        &self,
        title: &str,
        artist: &str,
        album: &str,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<SearchResult>> {
        debug!("Fuzzy searching local LRCLIB database: title='{}', artist='{}', album='{}', query='{}'", 
               title, artist, album, query);
        
        let limit = limit.unwrap_or(50);
        let matcher = SkimMatcherV2::default();
        
        // Get all tracks from the database
        let mut stmt = self.conn.prepare(r#"
            SELECT 
                t.id,
                t.name,
                t.artist_name,
                t.album_name,
                t.duration,
                l.plain_lyrics,
                l.synced_lyrics,
                l.instrumental
            FROM tracks t
            LEFT JOIN lyrics l ON t.last_lyrics_id = l.id
            WHERE (t.name IS NOT NULL AND t.name != '') 
               OR (t.artist_name IS NOT NULL AND t.artist_name != '')
               OR (t.album_name IS NOT NULL AND t.album_name != '')
            ORDER BY t.id
            LIMIT 10000
        "#)?;
        
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<f64>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<bool>>(7)?,
            ))
        })?;
        
        let mut candidates = Vec::new();
        
        for row in rows {
            let (id, name, artist_name, album_name, duration, plain_lyrics, synced_lyrics, instrumental) = row?;
            
            let track_name = name.unwrap_or_default();
            let track_artist = artist_name.unwrap_or_default();
            let track_album = album_name.unwrap_or_default();
            
            let mut score = 0i64;
            let mut matches = 0;
            
            // Calculate fuzzy match scores
            if !title.is_empty() && !track_name.is_empty() {
                if let Some(s) = matcher.fuzzy_match(&track_name, title) {
                    score += s * 3; // Weight title matches higher
                    matches += 1;
                }
            }
            
            if !artist.is_empty() && !track_artist.is_empty() {
                if let Some(s) = matcher.fuzzy_match(&track_artist, artist) {
                    score += s * 2; // Weight artist matches medium
                    matches += 1;
                }
            }
            
            if !album.is_empty() && !track_album.is_empty() {
                if let Some(s) = matcher.fuzzy_match(&track_album, album) {
                    score += s; // Weight album matches lower
                    matches += 1;
                }
            }
            
            if !query.is_empty() {
                // Search query in all fields
                let combined = format!("{} {} {}", track_name, track_artist, track_album);
                if let Some(s) = matcher.fuzzy_match(&combined, query) {
                    score += s * 2;
                    matches += 1;
                }
            }
            
            // Only include results with some matches
            if matches > 0 && score > 30 { // Minimum threshold
                candidates.push((score, SearchResult {
                    id,
                    name: if track_name.is_empty() { None } else { Some(track_name) },
                    artist_name: if track_artist.is_empty() { None } else { Some(track_artist) },
                    album_name: if track_album.is_empty() { None } else { Some(track_album) },
                    duration,
                    instrumental: instrumental.unwrap_or(false),
                    plain_lyrics,
                    synced_lyrics,
                    source: crate::core::services::lrclib::SearchResultSource::LocalDb,
                }));
            }
        }
        
        // Sort by score (highest first) and return top results
        candidates.sort_by(|a, b| b.0.cmp(&a.0));
        let results: Vec<SearchResult> = candidates.into_iter()
            .take(limit)
            .map(|(_, result)| result)
            .collect();
        
        debug!("Fuzzy search returned {} results", results.len());
        Ok(results)
    }
    
    pub async fn get_statistics(&self) -> Result<DatabaseStats> {
        let mut stmt = self.conn.prepare(r#"
            SELECT 
                COUNT(DISTINCT t.id) as total_tracks,
                COUNT(DISTINCT t.artist_name) as unique_artists,
                COUNT(DISTINCT t.album_name) as unique_albums,
                COUNT(CASE WHEN l.synced_lyrics IS NOT NULL AND l.synced_lyrics != '' THEN 1 END) as synced_tracks,
                COUNT(CASE WHEN l.plain_lyrics IS NOT NULL AND l.plain_lyrics != '' THEN 1 END) as plain_tracks,
                COUNT(CASE WHEN l.instrumental = 1 THEN 1 END) as instrumental_tracks
            FROM tracks t
            LEFT JOIN lyrics l ON t.last_lyrics_id = l.id
        "#)?;
        
        let stats = stmt.query_row([], |row| {
            Ok(DatabaseStats {
                total_tracks: row.get(0)?,
                unique_artists: row.get(1)?,
                unique_albums: row.get(2)?,
                synced_tracks: row.get(3)?,
                plain_tracks: row.get(4)?,
                instrumental_tracks: row.get(5)?,
            })
        })?;
        
        Ok(stats)
    }
    
    pub async fn get_last_updated(&self) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT value FROM lrclib_metadata WHERE key = 'last_updated'"
        )?;
        
        let result = stmt.query_row([], |row| Ok(row.get::<_, String>(0)?));
        
        match result {
            Ok(date) => Ok(Some(date)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    
    pub async fn set_last_updated(&self, timestamp: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO lrclib_metadata (key, value, updated_at) VALUES ('last_updated', ?1, ?1)",
            params![timestamp],
        )?;
        
        Ok(())
    }
    
    pub async fn execute_batch(&self, sql: &str) -> Result<()> {
        self.conn.execute_batch(sql)?;
        Ok(())
    }
}
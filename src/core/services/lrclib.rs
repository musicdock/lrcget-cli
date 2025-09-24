use anyhow::Result;
use reqwest;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::core::data::database::DatabaseTrack;
use crate::core::infrastructure::cache::{LyricsCache, LyricsCacheInterface};
use crate::core::data::lrclib_db::LrclibDatabase;

#[derive(Debug, Clone)]
pub struct LyricsDownloadResult {
    pub found: bool,
    pub instrumental: bool,
    pub synced_lyrics: bool,
    pub plain_lyrics: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchResultSource {
    LocalDb,
    Cache,
    Api,
}

impl SearchResultSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            SearchResultSource::LocalDb => "DB",
            SearchResultSource::Cache => "CACHE",
            SearchResultSource::Api => "API",
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub artist_name: Option<String>,
    pub album_name: Option<String>,
    pub duration: Option<f64>,
    pub plain_lyrics: Option<String>,
    pub synced_lyrics: Option<String>,
    pub instrumental: bool,
    #[serde(skip, default = "default_search_result_source")]
    pub source: SearchResultSource,
}

fn default_search_result_source() -> SearchResultSource {
    SearchResultSource::Api
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LyricsResponse {
    pub plain_lyrics: Option<String>,
    pub synced_lyrics: Option<String>,
    pub instrumental: bool,
}

impl From<SearchResult> for LyricsResponse {
    fn from(search_result: SearchResult) -> Self {
        LyricsResponse {
            plain_lyrics: search_result.plain_lyrics,
            synced_lyrics: search_result.synced_lyrics,
            instrumental: search_result.instrumental,
        }
    }
}

#[derive(Clone)]
pub struct LrclibClient {
    client: reqwest::Client,
    base_url: String,
    local_db_path: Option<std::path::PathBuf>,
}

impl LrclibClient {
    pub fn new(base_url: &str) -> Self {
        let version = env!("CARGO_PKG_VERSION");
        let user_agent = format!("LRCGET-CLI v{} (https://github.com/musicdock/lrcget-cli)", version);
        
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(user_agent)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            local_db_path: None,
        }
    }

    pub fn with_local_db<P: AsRef<std::path::Path>>(base_url: &str, db_path: P) -> Self {
        let mut client = Self::new(base_url);
        client.local_db_path = Some(db_path.as_ref().to_path_buf());
        client
    }

    pub async fn search(
        &self,
        title: &str,
        artist: &str,
        album: &str,
        query: &str,
    ) -> Result<Vec<SearchResult>> {
        if let Some(ref db_path) = self.local_db_path {
            if db_path.exists() {
                info!("Searching local LRCLIB database");
                let lrclib_db = LrclibDatabase::new(db_path).await?;
                let mut local_results = lrclib_db.search(title, artist, album, query).await?;
                if !local_results.is_empty() {
                    // Mark all results as coming from local database
                    for result in &mut local_results {
                        result.source = SearchResultSource::LocalDb;
                    }
                    return Ok(local_results);
                }
                // If no local results, we will try the remote API below
            }
        }

        // 2) Fallback to remote API
        info!("Searching LRCLIB API");
        let url = format!("{}/api/search", self.base_url);
        
        let mut params = vec![];
        if !title.is_empty() {
            params.push(("track_name", title));
        }
        if !artist.is_empty() {
            params.push(("artist_name", artist));
        }
        if !album.is_empty() {
            params.push(("album_name", album));
        }
        if !query.is_empty() {
            params.push(("q", query));
        }

        info!("Searching LRCLIB API with params: {:?}", params);

        // Basic retry with exponential backoff for transient errors
        let mut attempt = 0u32;
        let max_attempts = 3u32;
        loop {
            attempt += 1;
            let resp_result = self.client
                .get(&url)
                .query(&params)
                .send()
                .await;

            match resp_result {
                Ok(response) => {
                    if response.status().is_success() {
                        let mut results: Vec<SearchResult> = response.json().await?;

                        // Mark all results as coming from API
                        for result in &mut results {
                            result.source = SearchResultSource::Api;
                        }

                        // 3) If we have a local DB configured, update it with API results
                        if !results.is_empty() {
                            if let Some(ref db_path) = self.local_db_path {
                                if db_path.exists() {
                                    if let Err(e) = self.update_local_db_with_search_results(&results).await {
                                        warn!("Failed to update local database from search results: {}", e);
                                    }
                                }
                            }
                        }

                        return Ok(results);
                    }

                    let status = response.status();
                    // Retry on 429 Too Many Requests and 5xx server errors
                    if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                        if attempt < max_attempts {
                            let backoff = 2u64.pow(attempt - 1) * 300; // 300ms, 600ms
                            tokio::time::sleep(Duration::from_millis(backoff)).await;
                            continue;
                        }
                    }

                    anyhow::bail!("Search failed: {}", status);
                }
                Err(e) => {
                    // Retry on network errors
                    if attempt < max_attempts {
                        let backoff = 2u64.pow(attempt - 1) * 300; // 300ms, 600ms
                        tokio::time::sleep(Duration::from_millis(backoff)).await;
                        continue;
                    }
                    return Err(anyhow::anyhow!("Search request error: {}", e));
                }
            }
        }
    }

    pub async fn get_lyrics(
        &self,
        title: &str,
        artist: &str,
        album: &str,
        duration: f64,
    ) -> Result<Option<SearchResult>> {
        // Try local database first if available
        if let Some(ref db_path) = self.local_db_path {
            if db_path.exists() {
                info!("Searching local LRCLIB database for: {} - {}", artist, title);
                match self.search_local_db(title, artist, album, duration).await {
                    Ok(Some(lyrics)) => {
                        info!("Found lyrics in local database");
                        return Ok(Some(lyrics));
                    },
                    Ok(None) => {
                        info!("No lyrics found in local database, trying API");
                    },
                    Err(e) => {
                        warn!("Error searching local database: {}, trying API", e);
                    }
                }
            }
        }

        // Fallback to remote API
        debug!("Getting lyrics from LRCLIB API for: {} - {}", artist, title);
        let url = format!("{}/api/get", self.base_url);
        
        let duration_str = duration.round().to_string();
        let params = vec![
            ("track_name", title),
            ("artist_name", artist),
            ("album_name", album),
            ("duration", &duration_str),
        ];

        // Basic retry with exponential backoff for transient errors
        let mut attempt = 0u32;
        let max_attempts = 3u32;
        loop {
            attempt += 1;
            let resp_result = self.client
                .get(&url)
                .query(&params)
                .send()
                .await;

            match resp_result {
                Ok(response) => {
                    match response.status() {
                        reqwest::StatusCode::OK => {
                            let lyrics: SearchResult = response.json().await?;
                            
                            // Update local database if we have one and got results from API
                            if let Some(ref _db_path) = self.local_db_path {
                                if let Err(e) = self.update_local_db(&lyrics, title, artist, album, duration).await {
                                    warn!("Failed to update local database: {}", e);
                                }
                            }
                            
                            return Ok(Some(lyrics));
                        },
                        reqwest::StatusCode::NOT_FOUND => {
                            info!("No lyrics found for: {} - {}", artist, title);
                            return Ok(None);
                        },
                        status => {
                            if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                                if attempt < max_attempts {
                                    let backoff = 2u64.pow(attempt - 1) * 300; // 300ms, 600ms
                                    tokio::time::sleep(Duration::from_millis(backoff)).await;
                                    continue;
                                }
                            }
                            anyhow::bail!("Failed to get lyrics: {}", status);
                        }
                    }
                }
                Err(e) => {
                    if attempt < max_attempts {
                        let backoff = 2u64.pow(attempt - 1) * 300; // 300ms, 600ms
                        tokio::time::sleep(Duration::from_millis(backoff)).await;
                        continue;
                    }
                    return Err(anyhow::anyhow!("Get lyrics request error: {}", e));
                }
            }
        }
    }

    async fn search_local_db(
        &self,
        title: &str,
        artist: &str,
        album: &str,
        duration: f64,
    ) -> Result<Option<SearchResult>> {
        use crate::core::data::lrclib_db::LrclibDatabase;

        let db_path = self.local_db_path.as_ref().expect("local_db_path must be set when searching local DB");

        // Create a new connection for this specific search to avoid connection sharing issues
        // SQLite WAL mode allows multiple concurrent readers
        let lrclib_db = LrclibDatabase::new(db_path).await?;

        // First try exact match with duration tolerance
        if let Some(result) = lrclib_db.search_exact(title, artist, album, duration).await? {
            return Ok(Some(result));
        }

        // If exact match fails, try fuzzy search as fallback
        debug!("Exact match failed, trying fuzzy search in local database");
        let fuzzy_results = lrclib_db.fuzzy_search(title, artist, album, "", Some(5)).await?;

        if !fuzzy_results.is_empty() {
            debug!("Found {} fuzzy matches in local database", fuzzy_results.len());
            // Return the best fuzzy match
            return Ok(Some(fuzzy_results[0].clone()));
        }

        debug!("No matches found in local database (exact or fuzzy)");
        Ok(None)
    }

    async fn update_local_db(
        &self,
        lyrics: &SearchResult,
        title: &str,
        artist: &str,
        album: &str,
        duration: f64,
    ) -> Result<()> {
        use crate::core::data::lrclib_db::{LrclibDatabase, LrclibTrack};
        
        let db_path = self.local_db_path.as_ref().expect("local_db_path must be set when updating local DB");
        let lrclib_db = LrclibDatabase::new(db_path).await?;
        
        let track = LrclibTrack {
            id: lyrics.id.unwrap_or(0),
            name: Some(title.to_string()),
            artist_name: Some(artist.to_string()),
            album_name: Some(album.to_string()),
            duration: Some(duration),
            plain_lyrics: lyrics.plain_lyrics.clone(),
            synced_lyrics: lyrics.synced_lyrics.clone(),
            instrumental: lyrics.instrumental,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        
        lrclib_db.insert_track(&track).await?;
        debug!("Updated local database with lyrics for: {} - {}", artist, title);
        
        Ok(())
    }

    async fn update_local_db_with_search_results(&self, results: &Vec<SearchResult>) -> Result<()> {
        use crate::core::data::lrclib_db::{LrclibDatabase, LrclibTrack};
        let db_path = self.local_db_path.as_ref().expect("local_db_path must be set when updating local DB");
        let lrclib_db = LrclibDatabase::new(db_path).await?;

        for res in results {
            let track = LrclibTrack {
                id: res.id.unwrap_or(0),
                name: res.name.clone(),
                artist_name: res.artist_name.clone(),
                album_name: res.album_name.clone(),
                duration: res.duration,
                plain_lyrics: res.plain_lyrics.clone(),
                synced_lyrics: res.synced_lyrics.clone(),
                instrumental: res.instrumental,
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            };

            // Best-effort insert; continue on errors to avoid failing whole search
            if let Err(e) = lrclib_db.insert_track(&track).await {
                warn!("Failed to insert API search result into local DB: {}", e);
            }
        }

        Ok(())
    }

    pub async fn fuzzy_search(
        &self,
        title: &str,
        artist: &str,
        album: &str,
        query: &str,
    ) -> Result<Vec<SearchResult>> {
        debug!("Performing fuzzy search on API: title='{}', artist='{}', album='{}', query='{}'", 
               title, artist, album, query);

        // First try local database fuzzy search if available
        if let Some(ref db_path) = self.local_db_path {
            if db_path.exists() {
                debug!("Trying fuzzy search in local database first");
                let lrclib_db = LrclibDatabase::new(db_path).await?;
                let local_results = lrclib_db.fuzzy_search(title, artist, album, query, Some(10)).await?;
                if !local_results.is_empty() {
                    debug!("Found {} fuzzy matches in local database", local_results.len());
                    return Ok(local_results);
                }
            }
        }

        // Generate fuzzy search variations for API
        let matcher = SkimMatcherV2::default();
        let mut all_results = Vec::new();
        let mut unique_results = std::collections::HashSet::new();

        // Create search variations
        let search_terms = generate_search_variations(title, artist, album, query);
        
        for (search_title, search_artist, search_album, search_query) in search_terms {
            match self.search(&search_title, &search_artist, &search_album, &search_query).await {
                Ok(mut results) => {
                    // Score and filter results using fuzzy matching
                    let mut scored_results = Vec::new();
                    
                    for result in results.drain(..) {
                        let mut score = 0i64;
                        let mut match_count = 0;
                        
                        if let Some(ref name) = result.name {
                            if !title.is_empty() {
                                if let Some(s) = matcher.fuzzy_match(name, title) {
                                    score += s * 3;
                                    match_count += 1;
                                }
                            }
                        }
                        
                        if let Some(ref artist_name) = result.artist_name {
                            if !artist.is_empty() {
                                if let Some(s) = matcher.fuzzy_match(artist_name, artist) {
                                    score += s * 2;
                                    match_count += 1;
                                }
                            }
                        }
                        
                        if let Some(ref album_name) = result.album_name {
                            if !album.is_empty() {
                                if let Some(s) = matcher.fuzzy_match(album_name, album) {
                                    score += s;
                                    match_count += 1;
                                }
                            }
                        }
                        
                        if !query.is_empty() {
                            let combined = format!("{} {} {}", 
                                result.name.as_deref().unwrap_or(""),
                                result.artist_name.as_deref().unwrap_or(""),
                                result.album_name.as_deref().unwrap_or(""));
                            if let Some(s) = matcher.fuzzy_match(&combined, query) {
                                score += s * 2;
                                match_count += 1;
                            }
                        }
                        
                        // Only include results with reasonable matches
                        if match_count > 0 && score > 40 {
                            let key = format!("{}-{}-{}", 
                                result.name.as_deref().unwrap_or(""),
                                result.artist_name.as_deref().unwrap_or(""),
                                result.album_name.as_deref().unwrap_or(""));
                            if unique_results.insert(key) {
                                scored_results.push((score, result));
                            }
                        }
                    }
                    
                    all_results.extend(scored_results);
                },
                Err(e) => {
                    debug!("API search variation failed: {}", e);
                    // Continue with other variations
                }
            }
            
            // Small delay between API calls to be respectful
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        // Sort by score and return top results
        all_results.sort_by(|a, b| b.0.cmp(&a.0));
        let final_results: Vec<SearchResult> = all_results.into_iter()
            .take(20)
            .map(|(_, result)| result)
            .collect();
        
        debug!("Fuzzy search API returned {} unique results", final_results.len());
        
        // Update local database with fuzzy search results if available
        if !final_results.is_empty() {
            if let Some(ref db_path) = self.local_db_path {
                if db_path.exists() {
                    if let Err(e) = self.update_local_db_with_search_results(&final_results).await {
                        warn!("Failed to update local database with fuzzy search results: {}", e);
                    }
                }
            }
        }
        
        Ok(final_results)
    }
}

// Helper function to generate search variations for fuzzy matching
fn generate_search_variations(title: &str, artist: &str, album: &str, query: &str) -> Vec<(String, String, String, String)> {
    let mut variations = Vec::new();
    
    // Original search
    variations.push((title.to_string(), artist.to_string(), album.to_string(), query.to_string()));
    
    // Search with only essential terms
    if !title.is_empty() && !artist.is_empty() {
        variations.push((title.to_string(), artist.to_string(), String::new(), String::new()));
    }
    
    // Search with partial terms
    if !title.is_empty() {
        let title_words: Vec<&str> = title.split_whitespace().collect();
        if title_words.len() > 1 {
            // Try first word only
            variations.push((title_words[0].to_string(), artist.to_string(), String::new(), String::new()));
            // Try last word only
            if let Some(last_word) = title_words.last() {
                variations.push((last_word.to_string(), artist.to_string(), String::new(), String::new()));
            }
        }
    }
    
    // Search with artist only
    if !artist.is_empty() {
        variations.push((String::new(), artist.to_string(), String::new(), String::new()));
    }
    
    // Combined query search
    if !title.is_empty() || !artist.is_empty() || !album.is_empty() {
        let combined_query = format!("{} {} {}", title, artist, album).trim().to_string();
        if !combined_query.is_empty() {
            variations.push((String::new(), String::new(), String::new(), combined_query));
        }
    }
    
    // Remove duplicates and empty variations
    variations.dedup();
    variations.retain(|v| !v.0.is_empty() || !v.1.is_empty() || !v.2.is_empty() || !v.3.is_empty());
    
    variations
}

#[derive(Clone)]
pub struct LyricsDownloader {
    client: LrclibClient,
    cache: Option<Arc<RwLock<LyricsCache>>>,
}

impl LyricsDownloader {
    pub fn new(lrclib_instance: &str) -> Self {
        Self {
            client: LrclibClient::new(lrclib_instance),
            cache: None,
        }
    }

    pub fn from_client(client: LrclibClient) -> Self {
        Self {
            client,
            cache: None,
        }
    }

    pub fn with_cache(lrclib_instance: &str, cache: Arc<RwLock<LyricsCache>>) -> Self {
        Self {
            client: LrclibClient::new(lrclib_instance),
            cache: Some(cache),
        }
    }

    pub fn from_client_with_cache(client: LrclibClient, cache: Arc<RwLock<LyricsCache>>) -> Self {
        Self {
            client,
            cache: Some(cache),
        }
    }

    pub async fn download_for_track(&self, track: &DatabaseTrack) -> Result<LyricsDownloadResult> {
        debug!("Downloading lyrics for: {} - {}", track.artist_name, track.title);
        
        // Try cache first if available
        let lyrics = if let Some(cache) = &self.cache {
            // Use a scope to minimize lock duration
            let cached_lyrics = {
                let mut cache_guard = cache.write().await; // Need write for get() due to stats updates
                cache_guard.get(&track.title, &track.artist_name, &track.album_name, track.duration).await
            };

            if let Some(cached_lyrics) = cached_lyrics {
                debug!("Using cached lyrics for: {} - {}", track.artist_name, track.title);
                Some(cached_lyrics)
            } else {
                // Cache miss, fetch from API
                let lyrics = self.client.get_lyrics(
                    &track.title,
                    &track.artist_name,
                    &track.album_name,
                    track.duration,
                ).await?;

                // Cache the result if we got one
                if let Some(ref lyrics_data) = lyrics {
                    let lyrics_response: LyricsResponse = (*lyrics_data).clone().into();
                    let mut cache_write = cache.write().await;
                    if let Err(e) = cache_write.put(&track.title, &track.artist_name, &track.album_name, track.duration, lyrics_response).await {
                        debug!("Failed to cache lyrics for {} - {}: {}", track.artist_name, track.title, e);
                    }
                }

                lyrics.map(|l| l.into())
            }
        } else {
            // No cache, direct API call
            self.client.get_lyrics(
                &track.title,
                &track.artist_name,
                &track.album_name,
                track.duration,
            ).await?.map(|l| l.into())
        };

        if let Some(lyrics_data) = lyrics {
            if lyrics_data.instrumental {
                debug!("Track is marked as instrumental: {}", track.title);
                // TODO: Save instrumental marker
                return Ok(LyricsDownloadResult {
                    found: true,
                    instrumental: true,
                    synced_lyrics: false,
                    plain_lyrics: false,
                });
            }

            // Save lyrics to files
            use crate::core::files::lyrics::LyricsManager;
            let lyrics_manager = LyricsManager::new();

            lyrics_manager.save_lyrics_for_track(
                track,
                lyrics_data.plain_lyrics.as_deref(),
                lyrics_data.synced_lyrics.as_deref(),
                false,
            ).await?;

            let has_synced = lyrics_data.synced_lyrics.is_some();
            let has_plain = lyrics_data.plain_lyrics.is_some();

            if has_synced {
                debug!("Found and saved synced lyrics for: {}", track.title);
            }

            if has_plain {
                debug!("Found and saved plain lyrics for: {}", track.title);
            }

            Ok(LyricsDownloadResult {
                found: true,
                instrumental: false,
                synced_lyrics: has_synced,
                plain_lyrics: has_plain,
            })
        } else {
            debug!("No lyrics found for: {} - {}", track.artist_name, track.title);
            Ok(LyricsDownloadResult {
                found: false,
                instrumental: false,
                synced_lyrics: false,
                plain_lyrics: false,
            })
        }
    }
}
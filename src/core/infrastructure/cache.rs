use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock as StdRwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};
use async_trait::async_trait;
use redis::{AsyncCommands, Client as RedisClient};

use crate::core::services::lrclib::LyricsResponse;

// Generic trait for cache implementations
#[async_trait]
pub trait LyricsCacheInterface: Send + Sync {
    async fn get(&mut self, title: &str, artist: &str, album: &str, duration: f64) -> Option<LyricsResponse>;
    async fn put(&mut self, title: &str, artist: &str, album: &str, duration: f64, lyrics: LyricsResponse) -> Result<()>;
    async fn clear(&mut self) -> Result<()>;
    fn get_stats(&self) -> CacheStats;
    async fn cleanup_old_entries(&mut self) -> Result<()>;
    async fn save_index(&self) -> Result<()>;
}

// Re-export the trait with a cleaner name
pub use LyricsCacheInterface as CacheService;

#[derive(Serialize, Deserialize, Clone)]
struct CacheEntry {
    lyrics: LyricsResponse,
    cached_at: u64,
    access_count: u32,
    last_accessed: u64,
}

#[derive(Serialize, Deserialize)]
struct CacheIndex {
    entries: HashMap<String, CacheEntry>,
    total_requests: u64,
    cache_hits: u64,
    last_cleanup: u64,
}

pub struct FileCache {
    cache_dir: PathBuf,
    index: CacheIndex,
    max_age_hours: u64,
    max_entries: usize,
}

impl FileCache {
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&cache_dir)?;
        
        let index_path = cache_dir.join("cache_index.json");
        let index = if index_path.exists() {
            let content = fs::read_to_string(&index_path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| CacheIndex::new())
        } else {
            CacheIndex::new()
        };

        Ok(Self {
            cache_dir,
            index,
            max_age_hours: 24 * 7, // 1 week
            max_entries: 10000,
        })
    }


    fn generate_key(&self, title: &str, artist: &str, album: &str, duration: f64) -> String {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        title.to_lowercase().hash(&mut hasher);
        artist.to_lowercase().hash(&mut hasher);
        album.to_lowercase().hash(&mut hasher);
        (duration as u64).hash(&mut hasher);
        
        format!("{:x}", hasher.finish())
    }
}

impl CacheIndex {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            total_requests: 0,
            cache_hits: 0,
            last_cleanup: current_timestamp(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_requests: u64,
    pub cache_hits: u64,
    pub hit_rate_percent: f64,
    pub last_cleanup: u64,
}

#[async_trait]
impl LyricsCacheInterface for FileCache {
    async fn get(&mut self, title: &str, artist: &str, album: &str, duration: f64) -> Option<LyricsResponse> {
        let key = self.generate_key(title, artist, album, duration);
        self.index.total_requests += 1;

        if let Some(entry) = self.index.entries.get_mut(&key) {
            let now = current_timestamp();
            
            // Check if entry is expired
            if now - entry.cached_at > self.max_age_hours * 3600 {
                debug!("Cache entry expired for: {} - {}", artist, title);
                self.index.entries.remove(&key);
                return None;
            }

            // Update access statistics
            entry.access_count += 1;
            entry.last_accessed = now;
            self.index.cache_hits += 1;

            debug!("Cache hit for: {} - {}", artist, title);
            Some(entry.lyrics.clone())
        } else {
            debug!("Cache miss for: {} - {}", artist, title);
            None
        }
    }

    async fn put(&mut self, title: &str, artist: &str, album: &str, duration: f64, lyrics: LyricsResponse) -> Result<()> {
        let key = self.generate_key(title, artist, album, duration);
        let now = current_timestamp();

        let entry = CacheEntry {
            lyrics,
            cached_at: now,
            access_count: 1,
            last_accessed: now,
        };

        self.index.entries.insert(key, entry);

        // Cleanup if needed
        if self.index.entries.len() > self.max_entries {
            self.cleanup_old_entries().await?;
        }

        debug!("Cached lyrics for: {} - {}", artist, title);
        Ok(())
    }

    async fn clear(&mut self) -> Result<()> {
        self.index.entries.clear();
        self.index.total_requests = 0;
        self.index.cache_hits = 0;
        self.index.last_cleanup = current_timestamp();

        // Remove cache files
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)?;
            fs::create_dir_all(&self.cache_dir)?;
        }

        info!("File cache cleared");
        Ok(())
    }

    fn get_stats(&self) -> CacheStats {
        let hit_rate = if self.index.total_requests > 0 {
            (self.index.cache_hits as f64 / self.index.total_requests as f64) * 100.0
        } else {
            0.0
        };

        CacheStats {
            total_entries: self.index.entries.len(),
            total_requests: self.index.total_requests,
            cache_hits: self.index.cache_hits,
            hit_rate_percent: hit_rate,
            last_cleanup: self.index.last_cleanup,
        }
    }

    async fn cleanup_old_entries(&mut self) -> Result<()> {
        let now = current_timestamp();
        let max_age = self.max_age_hours * 3600;

        // Remove expired entries
        let before_count = self.index.entries.len();
        self.index.entries.retain(|_, entry| now - entry.cached_at <= max_age);
        let after_cleanup_count = self.index.entries.len();

        // If still too many entries, remove least recently used
        if self.index.entries.len() > self.max_entries {
            let mut entries: Vec<(String, &CacheEntry)> = self.index.entries.iter().map(|(k, v)| (k.clone(), v)).collect();
            entries.sort_by(|a, b| a.1.last_accessed.cmp(&b.1.last_accessed));
            
            let to_remove = self.index.entries.len() - self.max_entries;
            let keys_to_remove: Vec<String> = entries.iter().take(to_remove).map(|(key, _)| key.clone()).collect();
            
            for key in keys_to_remove {
                self.index.entries.remove(&key);
            }
        }

        let final_count = self.index.entries.len();
        self.index.last_cleanup = now;

        info!("File cache cleanup: {} -> {} -> {} entries", 
            before_count, after_cleanup_count, final_count);

        Ok(())
    }

    async fn save_index(&self) -> Result<()> {
        let index_path = self.cache_dir.join("cache_index.json");
        let content = serde_json::to_string_pretty(&self.index)?;
        // Ensure directory exists
        if let Some(parent) = index_path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Write atomically: write to temp then rename
        let tmp_path = index_path.with_extension("json.tmp");
        fs::write(&tmp_path, &content)?;
        std::fs::rename(&tmp_path, &index_path)?;
        Ok(())
    }
}

// Redis cache implementation
pub struct RedisCache {
    client: RedisClient,
    key_prefix: String,
    ttl_seconds: u64,
    stats: CacheStats,
}

impl RedisCache {
    pub fn new(redis_url: &str) -> Result<Self> {
        let client = RedisClient::open(redis_url)?;
        
        Ok(Self {
            client,
            key_prefix: "lrcget:lyrics:".to_string(),
            ttl_seconds: 7 * 24 * 3600, // 7 days
            stats: CacheStats {
                total_entries: 0,
                total_requests: 0,
                cache_hits: 0,
                hit_rate_percent: 0.0,
                last_cleanup: current_timestamp(),
            },
        })
    }

    fn generate_key(&self, title: &str, artist: &str, album: &str, duration: f64) -> String {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        title.to_lowercase().hash(&mut hasher);
        artist.to_lowercase().hash(&mut hasher);
        album.to_lowercase().hash(&mut hasher);
        (duration as u64).hash(&mut hasher);
        
        format!("{}{:x}", self.key_prefix, hasher.finish())
    }
}

#[async_trait]
impl LyricsCacheInterface for RedisCache {
    async fn get(&mut self, title: &str, artist: &str, album: &str, duration: f64) -> Option<LyricsResponse> {
        let key = self.generate_key(title, artist, album, duration);
        self.stats.total_requests += 1;

        match self.client.get_async_connection().await {
            Ok(mut con) => {
                match con.get::<_, Option<String>>(&key).await {
                    Ok(Some(value)) => {
                        match serde_json::from_str::<LyricsResponse>(&value) {
                            Ok(lyrics) => {
                                self.stats.cache_hits += 1;
                                debug!("Redis cache hit for: {} - {}", artist, title);
                                Some(lyrics)
                            }
                            Err(e) => {
                                warn!("Failed to deserialize cached lyrics: {}", e);
                                None
                            }
                        }
                    }
                    Ok(None) => {
                        debug!("Redis cache miss for: {} - {}", artist, title);
                        None
                    }
                    Err(e) => {
                        warn!("Redis get error for {}: {}", key, e);
                        None
                    }
                }
            }
            Err(e) => {
                warn!("Failed to connect to Redis: {}", e);
                None
            }
        }
    }

    async fn put(&mut self, title: &str, artist: &str, album: &str, duration: f64, lyrics: LyricsResponse) -> Result<()> {
        let key = self.generate_key(title, artist, album, duration);
        
        match serde_json::to_string(&lyrics) {
            Ok(value) => {
                match self.client.get_async_connection().await {
                    Ok(mut con) => {
                        let _: () = con.set_ex(&key, &value, self.ttl_seconds as u64).await
                            .map_err(|e| anyhow::anyhow!("Redis setex error: {}", e))?;
                        
                        debug!("Cached lyrics in Redis for: {} - {}", artist, title);
                        Ok(())
                    }
                    Err(e) => {
                        warn!("Failed to connect to Redis for put: {}", e);
                        Err(anyhow::anyhow!("Redis connection error: {}", e))
                    }
                }
            }
            Err(e) => {
                warn!("Failed to serialize lyrics for caching: {}", e);
                Err(anyhow::anyhow!("Serialization error: {}", e))
            }
        }
    }

    async fn clear(&mut self) -> Result<()> {
        match self.client.get_async_connection().await {
            Ok(mut con) => {
                let pattern = format!("{}*", self.key_prefix);
                let keys: Vec<String> = con.keys(&pattern).await
                    .map_err(|e| anyhow::anyhow!("Redis keys error: {}", e))?;
                
                if !keys.is_empty() {
                    let _: () = con.del(&keys).await
                        .map_err(|e| anyhow::anyhow!("Redis del error: {}", e))?;
                }
                
                self.stats.total_requests = 0;
                self.stats.cache_hits = 0;
                self.stats.last_cleanup = current_timestamp();
                
                info!("Redis cache cleared, removed {} keys", keys.len());
                Ok(())
            }
            Err(e) => {
                warn!("Failed to connect to Redis for clear: {}", e);
                Err(anyhow::anyhow!("Redis connection error: {}", e))
            }
        }
    }

    fn get_stats(&self) -> CacheStats {
        let mut stats = self.stats.clone();
        stats.hit_rate_percent = if stats.total_requests > 0 {
            (stats.cache_hits as f64 / stats.total_requests as f64) * 100.0
        } else {
            0.0
        };
        stats
    }

    async fn cleanup_old_entries(&mut self) -> Result<()> {
        // Redis handles TTL automatically, so this is mostly a no-op
        // We just update the cleanup timestamp
        self.stats.last_cleanup = current_timestamp();
        debug!("Redis cache cleanup completed (automatic TTL)");
        Ok(())
    }

    async fn save_index(&self) -> Result<()> {
        // Redis doesn't need explicit index saving
        Ok(())
    }
}

// Hybrid cache implementation (Redis + File)
pub struct HybridCache {
    redis_cache: Option<RedisCache>,
    file_cache: FileCache,
}

impl HybridCache {
    pub fn new(file_cache_dir: PathBuf, redis_url: Option<&str>) -> Result<Self> {
        let file_cache = FileCache::new(file_cache_dir)?;
        
        let redis_cache = if let Some(url) = redis_url {
            match RedisCache::new(url) {
                Ok(cache) => {
                    info!("Redis cache initialized successfully");
                    Some(cache)
                }
                Err(e) => {
                    warn!("Failed to initialize Redis cache, falling back to file only: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            redis_cache,
            file_cache,
        })
    }
}

#[async_trait]
impl LyricsCacheInterface for HybridCache {
    async fn get(&mut self, title: &str, artist: &str, album: &str, duration: f64) -> Option<LyricsResponse> {
        // Try Redis cache first
        if let Some(redis) = &mut self.redis_cache {
            if let Some(lyrics) = redis.get(title, artist, album, duration).await {
                debug!("Hybrid cache: Redis hit for {} - {}", artist, title);
                return Some(lyrics);
            }
        }

        // Fallback to file cache
        if let Some(lyrics) = self.file_cache.get(title, artist, album, duration).await {
            debug!("Hybrid cache: File hit for {} - {}", artist, title);
            
            // Update Redis cache if available
            if let Some(redis) = &mut self.redis_cache {
                if let Err(e) = redis.put(title, artist, album, duration, lyrics.clone()).await {
                    debug!("Failed to update Redis cache from file cache: {}", e);
                }
            }
            
            return Some(lyrics);
        }

        debug!("Hybrid cache: Miss for {} - {}", artist, title);
        None
    }

    async fn put(&mut self, title: &str, artist: &str, album: &str, duration: f64, lyrics: LyricsResponse) -> Result<()> {
        let mut redis_error = None;
        let mut file_error = None;

        // Update Redis cache
        if let Some(redis) = &mut self.redis_cache {
            if let Err(e) = redis.put(title, artist, album, duration, lyrics.clone()).await {
                redis_error = Some(e);
            }
        }

        // Update file cache
        if let Err(e) = self.file_cache.put(title, artist, album, duration, lyrics).await {
            file_error = Some(e);
        }

        // Return error only if both caches failed
        match (redis_error, file_error) {
            (Some(redis_err), Some(file_err)) => {
                Err(anyhow::anyhow!("Both caches failed - Redis: {}, File: {}", redis_err, file_err))
            }
            (Some(redis_err), None) => {
                debug!("Redis cache put failed, but file cache succeeded: {}", redis_err);
                Ok(())
            }
            (None, Some(file_err)) => {
                debug!("File cache put failed, but Redis cache succeeded: {}", file_err);
                Ok(())
            }
            (None, None) => Ok(())
        }
    }

    async fn clear(&mut self) -> Result<()> {
        let mut errors = Vec::new();

        if let Some(redis) = &mut self.redis_cache {
            if let Err(e) = redis.clear().await {
                errors.push(format!("Redis: {}", e));
            }
        }

        if let Err(e) = self.file_cache.clear().await {
            errors.push(format!("File: {}", e));
        }

        if errors.is_empty() {
            info!("Hybrid cache cleared successfully");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Cache clear errors: {}", errors.join(", ")))
        }
    }

    fn get_stats(&self) -> CacheStats {
        let file_stats = self.file_cache.get_stats();
        
        if let Some(redis) = &self.redis_cache {
            let redis_stats = redis.get_stats();
            
            // Combine stats from both caches
            CacheStats {
                total_entries: file_stats.total_entries, // File cache tracks entries
                total_requests: file_stats.total_requests + redis_stats.total_requests,
                cache_hits: file_stats.cache_hits + redis_stats.cache_hits,
                hit_rate_percent: {
                    let total_reqs = file_stats.total_requests + redis_stats.total_requests;
                    let total_hits = file_stats.cache_hits + redis_stats.cache_hits;
                    if total_reqs > 0 {
                        (total_hits as f64 / total_reqs as f64) * 100.0
                    } else {
                        0.0
                    }
                },
                last_cleanup: std::cmp::max(file_stats.last_cleanup, redis_stats.last_cleanup),
            }
        } else {
            file_stats
        }
    }

    async fn cleanup_old_entries(&mut self) -> Result<()> {
        let mut errors = Vec::new();

        if let Some(redis) = &mut self.redis_cache {
            if let Err(e) = redis.cleanup_old_entries().await {
                errors.push(format!("Redis: {}", e));
            }
        }

        if let Err(e) = self.file_cache.cleanup_old_entries().await {
            errors.push(format!("File: {}", e));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Cleanup errors: {}", errors.join(", ")))
        }
    }

    async fn save_index(&self) -> Result<()> {
        // Only file cache needs index saving
        self.file_cache.save_index().await
    }
}

// Type alias for backward compatibility
pub type LyricsCache = HybridCache;

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}
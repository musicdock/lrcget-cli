use crate::core::files::scanner::Track;
use crate::error::Result;

#[async_trait::async_trait]
pub trait DatabaseService: Send + Sync {
    async fn get_tracks(&self) -> Result<Vec<Track>>;
    async fn insert_track(&self, track: &Track) -> Result<i64>;
    async fn update_track(&self, track: &Track) -> Result<()>;
    async fn delete_track(&self, id: i64) -> Result<()>;
    async fn search_tracks(&self, query: &str) -> Result<Vec<Track>>;

    async fn get_track_by_path(&self, path: &str) -> Result<Option<Track>> {
        let tracks = self.search_tracks(&format!("path:{}", path)).await?;
        Ok(tracks.into_iter().next())
    }

    async fn get_tracks_without_lyrics(&self) -> Result<Vec<Track>> {
        self.search_tracks("has_lyrics:false").await
    }

    async fn get_tracks_by_artist(&self, artist: &str) -> Result<Vec<Track>> {
        self.search_tracks(&format!("artist:{}", artist)).await
    }

    async fn get_tracks_by_album(&self, album: &str) -> Result<Vec<Track>> {
        self.search_tracks(&format!("album:{}", album)).await
    }
}

pub struct MockDatabaseService {
    tracks: std::sync::Mutex<Vec<Track>>,
}

impl MockDatabaseService {
    pub fn new() -> Self {
        Self {
            tracks: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn with_tracks(tracks: Vec<Track>) -> Self {
        Self {
            tracks: std::sync::Mutex::new(tracks),
        }
    }
}

#[async_trait::async_trait]
impl DatabaseService for MockDatabaseService {
    async fn get_tracks(&self) -> Result<Vec<Track>> {
        let tracks = self.tracks.lock().unwrap();
        Ok(tracks.clone())
    }

    async fn insert_track(&self, track: &Track) -> Result<i64> {
        let mut tracks = self.tracks.lock().unwrap();
        let id = tracks.len() as i64 + 1;
        tracks.push(track.clone());
        Ok(id)
    }

    async fn update_track(&self, track: &Track) -> Result<()> {
        let mut tracks = self.tracks.lock().unwrap();
        // For simplicity, find by file_path
        if let Some(existing) = tracks.iter_mut().find(|t| t.file_path == track.file_path) {
            *existing = track.clone();
        }
        Ok(())
    }

    async fn delete_track(&self, id: i64) -> Result<()> {
        let mut tracks = self.tracks.lock().unwrap();
        // For simplicity, delete by index
        if id > 0 && (id as usize) <= tracks.len() {
            tracks.remove(id as usize - 1);
        }
        Ok(())
    }

    async fn search_tracks(&self, query: &str) -> Result<Vec<Track>> {
        let tracks = self.tracks.lock().unwrap();

        if query.starts_with("path:") {
            let path = &query[5..];
            Ok(tracks.iter()
                .filter(|t| t.file_path.contains(path))
                .cloned()
                .collect())
        } else if query.starts_with("artist:") {
            let artist = &query[7..];
            Ok(tracks.iter()
                .filter(|t| t.artist.contains(artist))
                .cloned()
                .collect())
        } else if query.starts_with("album:") {
            let album = &query[6..];
            Ok(tracks.iter()
                .filter(|t| t.album.contains(album))
                .cloned()
                .collect())
        } else if query == "has_lyrics:false" {
            Ok(tracks.iter()
                .filter(|t| t.lrc_lyrics.is_none() && t.txt_lyrics.is_none())
                .cloned()
                .collect())
        } else {
            Ok(tracks.iter()
                .filter(|t| {
                    t.title.contains(query) ||
                    t.artist.contains(query) ||
                    t.album.contains(query)
                })
                .cloned()
                .collect())
        }
    }
}
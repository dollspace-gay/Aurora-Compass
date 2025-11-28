//! External embed support for rich content
//!
//! This module provides support for embedding external content like YouTube videos,
//! Spotify tracks/playlists, and generic link previews in posts and messages.

use crate::link_preview::LinkPreview;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during embed operations
#[derive(Debug, Error)]
pub enum EmbedError {
    /// Invalid URL
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Unsupported embed type
    #[error("Unsupported embed type for URL: {0}")]
    UnsupportedType(String),

    /// Missing embed ID
    #[error("Missing embed ID from URL")]
    MissingId,

    /// Invalid embed configuration
    #[error("Invalid embed configuration: {0}")]
    InvalidConfig(String),
}

/// Result type for embed operations
pub type Result<T> = std::result::Result<T, EmbedError>;

/// Type of external embed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbedType {
    /// YouTube video
    YouTube,
    /// Spotify track, album, or playlist
    Spotify,
    /// Generic link preview
    Link,
}

impl EmbedType {
    /// Get the embed type as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            EmbedType::YouTube => "youtube",
            EmbedType::Spotify => "spotify",
            EmbedType::Link => "link",
        }
    }
}

/// YouTube video embed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YouTubeEmbed {
    /// Video ID
    pub video_id: String,
    /// Video title (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Channel name (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    /// Thumbnail URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    /// Start time in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<u32>,
}

impl YouTubeEmbed {
    /// Create a new YouTube embed
    pub fn new(video_id: impl Into<String>) -> Self {
        Self {
            video_id: video_id.into(),
            title: None,
            channel: None,
            thumbnail: None,
            start_time: None,
        }
    }

    /// Get the embed URL
    pub fn embed_url(&self) -> String {
        let mut url = format!("https://www.youtube.com/embed/{}", self.video_id);
        if let Some(start) = self.start_time {
            url.push_str(&format!("?start={}", start));
        }
        url
    }

    /// Get the watch URL
    pub fn watch_url(&self) -> String {
        let mut url = format!("https://www.youtube.com/watch?v={}", self.video_id);
        if let Some(start) = self.start_time {
            url.push_str(&format!("&t={}s", start));
        }
        url
    }

    /// Get thumbnail URL for a specific quality
    pub fn get_thumbnail(&self, quality: YouTubeThumbnailQuality) -> String {
        format!("https://img.youtube.com/vi/{}/{}", self.video_id, quality.filename())
    }
}

/// YouTube thumbnail quality
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YouTubeThumbnailQuality {
    /// Default quality (120x90)
    Default,
    /// Medium quality (320x180)
    Medium,
    /// High quality (480x360)
    High,
    /// Standard definition (640x480)
    StandardDef,
    /// Max resolution (1280x720)
    MaxRes,
}

impl YouTubeThumbnailQuality {
    /// Get the filename for this quality
    pub fn filename(&self) -> &'static str {
        match self {
            YouTubeThumbnailQuality::Default => "default.jpg",
            YouTubeThumbnailQuality::Medium => "mqdefault.jpg",
            YouTubeThumbnailQuality::High => "hqdefault.jpg",
            YouTubeThumbnailQuality::StandardDef => "sddefault.jpg",
            YouTubeThumbnailQuality::MaxRes => "maxresdefault.jpg",
        }
    }
}

/// Spotify embed type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpotifyEmbedType {
    /// Track
    Track,
    /// Album
    Album,
    /// Playlist
    Playlist,
    /// Artist
    Artist,
    /// Show (podcast)
    Show,
    /// Episode (podcast episode)
    Episode,
}

impl SpotifyEmbedType {
    /// Get the type as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            SpotifyEmbedType::Track => "track",
            SpotifyEmbedType::Album => "album",
            SpotifyEmbedType::Playlist => "playlist",
            SpotifyEmbedType::Artist => "artist",
            SpotifyEmbedType::Show => "show",
            SpotifyEmbedType::Episode => "episode",
        }
    }
}

/// Spotify embed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotifyEmbed {
    /// Embed type (track, album, playlist, etc.)
    pub embed_type: SpotifyEmbedType,
    /// Spotify ID
    pub id: String,
    /// Title (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Artist or creator name (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
}

impl SpotifyEmbed {
    /// Create a new Spotify embed
    pub fn new(embed_type: SpotifyEmbedType, id: impl Into<String>) -> Self {
        Self {
            embed_type,
            id: id.into(),
            title: None,
            artist: None,
        }
    }

    /// Get the embed URL
    pub fn embed_url(&self) -> String {
        format!("https://open.spotify.com/embed/{}/{}", self.embed_type.as_str(), self.id)
    }

    /// Get the open URL
    pub fn open_url(&self) -> String {
        format!("https://open.spotify.com/{}/{}", self.embed_type.as_str(), self.id)
    }
}

/// Generic link embed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkEmbed {
    /// Link preview
    pub preview: LinkPreview,
}

impl LinkEmbed {
    /// Create a new link embed
    pub fn new(preview: LinkPreview) -> Self {
        Self { preview }
    }

    /// Get the URL
    pub fn url(&self) -> &str {
        &self.preview.url
    }
}

/// External embed that can be any supported type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ExternalEmbed {
    /// YouTube video embed
    #[serde(rename = "youtube")]
    YouTube(YouTubeEmbed),
    /// Spotify embed
    #[serde(rename = "spotify")]
    Spotify(SpotifyEmbed),
    /// Generic link embed
    #[serde(rename = "link")]
    Link(LinkEmbed),
}

impl ExternalEmbed {
    /// Get the embed type
    pub fn embed_type(&self) -> EmbedType {
        match self {
            ExternalEmbed::YouTube(_) => EmbedType::YouTube,
            ExternalEmbed::Spotify(_) => EmbedType::Spotify,
            ExternalEmbed::Link(_) => EmbedType::Link,
        }
    }

    /// Check if this is a YouTube embed
    pub fn is_youtube(&self) -> bool {
        matches!(self, ExternalEmbed::YouTube(_))
    }

    /// Check if this is a Spotify embed
    pub fn is_spotify(&self) -> bool {
        matches!(self, ExternalEmbed::Spotify(_))
    }

    /// Check if this is a link embed
    pub fn is_link(&self) -> bool {
        matches!(self, ExternalEmbed::Link(_))
    }

    /// Get as YouTube embed if applicable
    pub fn as_youtube(&self) -> Option<&YouTubeEmbed> {
        match self {
            ExternalEmbed::YouTube(embed) => Some(embed),
            _ => None,
        }
    }

    /// Get as Spotify embed if applicable
    pub fn as_spotify(&self) -> Option<&SpotifyEmbed> {
        match self {
            ExternalEmbed::Spotify(embed) => Some(embed),
            _ => None,
        }
    }

    /// Get as link embed if applicable
    pub fn as_link(&self) -> Option<&LinkEmbed> {
        match self {
            ExternalEmbed::Link(embed) => Some(embed),
            _ => None,
        }
    }
}

/// Embed detector for identifying and parsing embeddable URLs
pub struct EmbedDetector;

impl EmbedDetector {
    /// Detect if a URL is embeddable and return its type
    pub fn detect(url: &str) -> Option<EmbedType> {
        if Self::is_youtube_url(url) {
            Some(EmbedType::YouTube)
        } else if Self::is_spotify_url(url) {
            Some(EmbedType::Spotify)
        } else {
            Some(EmbedType::Link)
        }
    }

    /// Check if URL is a YouTube URL
    pub fn is_youtube_url(url: &str) -> bool {
        url.contains("youtube.com") || url.contains("youtu.be")
    }

    /// Check if URL is a Spotify URL
    pub fn is_spotify_url(url: &str) -> bool {
        url.contains("spotify.com") || url.contains("spotify:")
    }

    /// Parse a YouTube URL into an embed
    pub fn parse_youtube(url: &str) -> Result<YouTubeEmbed> {
        let video_id = Self::extract_youtube_id(url)?;
        let start_time = Self::extract_youtube_timestamp(url);

        let mut embed = YouTubeEmbed::new(video_id);
        embed.start_time = start_time;
        Ok(embed)
    }

    /// Extract YouTube video ID from URL
    fn extract_youtube_id(url: &str) -> Result<String> {
        // Handle youtu.be short URLs
        if url.contains("youtu.be/") {
            if let Some(id_start) = url.find("youtu.be/") {
                let id_part = &url[id_start + 9..];
                let id = id_part
                    .split(&['?', '&', '#'][..])
                    .next()
                    .ok_or_else(|| EmbedError::MissingId)?;
                return Ok(id.to_string());
            }
        }

        // Handle standard youtube.com URLs
        if url.contains("youtube.com/watch") {
            // Look for v= parameter
            if let Some(v_pos) = url.find("v=") {
                let id_part = &url[v_pos + 2..];
                let id = id_part
                    .split(&['&', '#'][..])
                    .next()
                    .ok_or_else(|| EmbedError::MissingId)?;
                return Ok(id.to_string());
            }
        }

        // Handle youtube.com/embed/ URLs
        if url.contains("youtube.com/embed/") {
            if let Some(id_start) = url.find("youtube.com/embed/") {
                let id_part = &url[id_start + 18..];
                let id = id_part
                    .split(&['?', '&', '#'][..])
                    .next()
                    .ok_or_else(|| EmbedError::MissingId)?;
                return Ok(id.to_string());
            }
        }

        Err(EmbedError::MissingId)
    }

    /// Extract timestamp from YouTube URL
    fn extract_youtube_timestamp(url: &str) -> Option<u32> {
        // Look for t= or start= parameter
        for param in ["t=", "start="] {
            if let Some(t_pos) = url.find(param) {
                let time_part = &url[t_pos + param.len()..];
                let time_str = time_part.split(&['&', '#'][..]).next()?;
                // Remove trailing 's' if present
                let time_str = time_str.trim_end_matches('s');
                return time_str.parse::<u32>().ok();
            }
        }
        None
    }

    /// Parse a Spotify URL into an embed
    pub fn parse_spotify(url: &str) -> Result<SpotifyEmbed> {
        let (embed_type, id) = Self::extract_spotify_info(url)?;
        Ok(SpotifyEmbed::new(embed_type, id))
    }

    /// Extract Spotify type and ID from URL
    fn extract_spotify_info(url: &str) -> Result<(SpotifyEmbedType, String)> {
        // Handle spotify: URIs (e.g., spotify:track:123)
        if url.starts_with("spotify:") {
            let parts: Vec<&str> = url.split(':').collect();
            if parts.len() >= 3 {
                let embed_type = Self::parse_spotify_type(parts[1])?;
                let id = parts[2].to_string();
                return Ok((embed_type, id));
            }
        }

        // Handle open.spotify.com URLs
        if url.contains("open.spotify.com") {
            // Find the type and ID in the URL path
            let path = url
                .split("open.spotify.com/")
                .nth(1)
                .ok_or_else(|| EmbedError::InvalidUrl(url.to_string()))?;

            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() >= 2 {
                let embed_type = Self::parse_spotify_type(parts[0])?;
                let id = parts[1].split('?').next().unwrap_or(parts[1]).to_string();
                return Ok((embed_type, id));
            }
        }

        Err(EmbedError::InvalidUrl(url.to_string()))
    }

    /// Parse Spotify embed type string
    fn parse_spotify_type(type_str: &str) -> Result<SpotifyEmbedType> {
        match type_str {
            "track" => Ok(SpotifyEmbedType::Track),
            "album" => Ok(SpotifyEmbedType::Album),
            "playlist" => Ok(SpotifyEmbedType::Playlist),
            "artist" => Ok(SpotifyEmbedType::Artist),
            "show" => Ok(SpotifyEmbedType::Show),
            "episode" => Ok(SpotifyEmbedType::Episode),
            _ => Err(EmbedError::UnsupportedType(type_str.to_string())),
        }
    }

    /// Parse any URL into an external embed
    pub fn parse(url: &str) -> Result<ExternalEmbed> {
        if Self::is_youtube_url(url) {
            Ok(ExternalEmbed::YouTube(Self::parse_youtube(url)?))
        } else if Self::is_spotify_url(url) {
            Ok(ExternalEmbed::Spotify(Self::parse_spotify(url)?))
        } else {
            // For generic URLs, create a basic link embed
            let preview = LinkPreview::new(url);
            Ok(ExternalEmbed::Link(LinkEmbed::new(preview)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // YouTube tests

    #[test]
    fn test_youtube_embed_creation() {
        let embed = YouTubeEmbed::new("dQw4w9WgXcQ");
        assert_eq!(embed.video_id, "dQw4w9WgXcQ");
        assert!(embed.title.is_none());
        assert!(embed.start_time.is_none());
    }

    #[test]
    fn test_youtube_embed_url() {
        let embed = YouTubeEmbed::new("dQw4w9WgXcQ");
        assert_eq!(embed.embed_url(), "https://www.youtube.com/embed/dQw4w9WgXcQ");
    }

    #[test]
    fn test_youtube_embed_url_with_start_time() {
        let mut embed = YouTubeEmbed::new("dQw4w9WgXcQ");
        embed.start_time = Some(42);
        assert_eq!(embed.embed_url(), "https://www.youtube.com/embed/dQw4w9WgXcQ?start=42");
    }

    #[test]
    fn test_youtube_watch_url() {
        let embed = YouTubeEmbed::new("dQw4w9WgXcQ");
        assert_eq!(embed.watch_url(), "https://www.youtube.com/watch?v=dQw4w9WgXcQ");
    }

    #[test]
    fn test_youtube_watch_url_with_start_time() {
        let mut embed = YouTubeEmbed::new("dQw4w9WgXcQ");
        embed.start_time = Some(42);
        assert_eq!(embed.watch_url(), "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=42s");
    }

    #[test]
    fn test_youtube_thumbnail() {
        let embed = YouTubeEmbed::new("dQw4w9WgXcQ");
        assert_eq!(
            embed.get_thumbnail(YouTubeThumbnailQuality::High),
            "https://img.youtube.com/vi/dQw4w9WgXcQ/hqdefault.jpg"
        );
    }

    #[test]
    fn test_detect_youtube_url() {
        assert!(EmbedDetector::is_youtube_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
        assert!(EmbedDetector::is_youtube_url("https://youtu.be/dQw4w9WgXcQ"));
        assert!(!EmbedDetector::is_youtube_url("https://example.com"));
    }

    #[test]
    fn test_parse_youtube_standard_url() {
        let embed =
            EmbedDetector::parse_youtube("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
        assert_eq!(embed.video_id, "dQw4w9WgXcQ");
    }

    #[test]
    fn test_parse_youtube_short_url() {
        let embed = EmbedDetector::parse_youtube("https://youtu.be/dQw4w9WgXcQ").unwrap();
        assert_eq!(embed.video_id, "dQw4w9WgXcQ");
    }

    #[test]
    fn test_parse_youtube_embed_url() {
        let embed =
            EmbedDetector::parse_youtube("https://www.youtube.com/embed/dQw4w9WgXcQ").unwrap();
        assert_eq!(embed.video_id, "dQw4w9WgXcQ");
    }

    #[test]
    fn test_parse_youtube_with_timestamp() {
        let embed =
            EmbedDetector::parse_youtube("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=42s")
                .unwrap();
        assert_eq!(embed.video_id, "dQw4w9WgXcQ");
        assert_eq!(embed.start_time, Some(42));
    }

    #[test]
    fn test_parse_youtube_with_start_param() {
        let embed =
            EmbedDetector::parse_youtube("https://www.youtube.com/watch?v=dQw4w9WgXcQ&start=100")
                .unwrap();
        assert_eq!(embed.video_id, "dQw4w9WgXcQ");
        assert_eq!(embed.start_time, Some(100));
    }

    // Spotify tests

    #[test]
    fn test_spotify_embed_creation() {
        let embed = SpotifyEmbed::new(SpotifyEmbedType::Track, "3n3Ppam7vgaVa1iaRUc9Lp");
        assert_eq!(embed.embed_type, SpotifyEmbedType::Track);
        assert_eq!(embed.id, "3n3Ppam7vgaVa1iaRUc9Lp");
    }

    #[test]
    fn test_spotify_embed_url() {
        let embed = SpotifyEmbed::new(SpotifyEmbedType::Track, "3n3Ppam7vgaVa1iaRUc9Lp");
        assert_eq!(
            embed.embed_url(),
            "https://open.spotify.com/embed/track/3n3Ppam7vgaVa1iaRUc9Lp"
        );
    }

    #[test]
    fn test_spotify_open_url() {
        let embed = SpotifyEmbed::new(SpotifyEmbedType::Playlist, "37i9dQZF1DXcBWIGoYBM5M");
        assert_eq!(embed.open_url(), "https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M");
    }

    #[test]
    fn test_detect_spotify_url() {
        assert!(EmbedDetector::is_spotify_url("https://open.spotify.com/track/123"));
        assert!(EmbedDetector::is_spotify_url("spotify:track:123"));
        assert!(!EmbedDetector::is_spotify_url("https://example.com"));
    }

    #[test]
    fn test_parse_spotify_url_track() {
        let embed =
            EmbedDetector::parse_spotify("https://open.spotify.com/track/3n3Ppam7vgaVa1iaRUc9Lp")
                .unwrap();
        assert_eq!(embed.embed_type, SpotifyEmbedType::Track);
        assert_eq!(embed.id, "3n3Ppam7vgaVa1iaRUc9Lp");
    }

    #[test]
    fn test_parse_spotify_url_playlist() {
        let embed = EmbedDetector::parse_spotify(
            "https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M",
        )
        .unwrap();
        assert_eq!(embed.embed_type, SpotifyEmbedType::Playlist);
        assert_eq!(embed.id, "37i9dQZF1DXcBWIGoYBM5M");
    }

    #[test]
    fn test_parse_spotify_uri() {
        let embed = EmbedDetector::parse_spotify("spotify:track:3n3Ppam7vgaVa1iaRUc9Lp").unwrap();
        assert_eq!(embed.embed_type, SpotifyEmbedType::Track);
        assert_eq!(embed.id, "3n3Ppam7vgaVa1iaRUc9Lp");
    }

    #[test]
    fn test_parse_spotify_album() {
        let embed =
            EmbedDetector::parse_spotify("https://open.spotify.com/album/1DFixLWuPkv3KT3TnV35m3")
                .unwrap();
        assert_eq!(embed.embed_type, SpotifyEmbedType::Album);
        assert_eq!(embed.id, "1DFixLWuPkv3KT3TnV35m3");
    }

    // Link embed tests

    #[test]
    fn test_link_embed_creation() {
        let preview = LinkPreview::new("https://example.com");
        let embed = LinkEmbed::new(preview);
        assert_eq!(embed.url(), "https://example.com");
    }

    // External embed tests

    #[test]
    fn test_external_embed_youtube() {
        let youtube = YouTubeEmbed::new("dQw4w9WgXcQ");
        let embed = ExternalEmbed::YouTube(youtube);

        assert_eq!(embed.embed_type(), EmbedType::YouTube);
        assert!(embed.is_youtube());
        assert!(!embed.is_spotify());
        assert!(!embed.is_link());
        assert!(embed.as_youtube().is_some());
    }

    #[test]
    fn test_external_embed_spotify() {
        let spotify = SpotifyEmbed::new(SpotifyEmbedType::Track, "123");
        let embed = ExternalEmbed::Spotify(spotify);

        assert_eq!(embed.embed_type(), EmbedType::Spotify);
        assert!(!embed.is_youtube());
        assert!(embed.is_spotify());
        assert!(!embed.is_link());
        assert!(embed.as_spotify().is_some());
    }

    #[test]
    fn test_external_embed_link() {
        let preview = LinkPreview::new("https://example.com");
        let link = LinkEmbed::new(preview);
        let embed = ExternalEmbed::Link(link);

        assert_eq!(embed.embed_type(), EmbedType::Link);
        assert!(!embed.is_youtube());
        assert!(!embed.is_spotify());
        assert!(embed.is_link());
        assert!(embed.as_link().is_some());
    }

    // Detector tests

    #[test]
    fn test_detect_embed_type() {
        assert_eq!(
            EmbedDetector::detect("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            Some(EmbedType::YouTube)
        );
        assert_eq!(
            EmbedDetector::detect("https://open.spotify.com/track/123"),
            Some(EmbedType::Spotify)
        );
        assert_eq!(EmbedDetector::detect("https://example.com"), Some(EmbedType::Link));
    }

    #[test]
    fn test_parse_youtube_url() {
        let embed = EmbedDetector::parse("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
        assert!(embed.is_youtube());
        assert_eq!(embed.as_youtube().unwrap().video_id, "dQw4w9WgXcQ");
    }

    #[test]
    fn test_parse_spotify_url_full() {
        let embed = EmbedDetector::parse("https://open.spotify.com/track/123").unwrap();
        assert!(embed.is_spotify());
        assert_eq!(embed.as_spotify().unwrap().id, "123");
    }

    #[test]
    fn test_parse_generic_url() {
        let embed = EmbedDetector::parse("https://example.com").unwrap();
        assert!(embed.is_link());
        assert_eq!(embed.as_link().unwrap().url(), "https://example.com");
    }

    #[test]
    fn test_embed_type_as_str() {
        assert_eq!(EmbedType::YouTube.as_str(), "youtube");
        assert_eq!(EmbedType::Spotify.as_str(), "spotify");
        assert_eq!(EmbedType::Link.as_str(), "link");
    }

    #[test]
    fn test_spotify_embed_type_as_str() {
        assert_eq!(SpotifyEmbedType::Track.as_str(), "track");
        assert_eq!(SpotifyEmbedType::Album.as_str(), "album");
        assert_eq!(SpotifyEmbedType::Playlist.as_str(), "playlist");
    }

    #[test]
    fn test_youtube_thumbnail_quality() {
        assert_eq!(YouTubeThumbnailQuality::Default.filename(), "default.jpg");
        assert_eq!(YouTubeThumbnailQuality::High.filename(), "hqdefault.jpg");
        assert_eq!(YouTubeThumbnailQuality::MaxRes.filename(), "maxresdefault.jpg");
    }

    #[test]
    fn test_external_embed_serialization() {
        let youtube = YouTubeEmbed::new("dQw4w9WgXcQ");
        let embed = ExternalEmbed::YouTube(youtube);

        let json = serde_json::to_string(&embed).unwrap();
        assert!(json.contains("youtube"));
        assert!(json.contains("dQw4w9WgXcQ"));

        let deserialized: ExternalEmbed = serde_json::from_str(&json).unwrap();
        assert!(deserialized.is_youtube());
    }

    #[test]
    fn test_embed_error_display() {
        let error = EmbedError::InvalidUrl("bad url".to_string());
        assert!(format!("{}", error).contains("Invalid URL"));

        let error = EmbedError::MissingId;
        assert!(format!("{}", error).contains("Missing embed ID"));
    }
}

//! Link preview generation and metadata fetching
//!
//! This module provides functionality for generating rich link previews with
//! titles, descriptions, and images from URLs. It supports Open Graph, Twitter Cards,
//! and standard HTML metadata.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during link preview operations
#[derive(Debug, Error)]
pub enum LinkPreviewError {
    /// HTTP error
    #[error("HTTP error: {0}")]
    Http(String),

    /// Invalid URL
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Parsing error
    #[error("Parsing error: {0}")]
    Parse(String),

    /// No metadata found
    #[error("No metadata found for URL")]
    NoMetadata,

    /// Timeout
    #[error("Request timeout")]
    Timeout,

    /// Too large
    #[error("Content too large: {size} bytes exceeds maximum {max}")]
    TooLarge {
        /// Actual content size
        size: usize,
        /// Maximum allowed size
        max: usize,
    },
}

/// Result type for link preview operations
pub type Result<T> = std::result::Result<T, LinkPreviewError>;

/// Maximum HTML size to fetch (1 MB)
pub const MAX_HTML_SIZE: usize = 1_024_000;

/// Maximum number of redirects to follow
pub const MAX_REDIRECTS: usize = 5;

/// Link preview metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkPreview {
    /// Original URL
    pub url: String,
    /// Final URL after redirects
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_url: Option<String>,
    /// Page title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Page description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Preview image URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// Site name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_name: Option<String>,
    /// Favicon URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
    /// Content type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

impl LinkPreview {
    /// Create a new link preview
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            final_url: None,
            title: None,
            description: None,
            image: None,
            site_name: None,
            favicon: None,
            content_type: None,
        }
    }

    /// Check if the preview has any metadata
    pub fn has_metadata(&self) -> bool {
        self.title.is_some()
            || self.description.is_some()
            || self.image.is_some()
            || self.site_name.is_some()
    }

    /// Get display title (title or site name)
    pub fn display_title(&self) -> Option<&str> {
        self.title
            .as_deref()
            .or_else(|| self.site_name.as_deref())
    }

    /// Get display URL (hostname from final URL or original URL)
    pub fn display_url(&self) -> String {
        let url_str = self.final_url.as_ref().unwrap_or(&self.url);
        Self::extract_hostname(url_str).unwrap_or(url_str.clone())
    }

    /// Extract hostname from URL
    fn extract_hostname(url: &str) -> Option<String> {
        url.strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .and_then(|s| s.split('/').next())
            .map(|s| s.to_string())
    }

    /// Truncate description to a maximum length
    pub fn truncated_description(&self, max_length: usize) -> Option<String> {
        self.description.as_ref().map(|desc| {
            if desc.len() > max_length {
                format!("{}...", &desc[..max_length])
            } else {
                desc.clone()
            }
        })
    }
}

/// Raw HTML metadata extracted from a page
#[derive(Debug, Clone, Default)]
pub struct HtmlMetadata {
    /// Open Graph metadata (og:*)
    pub og: HashMap<String, String>,
    /// Twitter Card metadata (twitter:*)
    pub twitter: HashMap<String, String>,
    /// Standard HTML meta tags
    pub meta: HashMap<String, String>,
    /// Page title from <title> tag
    pub title: Option<String>,
}

impl HtmlMetadata {
    /// Create a new empty metadata collection
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a value from Open Graph metadata
    pub fn get_og(&self, key: &str) -> Option<&str> {
        self.og.get(key).map(|s| s.as_str())
    }

    /// Get a value from Twitter Card metadata
    pub fn get_twitter(&self, key: &str) -> Option<&str> {
        self.twitter.get(key).map(|s| s.as_str())
    }

    /// Get a value from standard meta tags
    pub fn get_meta(&self, key: &str) -> Option<&str> {
        self.meta.get(key).map(|s| s.as_str())
    }

    /// Convert to LinkPreview
    pub fn to_preview(&self, url: impl Into<String>) -> LinkPreview {
        let mut preview = LinkPreview::new(url);

        // Title: og:title > twitter:title > <title>
        preview.title = self
            .get_og("title")
            .or_else(|| self.get_twitter("title"))
            .or(self.title.as_deref())
            .map(|s| s.to_string());

        // Description: og:description > twitter:description > meta description
        preview.description = self
            .get_og("description")
            .or_else(|| self.get_twitter("description"))
            .or_else(|| self.get_meta("description"))
            .map(|s| s.to_string());

        // Image: og:image > twitter:image
        preview.image = self
            .get_og("image")
            .or_else(|| self.get_twitter("image"))
            .map(|s| s.to_string());

        // Site name: og:site_name
        preview.site_name = self.get_og("site_name").map(|s| s.to_string());

        preview
    }
}

/// Simple HTML parser for metadata extraction
///
/// This is a lightweight parser that extracts metadata without pulling in
/// a full HTML parsing library. It looks for specific meta tags and the title.
pub struct MetadataParser;

impl MetadataParser {
    /// Parse HTML and extract metadata
    pub fn parse(html: &str) -> HtmlMetadata {
        let mut metadata = HtmlMetadata::new();

        // Extract title
        if let Some(title) = Self::extract_title(html) {
            metadata.title = Some(title);
        }

        // Extract meta tags
        for (property, content) in Self::extract_meta_tags(html) {
            if let Some(og_key) = property.strip_prefix("og:") {
                metadata.og.insert(og_key.to_string(), content);
            } else if let Some(twitter_key) = property.strip_prefix("twitter:") {
                metadata.twitter.insert(twitter_key.to_string(), content);
            } else {
                metadata.meta.insert(property, content);
            }
        }

        // Also check for meta tags with name attribute
        for (name, content) in Self::extract_named_meta_tags(html) {
            if !metadata.meta.contains_key(&name) {
                metadata.meta.insert(name, content);
            }
        }

        metadata
    }

    /// Extract title from <title> tag
    fn extract_title(html: &str) -> Option<String> {
        let html_lower = html.to_lowercase();
        let start = html_lower.find("<title>")?;
        let end = html_lower[start..].find("</title>")?;
        let title = &html[start + 7..start + end];
        Some(Self::decode_html_entities(title.trim()))
    }

    /// Extract meta tags with property attribute
    fn extract_meta_tags(html: &str) -> Vec<(String, String)> {
        let mut tags = Vec::new();
        let html_lower = html.to_lowercase();
        let mut pos = 0;

        while let Some(start) = html_lower[pos..].find("<meta ") {
            let abs_start = pos + start;
            if let Some(end) = html_lower[abs_start..].find('>') {
                let tag = &html[abs_start..abs_start + end];

                if let (Some(property), Some(content)) =
                    (Self::extract_attribute(tag, "property"), Self::extract_attribute(tag, "content"))
                {
                    tags.push((property, content));
                }

                pos = abs_start + end + 1;
            } else {
                break;
            }
        }

        tags
    }

    /// Extract meta tags with name attribute
    fn extract_named_meta_tags(html: &str) -> Vec<(String, String)> {
        let mut tags = Vec::new();
        let html_lower = html.to_lowercase();
        let mut pos = 0;

        while let Some(start) = html_lower[pos..].find("<meta ") {
            let abs_start = pos + start;
            if let Some(end) = html_lower[abs_start..].find('>') {
                let tag = &html[abs_start..abs_start + end];

                if let (Some(name), Some(content)) =
                    (Self::extract_attribute(tag, "name"), Self::extract_attribute(tag, "content"))
                {
                    tags.push((name, content));
                }

                pos = abs_start + end + 1;
            } else {
                break;
            }
        }

        tags
    }

    /// Extract attribute value from tag
    fn extract_attribute(tag: &str, attr_name: &str) -> Option<String> {
        let tag_lower = tag.to_lowercase();
        let pattern = format!("{}=\"", attr_name);
        let start = tag_lower.find(&pattern)?;
        let value_start = start + pattern.len();
        let value_end = tag[value_start..].find('"')?;
        Some(Self::decode_html_entities(&tag[value_start..value_start + value_end]))
    }

    /// Decode common HTML entities
    fn decode_html_entities(text: &str) -> String {
        text.replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&apos;", "'")
    }
}

/// Link preview cache entry
#[derive(Debug, Clone)]
pub struct CachedPreview {
    /// The preview
    pub preview: LinkPreview,
    /// Timestamp when cached
    pub cached_at: std::time::Instant,
}

impl CachedPreview {
    /// Create a new cached preview
    pub fn new(preview: LinkPreview) -> Self {
        Self {
            preview,
            cached_at: std::time::Instant::now(),
        }
    }

    /// Check if the cached preview has expired
    pub fn is_expired(&self, ttl: std::time::Duration) -> bool {
        self.cached_at.elapsed() > ttl
    }
}

/// Link preview cache
#[derive(Debug, Clone, Default)]
pub struct PreviewCache {
    cache: HashMap<String, CachedPreview>,
    ttl: std::time::Duration,
}

impl PreviewCache {
    /// Create a new preview cache with default TTL (1 hour)
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            ttl: std::time::Duration::from_secs(3600),
        }
    }

    /// Create a new preview cache with custom TTL
    pub fn with_ttl(ttl: std::time::Duration) -> Self {
        Self {
            cache: HashMap::new(),
            ttl,
        }
    }

    /// Get a preview from cache
    pub fn get(&mut self, url: &str) -> Option<&LinkPreview> {
        self.cleanup_expired();
        self.cache.get(url).and_then(|entry| {
            if entry.is_expired(self.ttl) {
                None
            } else {
                Some(&entry.preview)
            }
        })
    }

    /// Store a preview in cache
    pub fn insert(&mut self, url: impl Into<String>, preview: LinkPreview) {
        self.cache.insert(url.into(), CachedPreview::new(preview));
    }

    /// Remove expired entries
    pub fn cleanup_expired(&mut self) {
        self.cache.retain(|_, entry| !entry.is_expired(self.ttl));
    }

    /// Clear all cached previews
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Get number of cached previews
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_preview_creation() {
        let preview = LinkPreview::new("https://example.com");
        assert_eq!(preview.url, "https://example.com");
        assert!(!preview.has_metadata());
    }

    #[test]
    fn test_link_preview_has_metadata() {
        let mut preview = LinkPreview::new("https://example.com");
        assert!(!preview.has_metadata());

        preview.title = Some("Example".to_string());
        assert!(preview.has_metadata());
    }

    #[test]
    fn test_link_preview_display_title() {
        let mut preview = LinkPreview::new("https://example.com");
        assert_eq!(preview.display_title(), None);

        preview.site_name = Some("Example Site".to_string());
        assert_eq!(preview.display_title(), Some("Example Site"));

        preview.title = Some("Page Title".to_string());
        assert_eq!(preview.display_title(), Some("Page Title"));
    }

    #[test]
    fn test_link_preview_display_url() {
        let preview = LinkPreview::new("https://example.com/path");
        assert_eq!(preview.display_url(), "example.com");

        let mut preview2 = LinkPreview::new("https://example.com/path");
        preview2.final_url = Some("https://www.example.org/final".to_string());
        assert_eq!(preview2.display_url(), "www.example.org");
    }

    #[test]
    fn test_link_preview_truncated_description() {
        let mut preview = LinkPreview::new("https://example.com");
        preview.description = Some("This is a very long description that should be truncated".to_string());

        assert_eq!(
            preview.truncated_description(20),
            Some("This is a very long ...".to_string())
        );

        assert_eq!(
            preview.truncated_description(100),
            Some("This is a very long description that should be truncated".to_string())
        );
    }

    #[test]
    fn test_html_metadata_creation() {
        let metadata = HtmlMetadata::new();
        assert!(metadata.og.is_empty());
        assert!(metadata.twitter.is_empty());
        assert!(metadata.meta.is_empty());
        assert!(metadata.title.is_none());
    }

    #[test]
    fn test_html_metadata_getters() {
        let mut metadata = HtmlMetadata::new();
        metadata.og.insert("title".to_string(), "OG Title".to_string());
        metadata.twitter.insert("card".to_string(), "summary".to_string());
        metadata.meta.insert("description".to_string(), "Meta Description".to_string());

        assert_eq!(metadata.get_og("title"), Some("OG Title"));
        assert_eq!(metadata.get_twitter("card"), Some("summary"));
        assert_eq!(metadata.get_meta("description"), Some("Meta Description"));
        assert_eq!(metadata.get_og("nonexistent"), None);
    }

    #[test]
    fn test_metadata_to_preview() {
        let mut metadata = HtmlMetadata::new();
        metadata.og.insert("title".to_string(), "OG Title".to_string());
        metadata.og.insert("description".to_string(), "OG Description".to_string());
        metadata.og.insert("image".to_string(), "https://example.com/image.jpg".to_string());
        metadata.og.insert("site_name".to_string(), "Example Site".to_string());

        let preview = metadata.to_preview("https://example.com");
        assert_eq!(preview.url, "https://example.com");
        assert_eq!(preview.title, Some("OG Title".to_string()));
        assert_eq!(preview.description, Some("OG Description".to_string()));
        assert_eq!(preview.image, Some("https://example.com/image.jpg".to_string()));
        assert_eq!(preview.site_name, Some("Example Site".to_string()));
    }

    #[test]
    fn test_parse_title() {
        let html = "<html><head><title>Test Page</title></head><body></body></html>";
        let metadata = MetadataParser::parse(html);
        assert_eq!(metadata.title, Some("Test Page".to_string()));
    }

    #[test]
    fn test_parse_og_tags() {
        let html = r#"<meta property="og:title" content="OG Title"><meta property="og:description" content="OG Desc">"#;
        let metadata = MetadataParser::parse(html);
        assert_eq!(metadata.get_og("title"), Some("OG Title"));
        assert_eq!(metadata.get_og("description"), Some("OG Desc"));
    }

    #[test]
    fn test_parse_twitter_tags() {
        let html = r#"<meta property="twitter:card" content="summary"><meta property="twitter:title" content="Twitter Title">"#;
        let metadata = MetadataParser::parse(html);
        assert_eq!(metadata.get_twitter("card"), Some("summary"));
        assert_eq!(metadata.get_twitter("title"), Some("Twitter Title"));
    }

    #[test]
    fn test_parse_meta_name_tags() {
        let html = r#"<meta name="description" content="Meta Description">"#;
        let metadata = MetadataParser::parse(html);
        assert_eq!(metadata.get_meta("description"), Some("Meta Description"));
    }

    #[test]
    fn test_html_entity_decoding() {
        let html = "<title>Test &amp; Example &lt;tag&gt; &quot;quotes&quot;</title>";
        let metadata = MetadataParser::parse(html);
        assert_eq!(metadata.title, Some("Test & Example <tag> \"quotes\"".to_string()));
    }

    #[test]
    fn test_preview_cache_creation() {
        let cache = PreviewCache::new();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_preview_cache_insert_get() {
        let mut cache = PreviewCache::new();
        let mut preview = LinkPreview::new("https://example.com");
        preview.title = Some("Example".to_string());

        cache.insert("https://example.com", preview.clone());
        assert_eq!(cache.len(), 1);

        let cached = cache.get("https://example.com");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().title, Some("Example".to_string()));
    }

    #[test]
    fn test_preview_cache_miss() {
        let mut cache = PreviewCache::new();
        assert!(cache.get("https://example.com").is_none());
    }

    #[test]
    fn test_preview_cache_clear() {
        let mut cache = PreviewCache::new();
        cache.insert("https://example.com", LinkPreview::new("https://example.com"));
        assert_eq!(cache.len(), 1);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_preview_cache_expired() {
        let mut cache = PreviewCache::with_ttl(std::time::Duration::from_millis(10));
        cache.insert("https://example.com", LinkPreview::new("https://example.com"));

        // Should be cached initially
        assert!(cache.get("https://example.com").is_some());

        // Wait for expiration
        std::thread::sleep(std::time::Duration::from_millis(20));

        // Should be expired now
        assert!(cache.get("https://example.com").is_none());
    }

    #[test]
    fn test_link_preview_serialization() {
        let mut preview = LinkPreview::new("https://example.com");
        preview.title = Some("Example".to_string());
        preview.description = Some("An example site".to_string());

        let json = serde_json::to_string(&preview).unwrap();
        assert!(json.contains("example.com"));
        assert!(json.contains("Example"));

        let deserialized: LinkPreview = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.url, preview.url);
        assert_eq!(deserialized.title, preview.title);
    }

    #[test]
    fn test_link_preview_error_display() {
        let error = LinkPreviewError::InvalidUrl("not a url".to_string());
        assert!(format!("{}", error).contains("Invalid URL"));

        let error = LinkPreviewError::TooLarge { size: 2_000_000, max: 1_000_000 };
        assert!(format!("{}", error).contains("2000000"));
    }

    #[test]
    fn test_extract_hostname() {
        assert_eq!(
            LinkPreview::extract_hostname("https://example.com/path"),
            Some("example.com".to_string())
        );

        assert_eq!(
            LinkPreview::extract_hostname("http://www.example.org:8080/path"),
            Some("www.example.org:8080".to_string())
        );

        assert_eq!(
            LinkPreview::extract_hostname("not a url"),
            None
        );
    }

    #[test]
    fn test_metadata_precedence() {
        let mut metadata = HtmlMetadata::new();
        metadata.title = Some("HTML Title".to_string());
        metadata.og.insert("title".to_string(), "OG Title".to_string());
        metadata.twitter.insert("title".to_string(), "Twitter Title".to_string());

        let preview = metadata.to_preview("https://example.com");
        // OG should take precedence
        assert_eq!(preview.title, Some("OG Title".to_string()));
    }

    #[test]
    fn test_metadata_fallback() {
        let mut metadata = HtmlMetadata::new();
        metadata.title = Some("HTML Title".to_string());
        metadata.meta.insert("description".to_string(), "Meta Description".to_string());

        let preview = metadata.to_preview("https://example.com");
        // Should fall back to HTML title when no OG/Twitter
        assert_eq!(preview.title, Some("HTML Title".to_string()));
        assert_eq!(preview.description, Some("Meta Description".to_string()));
    }
}

//! Application-level persisted state schema
//!
//! This module defines the top-level structure for all persisted application state,
//! including sessions, preferences, and UI settings.

use serde::{Deserialize, Serialize};

/// Color mode preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ColorMode {
    /// Follow system color scheme
    #[default]
    System,
    /// Always use light mode
    Light,
    /// Always use dark mode
    Dark,
}

/// Language preferences
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguagePrefs {
    /// Primary language code (e.g., "en", "es", "fr")
    pub primary_language: String,
    /// List of additional languages in order of preference
    #[serde(default)]
    pub additional_languages: Vec<String>,
    /// Show content in all languages
    #[serde(default)]
    pub show_all_languages: bool,
}

impl Default for LanguagePrefs {
    fn default() -> Self {
        Self {
            primary_language: "en".to_string(),
            additional_languages: Vec::new(),
            show_all_languages: false,
        }
    }
}

/// Onboarding state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingState {
    /// Whether the user has completed onboarding
    #[serde(default)]
    pub completed: bool,
    /// Current onboarding step (if not completed)
    pub current_step: Option<String>,
    /// Whether the user has seen the welcome screen
    #[serde(default)]
    pub seen_welcome: bool,
}

/// Application-level persisted state
///
/// This structure represents all state that should be persisted across app restarts.
/// It includes session data, user preferences, and UI state.
///
/// # Schema Version
///
/// The current schema version is 1. When making breaking changes to this structure,
/// increment the version and provide a migration path.
///
/// # Example
///
/// ```rust
/// use storage::app_state::{AppPersistedState, ColorMode};
///
/// let state = AppPersistedState::default();
/// assert_eq!(state.color_mode, ColorMode::System);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPersistedState {
    /// Color mode preference (light, dark, or system)
    #[serde(default)]
    pub color_mode: ColorMode,

    /// Language preferences
    #[serde(default)]
    pub language_prefs: LanguagePrefs,

    /// Onboarding state
    #[serde(default)]
    pub onboarding: OnboardingState,

    /// Whether to show sensitive content warnings
    #[serde(default = "default_true")]
    pub show_content_warnings: bool,

    /// Whether to auto-play GIFs
    #[serde(default = "default_true")]
    pub auto_play_gifs: bool,

    /// Whether to auto-play videos
    #[serde(default)]
    pub auto_play_videos: bool,

    /// Whether to reduce motion animations
    #[serde(default)]
    pub reduce_motion: bool,

    /// Font size multiplier (1.0 = normal, 1.2 = 20% larger, etc.)
    #[serde(default = "default_font_size")]
    pub font_size: f32,

    /// Whether to enable haptic feedback (mobile)
    #[serde(default = "default_true")]
    pub haptic_feedback: bool,

    /// Whether to enable sound effects
    #[serde(default)]
    pub sound_effects: bool,

    /// Last selected feed URI
    pub last_feed_uri: Option<String>,

    /// Pinned feed URIs in order
    #[serde(default)]
    pub pinned_feeds: Vec<String>,

    /// Hidden/muted words and phrases
    #[serde(default)]
    pub muted_words: Vec<String>,

    /// Whether analytics are enabled
    #[serde(default = "default_true")]
    pub analytics_enabled: bool,

    /// Whether crash reporting is enabled
    #[serde(default = "default_true")]
    pub crash_reporting_enabled: bool,
}

fn default_true() -> bool {
    true
}

fn default_font_size() -> f32 {
    1.0
}

impl Default for AppPersistedState {
    fn default() -> Self {
        Self {
            color_mode: ColorMode::default(),
            language_prefs: LanguagePrefs::default(),
            onboarding: OnboardingState::default(),
            show_content_warnings: true,
            auto_play_gifs: true,
            auto_play_videos: false,
            reduce_motion: false,
            font_size: 1.0,
            haptic_feedback: true,
            sound_effects: false,
            last_feed_uri: None,
            pinned_feeds: Vec::new(),
            muted_words: Vec::new(),
            analytics_enabled: true,
            crash_reporting_enabled: true,
        }
    }
}

impl AppPersistedState {
    /// Create a new app state with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create app state with a specific color mode
    pub fn with_color_mode(mut self, mode: ColorMode) -> Self {
        self.color_mode = mode;
        self
    }

    /// Create app state with a specific language
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language_prefs.primary_language = language.into();
        self
    }

    /// Mark onboarding as completed
    pub fn complete_onboarding(mut self) -> Self {
        self.onboarding.completed = true;
        self.onboarding.current_step = None;
        self
    }

    /// Add a pinned feed
    pub fn add_pinned_feed(&mut self, uri: impl Into<String>) {
        let uri = uri.into();
        if !self.pinned_feeds.contains(&uri) {
            self.pinned_feeds.push(uri);
        }
    }

    /// Remove a pinned feed
    pub fn remove_pinned_feed(&mut self, uri: &str) {
        self.pinned_feeds.retain(|f| f != uri);
    }

    /// Reorder pinned feeds
    pub fn reorder_pinned_feeds(&mut self, from_index: usize, to_index: usize) {
        if from_index < self.pinned_feeds.len() && to_index < self.pinned_feeds.len() {
            let feed = self.pinned_feeds.remove(from_index);
            self.pinned_feeds.insert(to_index, feed);
        }
    }

    /// Add a muted word
    pub fn add_muted_word(&mut self, word: impl Into<String>) {
        let word = word.into();
        if !self.muted_words.contains(&word) {
            self.muted_words.push(word);
        }
    }

    /// Remove a muted word
    pub fn remove_muted_word(&mut self, word: &str) {
        self.muted_words.retain(|w| w != word);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_app_state() {
        let state = AppPersistedState::default();

        assert_eq!(state.color_mode, ColorMode::System);
        assert_eq!(state.language_prefs.primary_language, "en");
        assert!(state.show_content_warnings);
        assert!(state.auto_play_gifs);
        assert!(!state.auto_play_videos);
        assert!(!state.reduce_motion);
        assert_eq!(state.font_size, 1.0);
        assert!(state.haptic_feedback);
        assert!(!state.sound_effects);
        assert!(state.analytics_enabled);
        assert!(state.crash_reporting_enabled);
    }

    #[test]
    fn test_color_mode_serialization() {
        let modes = vec![ColorMode::System, ColorMode::Light, ColorMode::Dark];

        for mode in modes {
            let json = serde_json::to_string(&mode).unwrap();
            let deserialized: ColorMode = serde_json::from_str(&json).unwrap();
            assert_eq!(mode, deserialized);
        }
    }

    #[test]
    fn test_app_state_serialization() {
        let state = AppPersistedState {
            color_mode: ColorMode::Dark,
            language_prefs: LanguagePrefs {
                primary_language: "es".to_string(),
                additional_languages: vec!["en".to_string()],
                show_all_languages: false,
            },
            onboarding: OnboardingState {
                completed: true,
                current_step: None,
                seen_welcome: true,
            },
            show_content_warnings: false,
            auto_play_gifs: true,
            auto_play_videos: false,
            reduce_motion: true,
            font_size: 1.2,
            haptic_feedback: true,
            sound_effects: false,
            last_feed_uri: Some("at://did:plc:abc/app.bsky.feed.generator/foo".to_string()),
            pinned_feeds: vec![
                "at://did:plc:abc/app.bsky.feed.generator/feed1".to_string(),
                "at://did:plc:abc/app.bsky.feed.generator/feed2".to_string(),
            ],
            muted_words: vec!["spam".to_string(), "spoiler".to_string()],
            analytics_enabled: false,
            crash_reporting_enabled: true,
        };

        let json = serde_json::to_string_pretty(&state).unwrap();
        let deserialized: AppPersistedState = serde_json::from_str(&json).unwrap();

        assert_eq!(state.color_mode, deserialized.color_mode);
        assert_eq!(
            state.language_prefs.primary_language,
            deserialized.language_prefs.primary_language
        );
        assert_eq!(state.onboarding.completed, deserialized.onboarding.completed);
        assert_eq!(state.font_size, deserialized.font_size);
        assert_eq!(state.pinned_feeds, deserialized.pinned_feeds);
        assert_eq!(state.muted_words, deserialized.muted_words);
    }

    #[test]
    fn test_builder_pattern() {
        let state = AppPersistedState::new()
            .with_color_mode(ColorMode::Dark)
            .with_language("fr")
            .complete_onboarding();

        assert_eq!(state.color_mode, ColorMode::Dark);
        assert_eq!(state.language_prefs.primary_language, "fr");
        assert!(state.onboarding.completed);
    }

    #[test]
    fn test_add_remove_pinned_feed() {
        let mut state = AppPersistedState::default();

        state.add_pinned_feed("feed1");
        state.add_pinned_feed("feed2");
        assert_eq!(state.pinned_feeds.len(), 2);

        // Adding duplicate should not increase count
        state.add_pinned_feed("feed1");
        assert_eq!(state.pinned_feeds.len(), 2);

        state.remove_pinned_feed("feed1");
        assert_eq!(state.pinned_feeds.len(), 1);
        assert_eq!(state.pinned_feeds[0], "feed2");
    }

    #[test]
    fn test_reorder_pinned_feeds() {
        let mut state = AppPersistedState::default();
        state.add_pinned_feed("feed1");
        state.add_pinned_feed("feed2");
        state.add_pinned_feed("feed3");

        state.reorder_pinned_feeds(0, 2);
        assert_eq!(state.pinned_feeds, vec!["feed2", "feed3", "feed1"]);

        state.reorder_pinned_feeds(2, 0);
        assert_eq!(state.pinned_feeds, vec!["feed1", "feed2", "feed3"]);
    }

    #[test]
    fn test_add_remove_muted_word() {
        let mut state = AppPersistedState::default();

        state.add_muted_word("spam");
        state.add_muted_word("nsfw");
        assert_eq!(state.muted_words.len(), 2);

        // Adding duplicate should not increase count
        state.add_muted_word("spam");
        assert_eq!(state.muted_words.len(), 2);

        state.remove_muted_word("spam");
        assert_eq!(state.muted_words.len(), 1);
        assert_eq!(state.muted_words[0], "nsfw");
    }

    #[test]
    fn test_language_prefs_default() {
        let prefs = LanguagePrefs::default();
        assert_eq!(prefs.primary_language, "en");
        assert!(prefs.additional_languages.is_empty());
        assert!(!prefs.show_all_languages);
    }

    #[test]
    fn test_onboarding_state_default() {
        let onboarding = OnboardingState::default();
        assert!(!onboarding.completed);
        assert!(onboarding.current_step.is_none());
        assert!(!onboarding.seen_welcome);
    }
}

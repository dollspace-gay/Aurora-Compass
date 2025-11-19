//! Application-level persisted state schema
//!
//! This module defines the top-level structure for all persisted application state,
//! including sessions, preferences, and UI settings.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// Per-account preference overrides
///
/// This structure allows individual accounts to override global preferences.
/// All fields are optional - if None, the global preference is used.
///
/// # Use Cases
///
/// - Different theme per account (work vs personal)
/// - Different language per account
/// - Different content filters per account
/// - Account-specific pinned feeds
///
/// # Example
///
/// ```rust
/// use storage::app_state::{AccountPreferences, ColorMode};
///
/// let mut prefs = AccountPreferences::default();
/// prefs.color_mode = Some(ColorMode::Dark);
/// // Other fields remain None, so they use global preferences
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AccountPreferences {
    /// Account-specific color mode override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_mode: Option<ColorMode>,

    /// Account-specific language preferences override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_prefs: Option<LanguagePrefs>,

    /// Account-specific content warnings preference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_content_warnings: Option<bool>,

    /// Account-specific auto-play GIFs preference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_play_gifs: Option<bool>,

    /// Account-specific auto-play videos preference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_play_videos: Option<bool>,

    /// Account-specific reduce motion preference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_motion: Option<bool>,

    /// Account-specific font size override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f32>,

    /// Account-specific haptic feedback preference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub haptic_feedback: Option<bool>,

    /// Account-specific sound effects preference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sound_effects: Option<bool>,

    /// Account-specific pinned feeds (completely replaces global if set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned_feeds: Option<Vec<String>>,

    /// Account-specific muted words (completely replaces global if set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub muted_words: Option<Vec<String>>,
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

    /// Per-account preference overrides keyed by account DID
    /// Each account can have its own preferences that override the global ones
    #[serde(default)]
    pub account_preferences: HashMap<String, AccountPreferences>,
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
            account_preferences: HashMap::new(),
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

    // ========== Per-Account Preference Resolution ==========

    /// Get effective color mode for an account (with fallback to global)
    ///
    /// Returns the account-specific preference if set, otherwise the global preference.
    ///
    /// # Arguments
    ///
    /// * `did` - Optional account DID. If None, returns global preference.
    pub fn get_color_mode_for_account(&self, did: Option<&str>) -> ColorMode {
        did.and_then(|d| self.account_preferences.get(d))
            .and_then(|prefs| prefs.color_mode)
            .unwrap_or(self.color_mode)
    }

    /// Get effective language preferences for an account (with fallback to global)
    pub fn get_language_prefs_for_account(&self, did: Option<&str>) -> &LanguagePrefs {
        did.and_then(|d| self.account_preferences.get(d))
            .and_then(|prefs| prefs.language_prefs.as_ref())
            .unwrap_or(&self.language_prefs)
    }

    /// Get effective content warnings preference for an account
    pub fn get_show_content_warnings_for_account(&self, did: Option<&str>) -> bool {
        did.and_then(|d| self.account_preferences.get(d))
            .and_then(|prefs| prefs.show_content_warnings)
            .unwrap_or(self.show_content_warnings)
    }

    /// Get effective auto-play GIFs preference for an account
    pub fn get_auto_play_gifs_for_account(&self, did: Option<&str>) -> bool {
        did.and_then(|d| self.account_preferences.get(d))
            .and_then(|prefs| prefs.auto_play_gifs)
            .unwrap_or(self.auto_play_gifs)
    }

    /// Get effective auto-play videos preference for an account
    pub fn get_auto_play_videos_for_account(&self, did: Option<&str>) -> bool {
        did.and_then(|d| self.account_preferences.get(d))
            .and_then(|prefs| prefs.auto_play_videos)
            .unwrap_or(self.auto_play_videos)
    }

    /// Get effective reduce motion preference for an account
    pub fn get_reduce_motion_for_account(&self, did: Option<&str>) -> bool {
        did.and_then(|d| self.account_preferences.get(d))
            .and_then(|prefs| prefs.reduce_motion)
            .unwrap_or(self.reduce_motion)
    }

    /// Get effective font size for an account
    pub fn get_font_size_for_account(&self, did: Option<&str>) -> f32 {
        did.and_then(|d| self.account_preferences.get(d))
            .and_then(|prefs| prefs.font_size)
            .unwrap_or(self.font_size)
    }

    /// Get effective haptic feedback preference for an account
    pub fn get_haptic_feedback_for_account(&self, did: Option<&str>) -> bool {
        did.and_then(|d| self.account_preferences.get(d))
            .and_then(|prefs| prefs.haptic_feedback)
            .unwrap_or(self.haptic_feedback)
    }

    /// Get effective sound effects preference for an account
    pub fn get_sound_effects_for_account(&self, did: Option<&str>) -> bool {
        did.and_then(|d| self.account_preferences.get(d))
            .and_then(|prefs| prefs.sound_effects)
            .unwrap_or(self.sound_effects)
    }

    /// Get effective pinned feeds for an account
    ///
    /// If account has specific pinned feeds, returns those.
    /// Otherwise returns global pinned feeds.
    pub fn get_pinned_feeds_for_account(&self, did: Option<&str>) -> &[String] {
        did.and_then(|d| self.account_preferences.get(d))
            .and_then(|prefs| prefs.pinned_feeds.as_deref())
            .unwrap_or(&self.pinned_feeds)
    }

    /// Get effective muted words for an account
    ///
    /// If account has specific muted words, returns those.
    /// Otherwise returns global muted words.
    pub fn get_muted_words_for_account(&self, did: Option<&str>) -> &[String] {
        did.and_then(|d| self.account_preferences.get(d))
            .and_then(|prefs| prefs.muted_words.as_deref())
            .unwrap_or(&self.muted_words)
    }

    /// Check if an account has any preference overrides
    pub fn has_account_overrides(&self, did: &str) -> bool {
        self.account_preferences.contains_key(did)
    }

    /// Clear all preference overrides for an account (will use globals)
    pub fn clear_account_preferences(&mut self, did: &str) {
        self.account_preferences.remove(did);
    }

    // ========== Per-Account Preference Setters ==========

    /// Set account-specific color mode override
    ///
    /// # Arguments
    ///
    /// * `did` - Account DID
    /// * `mode` - Color mode override, or None to use global
    pub fn set_account_color_mode(&mut self, did: &str, mode: Option<ColorMode>) {
        let prefs = self.account_preferences.entry(did.to_string()).or_default();
        prefs.color_mode = mode;
        self.cleanup_account_prefs_if_empty(did);
    }

    /// Set account-specific language preferences override
    pub fn set_account_language_prefs(&mut self, did: &str, prefs: Option<LanguagePrefs>) {
        let account_prefs = self.account_preferences.entry(did.to_string()).or_default();
        account_prefs.language_prefs = prefs;
        self.cleanup_account_prefs_if_empty(did);
    }

    /// Set account-specific content warnings preference
    pub fn set_account_show_content_warnings(&mut self, did: &str, enabled: Option<bool>) {
        let prefs = self.account_preferences.entry(did.to_string()).or_default();
        prefs.show_content_warnings = enabled;
        self.cleanup_account_prefs_if_empty(did);
    }

    /// Set account-specific auto-play GIFs preference
    pub fn set_account_auto_play_gifs(&mut self, did: &str, enabled: Option<bool>) {
        let prefs = self.account_preferences.entry(did.to_string()).or_default();
        prefs.auto_play_gifs = enabled;
        self.cleanup_account_prefs_if_empty(did);
    }

    /// Set account-specific auto-play videos preference
    pub fn set_account_auto_play_videos(&mut self, did: &str, enabled: Option<bool>) {
        let prefs = self.account_preferences.entry(did.to_string()).or_default();
        prefs.auto_play_videos = enabled;
        self.cleanup_account_prefs_if_empty(did);
    }

    /// Set account-specific reduce motion preference
    pub fn set_account_reduce_motion(&mut self, did: &str, enabled: Option<bool>) {
        let prefs = self.account_preferences.entry(did.to_string()).or_default();
        prefs.reduce_motion = enabled;
        self.cleanup_account_prefs_if_empty(did);
    }

    /// Set account-specific font size
    pub fn set_account_font_size(&mut self, did: &str, size: Option<f32>) {
        let prefs = self.account_preferences.entry(did.to_string()).or_default();
        prefs.font_size = size;
        self.cleanup_account_prefs_if_empty(did);
    }

    /// Set account-specific haptic feedback preference
    pub fn set_account_haptic_feedback(&mut self, did: &str, enabled: Option<bool>) {
        let prefs = self.account_preferences.entry(did.to_string()).or_default();
        prefs.haptic_feedback = enabled;
        self.cleanup_account_prefs_if_empty(did);
    }

    /// Set account-specific sound effects preference
    pub fn set_account_sound_effects(&mut self, did: &str, enabled: Option<bool>) {
        let prefs = self.account_preferences.entry(did.to_string()).or_default();
        prefs.sound_effects = enabled;
        self.cleanup_account_prefs_if_empty(did);
    }

    /// Set account-specific pinned feeds (completely replaces any existing)
    pub fn set_account_pinned_feeds(&mut self, did: &str, feeds: Option<Vec<String>>) {
        let prefs = self.account_preferences.entry(did.to_string()).or_default();
        prefs.pinned_feeds = feeds;
        self.cleanup_account_prefs_if_empty(did);
    }

    /// Set account-specific muted words (completely replaces any existing)
    pub fn set_account_muted_words(&mut self, did: &str, words: Option<Vec<String>>) {
        let prefs = self.account_preferences.entry(did.to_string()).or_default();
        prefs.muted_words = words;
        self.cleanup_account_prefs_if_empty(did);
    }

    /// Remove account preferences entry if all fields are None
    ///
    /// This keeps the preferences map clean by removing empty entries
    fn cleanup_account_prefs_if_empty(&mut self, did: &str) {
        if let Some(prefs) = self.account_preferences.get(did) {
            if prefs.color_mode.is_none()
                && prefs.language_prefs.is_none()
                && prefs.show_content_warnings.is_none()
                && prefs.auto_play_gifs.is_none()
                && prefs.auto_play_videos.is_none()
                && prefs.reduce_motion.is_none()
                && prefs.font_size.is_none()
                && prefs.haptic_feedback.is_none()
                && prefs.sound_effects.is_none()
                && prefs.pinned_feeds.is_none()
                && prefs.muted_words.is_none()
            {
                self.account_preferences.remove(did);
            }
        }
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
            account_preferences: HashMap::new(),
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

    // ========== Per-Account Preferences Tests ==========

    #[test]
    fn test_account_preferences_default() {
        let prefs = AccountPreferences::default();
        assert!(prefs.color_mode.is_none());
        assert!(prefs.language_prefs.is_none());
        assert!(prefs.show_content_warnings.is_none());
        assert!(prefs.auto_play_gifs.is_none());
        assert!(prefs.pinned_feeds.is_none());
        assert!(prefs.muted_words.is_none());
    }

    #[test]
    fn test_color_mode_resolution_no_override() {
        let mut state = AppPersistedState::default();
        state.color_mode = ColorMode::Dark;

        // No account preference set, should use global
        assert_eq!(state.get_color_mode_for_account(Some("did:plc:test")), ColorMode::Dark);
        assert_eq!(state.get_color_mode_for_account(None), ColorMode::Dark);
    }

    #[test]
    fn test_color_mode_resolution_with_override() {
        let mut state = AppPersistedState::default();
        state.color_mode = ColorMode::Light;

        // Set account override
        state.set_account_color_mode("did:plc:work", Some(ColorMode::Dark));

        // Account with override should use override
        assert_eq!(state.get_color_mode_for_account(Some("did:plc:work")), ColorMode::Dark);

        // Account without override should use global
        assert_eq!(state.get_color_mode_for_account(Some("did:plc:personal")), ColorMode::Light);

        // No account should use global
        assert_eq!(state.get_color_mode_for_account(None), ColorMode::Light);
    }

    #[test]
    fn test_clear_account_override() {
        let mut state = AppPersistedState::default();
        state.color_mode = ColorMode::Light;

        // Set override
        state.set_account_color_mode("did:plc:test", Some(ColorMode::Dark));
        assert_eq!(state.get_color_mode_for_account(Some("did:plc:test")), ColorMode::Dark);

        // Clear override by setting to None
        state.set_account_color_mode("did:plc:test", None);
        assert_eq!(state.get_color_mode_for_account(Some("did:plc:test")), ColorMode::Light);

        // Preferences entry should be cleaned up
        assert!(!state.has_account_overrides("did:plc:test"));
    }

    #[test]
    fn test_multiple_account_preferences() {
        let mut state = AppPersistedState::default();
        state.color_mode = ColorMode::System;
        state.font_size = 1.0;

        // Set different preferences for different accounts
        state.set_account_color_mode("did:plc:work", Some(ColorMode::Light));
        state.set_account_font_size("did:plc:work", Some(1.2));

        state.set_account_color_mode("did:plc:personal", Some(ColorMode::Dark));
        state.set_account_font_size("did:plc:personal", Some(1.5));

        // Verify work account
        assert_eq!(state.get_color_mode_for_account(Some("did:plc:work")), ColorMode::Light);
        assert_eq!(state.get_font_size_for_account(Some("did:plc:work")), 1.2);

        // Verify personal account
        assert_eq!(state.get_color_mode_for_account(Some("did:plc:personal")), ColorMode::Dark);
        assert_eq!(state.get_font_size_for_account(Some("did:plc:personal")), 1.5);

        // Verify global fallback
        assert_eq!(state.get_color_mode_for_account(None), ColorMode::System);
        assert_eq!(state.get_font_size_for_account(None), 1.0);
    }

    #[test]
    fn test_account_pinned_feeds_override() {
        let mut state = AppPersistedState::default();
        state.pinned_feeds = vec!["global1".to_string(), "global2".to_string()];

        // Set account-specific pinned feeds
        state.set_account_pinned_feeds(
            "did:plc:work",
            Some(vec!["work1".to_string(), "work2".to_string()]),
        );

        // Work account should use its own feeds
        assert_eq!(
            state.get_pinned_feeds_for_account(Some("did:plc:work")),
            &["work1".to_string(), "work2".to_string()]
        );

        // Other accounts should use global
        assert_eq!(
            state.get_pinned_feeds_for_account(Some("did:plc:personal")),
            &["global1".to_string(), "global2".to_string()]
        );

        // No account should use global
        assert_eq!(
            state.get_pinned_feeds_for_account(None),
            &["global1".to_string(), "global2".to_string()]
        );
    }

    #[test]
    fn test_language_prefs_override() {
        let mut state = AppPersistedState::default();
        state.language_prefs.primary_language = "en".to_string();

        // Set Spanish for work account
        let mut work_lang = LanguagePrefs::default();
        work_lang.primary_language = "es".to_string();
        state.set_account_language_prefs("did:plc:work", Some(work_lang));

        // Work account uses Spanish
        assert_eq!(
            state.get_language_prefs_for_account(Some("did:plc:work")).primary_language,
            "es"
        );

        // Other accounts use English
        assert_eq!(
            state.get_language_prefs_for_account(Some("did:plc:personal")).primary_language,
            "en"
        );
    }

    #[test]
    fn test_has_account_overrides() {
        let mut state = AppPersistedState::default();

        assert!(!state.has_account_overrides("did:plc:test"));

        state.set_account_color_mode("did:plc:test", Some(ColorMode::Dark));
        assert!(state.has_account_overrides("did:plc:test"));

        state.clear_account_preferences("did:plc:test");
        assert!(!state.has_account_overrides("did:plc:test"));
    }

    #[test]
    fn test_clear_all_account_preferences() {
        let mut state = AppPersistedState::default();

        // Set multiple preferences
        state.set_account_color_mode("did:plc:test", Some(ColorMode::Dark));
        state.set_account_font_size("did:plc:test", Some(1.5));
        state.set_account_auto_play_gifs("did:plc:test", Some(false));

        assert!(state.has_account_overrides("did:plc:test"));

        // Clear all
        state.clear_account_preferences("did:plc:test");
        assert!(!state.has_account_overrides("did:plc:test"));

        // Should now use globals
        assert_eq!(state.get_color_mode_for_account(Some("did:plc:test")), state.color_mode);
        assert_eq!(state.get_font_size_for_account(Some("did:plc:test")), state.font_size);
        assert_eq!(
            state.get_auto_play_gifs_for_account(Some("did:plc:test")),
            state.auto_play_gifs
        );
    }

    #[test]
    fn test_cleanup_empty_account_prefs() {
        let mut state = AppPersistedState::default();

        // Set a preference
        state.set_account_color_mode("did:plc:test", Some(ColorMode::Dark));
        assert!(state.has_account_overrides("did:plc:test"));

        // Clear it by setting to None
        state.set_account_color_mode("did:plc:test", None);

        // Entry should be automatically cleaned up
        assert!(!state.has_account_overrides("did:plc:test"));
        assert!(!state.account_preferences.contains_key("did:plc:test"));
    }

    #[test]
    fn test_account_preferences_serialization() {
        let mut state = AppPersistedState::default();

        // Set account preferences
        state.set_account_color_mode("did:plc:work", Some(ColorMode::Dark));
        state.set_account_font_size("did:plc:work", Some(1.2));

        // Serialize
        let json = serde_json::to_string_pretty(&state).unwrap();

        // Verify account preferences are in JSON
        assert!(json.contains("accountPreferences"));
        assert!(json.contains("did:plc:work"));

        // Deserialize
        let deserialized: AppPersistedState = serde_json::from_str(&json).unwrap();

        // Verify preferences survived round-trip
        assert_eq!(
            deserialized.get_color_mode_for_account(Some("did:plc:work")),
            ColorMode::Dark
        );
        assert_eq!(deserialized.get_font_size_for_account(Some("did:plc:work")), 1.2);
    }

    #[test]
    fn test_account_preferences_not_serialized_when_empty() {
        let state = AppPersistedState::default();

        // Serialize
        let json = serde_json::to_string_pretty(&state).unwrap();

        // Empty account_preferences should serialize as empty object or not be present
        // (depends on serde default attribute)
        let deserialized: AppPersistedState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.account_preferences.len(), 0);
    }

    #[test]
    fn test_boolean_preference_overrides() {
        let mut state = AppPersistedState::default();
        state.show_content_warnings = true;
        state.auto_play_gifs = true;
        state.auto_play_videos = false;

        // Set overrides for work account
        state.set_account_show_content_warnings("did:plc:work", Some(false));
        state.set_account_auto_play_gifs("did:plc:work", Some(false));
        state.set_account_auto_play_videos("did:plc:work", Some(true));

        // Work account uses overrides
        assert!(!state.get_show_content_warnings_for_account(Some("did:plc:work")));
        assert!(!state.get_auto_play_gifs_for_account(Some("did:plc:work")));
        assert!(state.get_auto_play_videos_for_account(Some("did:plc:work")));

        // Other accounts use globals
        assert!(state.get_show_content_warnings_for_account(Some("did:plc:personal")));
        assert!(state.get_auto_play_gifs_for_account(Some("did:plc:personal")));
        assert!(!state.get_auto_play_videos_for_account(Some("did:plc:personal")));
    }

    #[test]
    fn test_muted_words_override() {
        let mut state = AppPersistedState::default();
        state.muted_words = vec!["global1".to_string(), "global2".to_string()];

        // Set account-specific muted words
        state.set_account_muted_words(
            "did:plc:work",
            Some(vec!["work1".to_string(), "work2".to_string(), "work3".to_string()]),
        );

        // Work account uses its own muted words
        assert_eq!(
            state.get_muted_words_for_account(Some("did:plc:work")),
            &["work1".to_string(), "work2".to_string(), "work3".to_string()]
        );

        // Other accounts use global
        assert_eq!(
            state.get_muted_words_for_account(Some("did:plc:personal")),
            &["global1".to_string(), "global2".to_string()]
        );
    }
}

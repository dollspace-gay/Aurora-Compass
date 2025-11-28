//! Content filtering system
//!
//! This module provides content filtering based on labels, user preferences,
//! muted words, and other moderation controls.

use crate::labels::{Label, LabelApplicator, LabelBehavior, LabelResult, LabelerSubscriptions};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

/// Errors that can occur during content filtering
#[derive(Debug, Error)]
pub enum FilterError {
    /// Invalid filter configuration
    #[error("Invalid filter configuration: {0}")]
    InvalidConfig(String),

    /// Word list too large
    #[error("Word list too large: {count} exceeds maximum {max}")]
    TooManyWords {
        /// Actual count
        count: usize,
        /// Maximum allowed
        max: usize,
    },
}

/// Result type for filter operations
pub type Result<T> = std::result::Result<T, FilterError>;

/// Maximum number of muted words allowed
pub const MAX_MUTED_WORDS: usize = 500;

/// Reason content was filtered
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FilterReason {
    /// Content has a label that the user has configured to hide
    Label(String),
    /// Content contains a muted word
    MutedWord(String),
    /// Content is from a muted account
    MutedAccount,
    /// Content is from a blocked account
    BlockedAccount,
    /// Content contains adult content and user has disabled adult content
    AdultContent,
    /// Content is a reply and user has hidden replies
    HiddenReplies,
    /// Content is a repost and user has hidden reposts
    HiddenReposts,
    /// Content is a quote post and user has hidden quote posts
    HiddenQuotePosts,
}

impl FilterReason {
    /// Get a human-readable description
    pub fn description(&self) -> String {
        match self {
            FilterReason::Label(label) => format!("Content labeled: {}", label),
            FilterReason::MutedWord(word) => format!("Contains muted word: {}", word),
            FilterReason::MutedAccount => "From muted account".to_string(),
            FilterReason::BlockedAccount => "From blocked account".to_string(),
            FilterReason::AdultContent => "Adult content".to_string(),
            FilterReason::HiddenReplies => "Reply hidden by preference".to_string(),
            FilterReason::HiddenReposts => "Repost hidden by preference".to_string(),
            FilterReason::HiddenQuotePosts => "Quote post hidden by preference".to_string(),
        }
    }
}

/// Action to take on filtered content
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterAction {
    /// Show content normally
    #[default]
    Show,
    /// Show with a warning overlay
    Warn,
    /// Hide behind a click-through
    Hide,
    /// Blur media but show text
    BlurMedia,
    /// Completely remove from view
    Remove,
}

impl FilterAction {
    /// Check if content should be visible by default
    pub fn is_visible(&self) -> bool {
        matches!(self, FilterAction::Show | FilterAction::Warn | FilterAction::BlurMedia)
    }

    /// Check if content requires user action to view
    pub fn requires_click(&self) -> bool {
        matches!(self, FilterAction::Warn | FilterAction::Hide)
    }

    /// Check if media should be blurred
    pub fn blur_media(&self) -> bool {
        matches!(self, FilterAction::BlurMedia | FilterAction::Hide)
    }

    /// Get the more restrictive of two actions
    pub fn more_restrictive(self, other: FilterAction) -> FilterAction {
        match (self.priority(), other.priority()) {
            (a, b) if a >= b => self,
            _ => other,
        }
    }

    /// Get priority (higher = more restrictive)
    fn priority(&self) -> u8 {
        match self {
            FilterAction::Show => 0,
            FilterAction::BlurMedia => 1,
            FilterAction::Warn => 2,
            FilterAction::Hide => 3,
            FilterAction::Remove => 4,
        }
    }
}

impl From<LabelBehavior> for FilterAction {
    fn from(behavior: LabelBehavior) -> Self {
        match behavior {
            LabelBehavior::Ignore => FilterAction::Show,
            LabelBehavior::Warn => FilterAction::Warn,
            LabelBehavior::Hide => FilterAction::Hide,
            LabelBehavior::Block => FilterAction::Remove,
            LabelBehavior::BlurMedia => FilterAction::BlurMedia,
        }
    }
}

/// Result of filtering content
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterResult {
    /// Action to take
    pub action: FilterAction,
    /// Reasons for filtering (may be multiple)
    pub reasons: Vec<FilterReason>,
    /// Warning message to display
    pub warning: Option<String>,
    /// Labels applied to content
    pub labels: Vec<Label>,
}

impl FilterResult {
    /// Create a result that shows content normally
    pub fn show() -> Self {
        Self {
            action: FilterAction::Show,
            reasons: Vec::new(),
            warning: None,
            labels: Vec::new(),
        }
    }

    /// Create a result that removes content
    pub fn remove(reason: FilterReason) -> Self {
        Self {
            action: FilterAction::Remove,
            reasons: vec![reason],
            warning: None,
            labels: Vec::new(),
        }
    }

    /// Check if content should be shown
    pub fn should_show(&self) -> bool {
        self.action.is_visible()
    }

    /// Check if content should be completely removed
    pub fn is_removed(&self) -> bool {
        matches!(self.action, FilterAction::Remove)
    }

    /// Check if content has any filter applied
    pub fn is_filtered(&self) -> bool {
        !matches!(self.action, FilterAction::Show)
    }

    /// Add a reason
    pub fn add_reason(&mut self, reason: FilterReason) {
        self.reasons.push(reason);
    }

    /// Update action if more restrictive
    pub fn apply_action(&mut self, action: FilterAction) {
        self.action = self.action.more_restrictive(action);
    }

    /// Merge with label result
    pub fn merge_label_result(&mut self, label_result: LabelResult) {
        let action = FilterAction::from(label_result.behavior);
        self.apply_action(action);

        if let Some(warning) = label_result.warning {
            self.warning = Some(warning);
        }

        for label in label_result.labels {
            self.reasons.push(FilterReason::Label(label.val.clone()));
            self.labels.push(label);
        }
    }
}

/// Content warning configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentWarningConfig {
    /// Whether to show content warnings
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Whether to require click-through for adult content
    #[serde(default = "default_true")]
    pub require_click_adult: bool,
    /// Whether to require click-through for graphic media
    #[serde(default = "default_true")]
    pub require_click_graphic: bool,
    /// Whether to blur media by default
    #[serde(default)]
    pub blur_media_default: bool,
    /// Custom warnings that are always shown
    #[serde(default)]
    pub custom_warnings: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl Default for ContentWarningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            require_click_adult: true,
            require_click_graphic: true,
            blur_media_default: false,
            custom_warnings: Vec::new(),
        }
    }
}

impl ContentWarningConfig {
    /// Create a new config with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a config that hides all sensitive content
    pub fn strict() -> Self {
        Self {
            enabled: true,
            require_click_adult: true,
            require_click_graphic: true,
            blur_media_default: true,
            custom_warnings: Vec::new(),
        }
    }

    /// Create a config that shows all content
    pub fn permissive() -> Self {
        Self {
            enabled: false,
            require_click_adult: false,
            require_click_graphic: false,
            blur_media_default: false,
            custom_warnings: Vec::new(),
        }
    }
}

/// User's content filter preferences
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FilterPreferences {
    /// Muted words
    #[serde(default)]
    pub muted_words: HashSet<String>,
    /// Muted accounts (DIDs)
    #[serde(default)]
    pub muted_accounts: HashSet<String>,
    /// Blocked accounts (DIDs)
    #[serde(default)]
    pub blocked_accounts: HashSet<String>,
    /// Whether to show adult content
    #[serde(default)]
    pub show_adult_content: bool,
    /// Whether to hide replies
    #[serde(default)]
    pub hide_replies: bool,
    /// Whether to hide reposts
    #[serde(default)]
    pub hide_reposts: bool,
    /// Whether to hide quote posts
    #[serde(default)]
    pub hide_quote_posts: bool,
    /// Content warning configuration
    #[serde(default)]
    pub content_warnings: ContentWarningConfig,
}

impl FilterPreferences {
    /// Create new default preferences
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a muted word
    pub fn mute_word(&mut self, word: impl Into<String>) -> Result<()> {
        if self.muted_words.len() >= MAX_MUTED_WORDS {
            return Err(FilterError::TooManyWords {
                count: self.muted_words.len() + 1,
                max: MAX_MUTED_WORDS,
            });
        }
        self.muted_words.insert(word.into().to_lowercase());
        Ok(())
    }

    /// Remove a muted word
    pub fn unmute_word(&mut self, word: &str) {
        self.muted_words.remove(&word.to_lowercase());
    }

    /// Check if a word is muted
    pub fn is_word_muted(&self, word: &str) -> bool {
        self.muted_words.contains(&word.to_lowercase())
    }

    /// Mute an account
    pub fn mute_account(&mut self, did: impl Into<String>) {
        self.muted_accounts.insert(did.into());
    }

    /// Unmute an account
    pub fn unmute_account(&mut self, did: &str) {
        self.muted_accounts.remove(did);
    }

    /// Check if an account is muted
    pub fn is_account_muted(&self, did: &str) -> bool {
        self.muted_accounts.contains(did)
    }

    /// Block an account
    pub fn block_account(&mut self, did: impl Into<String>) {
        self.blocked_accounts.insert(did.into());
    }

    /// Unblock an account
    pub fn unblock_account(&mut self, did: &str) {
        self.blocked_accounts.remove(did);
    }

    /// Check if an account is blocked
    pub fn is_account_blocked(&self, did: &str) -> bool {
        self.blocked_accounts.contains(did)
    }
}

/// Content to be filtered
#[derive(Debug, Clone)]
pub struct FilterableContent<'a> {
    /// Author DID
    pub author_did: &'a str,
    /// Text content (if any)
    pub text: Option<&'a str>,
    /// Labels applied to content
    pub labels: &'a [Label],
    /// Whether content contains adult material
    pub is_adult: bool,
    /// Whether this is a reply
    pub is_reply: bool,
    /// Whether this is a repost
    pub is_repost: bool,
    /// Whether this is a quote post
    pub is_quote_post: bool,
}

impl<'a> FilterableContent<'a> {
    /// Create new filterable content
    pub fn new(author_did: &'a str) -> Self {
        Self {
            author_did,
            text: None,
            labels: &[],
            is_adult: false,
            is_reply: false,
            is_repost: false,
            is_quote_post: false,
        }
    }

    /// Set text content
    pub fn with_text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }

    /// Set labels
    pub fn with_labels(mut self, labels: &'a [Label]) -> Self {
        self.labels = labels;
        self
    }

    /// Mark as adult content
    pub fn adult(mut self) -> Self {
        self.is_adult = true;
        self
    }

    /// Mark as reply
    pub fn reply(mut self) -> Self {
        self.is_reply = true;
        self
    }

    /// Mark as repost
    pub fn repost(mut self) -> Self {
        self.is_repost = true;
        self
    }

    /// Mark as quote post
    pub fn quote_post(mut self) -> Self {
        self.is_quote_post = true;
        self
    }
}

/// Content filter that applies all filtering rules
pub struct ContentFilter {
    /// User preferences
    preferences: FilterPreferences,
    /// Label applicator for label-based filtering
    label_applicator: LabelApplicator,
}

impl ContentFilter {
    /// Create a new content filter
    pub fn new(preferences: FilterPreferences, subscriptions: LabelerSubscriptions) -> Self {
        Self {
            preferences,
            label_applicator: LabelApplicator::new(subscriptions),
        }
    }

    /// Create a content filter with just preferences (no label subscriptions)
    pub fn with_preferences(preferences: FilterPreferences) -> Self {
        Self {
            preferences,
            label_applicator: LabelApplicator::new(LabelerSubscriptions::new()),
        }
    }

    /// Get mutable reference to label applicator
    pub fn label_applicator_mut(&mut self) -> &mut LabelApplicator {
        &mut self.label_applicator
    }

    /// Filter content
    pub fn filter(&self, content: &FilterableContent) -> FilterResult {
        let mut result = FilterResult::show();

        // Check blocked accounts first (most restrictive)
        if self.preferences.is_account_blocked(content.author_did) {
            return FilterResult::remove(FilterReason::BlockedAccount);
        }

        // Check muted accounts
        if self.preferences.is_account_muted(content.author_did) {
            result.apply_action(FilterAction::Remove);
            result.add_reason(FilterReason::MutedAccount);
            return result;
        }

        // Check content type preferences
        if content.is_reply && self.preferences.hide_replies {
            result.apply_action(FilterAction::Remove);
            result.add_reason(FilterReason::HiddenReplies);
            return result;
        }

        if content.is_repost && self.preferences.hide_reposts {
            result.apply_action(FilterAction::Remove);
            result.add_reason(FilterReason::HiddenReposts);
            return result;
        }

        if content.is_quote_post && self.preferences.hide_quote_posts {
            result.apply_action(FilterAction::Remove);
            result.add_reason(FilterReason::HiddenQuotePosts);
            return result;
        }

        // Check adult content
        if content.is_adult && !self.preferences.show_adult_content {
            result.apply_action(FilterAction::Hide);
            result.add_reason(FilterReason::AdultContent);
            result.warning = Some("Adult content".to_string());
        }

        // Check muted words
        if let Some(text) = content.text {
            if let Some(muted_word) = self.find_muted_word(text) {
                result.apply_action(FilterAction::Remove);
                result.add_reason(FilterReason::MutedWord(muted_word));
            }
        }

        // Apply label-based filtering
        let label_result = self.label_applicator.apply(content.labels);
        result.merge_label_result(label_result);

        result
    }

    /// Find the first muted word in text
    fn find_muted_word(&self, text: &str) -> Option<String> {
        let text_lower = text.to_lowercase();
        for word in &self.preferences.muted_words {
            if text_lower.contains(word) {
                return Some(word.clone());
            }
        }
        None
    }

    /// Check if text contains any muted words
    pub fn contains_muted_word(&self, text: &str) -> bool {
        self.find_muted_word(text).is_some()
    }

    /// Update preferences
    pub fn set_preferences(&mut self, preferences: FilterPreferences) {
        self.preferences = preferences;
    }

    /// Get current preferences
    pub fn preferences(&self) -> &FilterPreferences {
        &self.preferences
    }
}

/// Hide/show state for content
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContentVisibility {
    /// Content URIs that user has manually shown
    shown: HashSet<String>,
    /// Content URIs that user has manually hidden
    hidden: HashSet<String>,
}

impl ContentVisibility {
    /// Create new visibility state
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark content as shown
    pub fn show(&mut self, uri: impl Into<String>) {
        let uri = uri.into();
        self.hidden.remove(&uri);
        self.shown.insert(uri);
    }

    /// Mark content as hidden
    pub fn hide(&mut self, uri: impl Into<String>) {
        let uri = uri.into();
        self.shown.remove(&uri);
        self.hidden.insert(uri);
    }

    /// Reset visibility for content
    pub fn reset(&mut self, uri: &str) {
        self.shown.remove(uri);
        self.hidden.remove(uri);
    }

    /// Check if content has been manually shown
    pub fn is_shown(&self, uri: &str) -> bool {
        self.shown.contains(uri)
    }

    /// Check if content has been manually hidden
    pub fn is_hidden(&self, uri: &str) -> bool {
        self.hidden.contains(uri)
    }

    /// Apply visibility override to filter result
    pub fn apply(&self, uri: &str, mut result: FilterResult) -> FilterResult {
        if self.is_shown(uri) {
            result.action = FilterAction::Show;
        } else if self.is_hidden(uri) {
            result.action = FilterAction::Remove;
        }
        result
    }

    /// Clear all visibility overrides
    pub fn clear(&mut self) {
        self.shown.clear();
        self.hidden.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::labels::{LabelDefinition, LabelLocale};

    // Filter reason tests

    #[test]
    fn test_filter_reason_description() {
        assert!(FilterReason::Label("nsfw".to_string())
            .description()
            .contains("nsfw"));
        assert!(FilterReason::MutedWord("spam".to_string())
            .description()
            .contains("spam"));
        assert!(FilterReason::MutedAccount.description().contains("muted"));
        assert!(FilterReason::BlockedAccount
            .description()
            .contains("blocked"));
    }

    // Filter action tests

    #[test]
    fn test_filter_action_is_visible() {
        assert!(FilterAction::Show.is_visible());
        assert!(FilterAction::Warn.is_visible());
        assert!(FilterAction::BlurMedia.is_visible());
        assert!(!FilterAction::Hide.is_visible());
        assert!(!FilterAction::Remove.is_visible());
    }

    #[test]
    fn test_filter_action_requires_click() {
        assert!(!FilterAction::Show.requires_click());
        assert!(FilterAction::Warn.requires_click());
        assert!(FilterAction::Hide.requires_click());
        assert!(!FilterAction::Remove.requires_click());
    }

    #[test]
    fn test_filter_action_blur_media() {
        assert!(!FilterAction::Show.blur_media());
        assert!(FilterAction::BlurMedia.blur_media());
        assert!(FilterAction::Hide.blur_media());
    }

    #[test]
    fn test_filter_action_more_restrictive() {
        assert_eq!(FilterAction::Show.more_restrictive(FilterAction::Warn), FilterAction::Warn);
        assert_eq!(FilterAction::Hide.more_restrictive(FilterAction::Warn), FilterAction::Hide);
        assert_eq!(FilterAction::Remove.more_restrictive(FilterAction::Hide), FilterAction::Remove);
    }

    #[test]
    fn test_filter_action_from_label_behavior() {
        assert_eq!(FilterAction::from(LabelBehavior::Ignore), FilterAction::Show);
        assert_eq!(FilterAction::from(LabelBehavior::Warn), FilterAction::Warn);
        assert_eq!(FilterAction::from(LabelBehavior::Hide), FilterAction::Hide);
        assert_eq!(FilterAction::from(LabelBehavior::Block), FilterAction::Remove);
        assert_eq!(FilterAction::from(LabelBehavior::BlurMedia), FilterAction::BlurMedia);
    }

    // Filter result tests

    #[test]
    fn test_filter_result_show() {
        let result = FilterResult::show();
        assert!(result.should_show());
        assert!(!result.is_removed());
        assert!(!result.is_filtered());
    }

    #[test]
    fn test_filter_result_remove() {
        let result = FilterResult::remove(FilterReason::BlockedAccount);
        assert!(!result.should_show());
        assert!(result.is_removed());
        assert!(result.is_filtered());
        assert_eq!(result.reasons.len(), 1);
    }

    #[test]
    fn test_filter_result_apply_action() {
        let mut result = FilterResult::show();
        result.apply_action(FilterAction::Warn);
        assert_eq!(result.action, FilterAction::Warn);

        result.apply_action(FilterAction::Show);
        assert_eq!(result.action, FilterAction::Warn); // Should not downgrade
    }

    // Content warning config tests

    #[test]
    fn test_content_warning_config_default() {
        let config = ContentWarningConfig::new();
        assert!(config.enabled);
        assert!(config.require_click_adult);
        assert!(config.require_click_graphic);
        assert!(!config.blur_media_default);
    }

    #[test]
    fn test_content_warning_config_strict() {
        let config = ContentWarningConfig::strict();
        assert!(config.enabled);
        assert!(config.blur_media_default);
    }

    #[test]
    fn test_content_warning_config_permissive() {
        let config = ContentWarningConfig::permissive();
        assert!(!config.enabled);
        assert!(!config.require_click_adult);
    }

    // Filter preferences tests

    #[test]
    fn test_filter_preferences_muted_words() {
        let mut prefs = FilterPreferences::new();

        prefs.mute_word("spam").unwrap();
        assert!(prefs.is_word_muted("spam"));
        assert!(prefs.is_word_muted("SPAM")); // Case insensitive

        prefs.unmute_word("spam");
        assert!(!prefs.is_word_muted("spam"));
    }

    #[test]
    fn test_filter_preferences_muted_accounts() {
        let mut prefs = FilterPreferences::new();

        prefs.mute_account("did:plc:test");
        assert!(prefs.is_account_muted("did:plc:test"));

        prefs.unmute_account("did:plc:test");
        assert!(!prefs.is_account_muted("did:plc:test"));
    }

    #[test]
    fn test_filter_preferences_blocked_accounts() {
        let mut prefs = FilterPreferences::new();

        prefs.block_account("did:plc:test");
        assert!(prefs.is_account_blocked("did:plc:test"));

        prefs.unblock_account("did:plc:test");
        assert!(!prefs.is_account_blocked("did:plc:test"));
    }

    // Filterable content tests

    #[test]
    fn test_filterable_content_builder() {
        let content = FilterableContent::new("did:plc:author")
            .with_text("Hello world")
            .adult()
            .reply();

        assert_eq!(content.author_did, "did:plc:author");
        assert_eq!(content.text, Some("Hello world"));
        assert!(content.is_adult);
        assert!(content.is_reply);
        assert!(!content.is_repost);
    }

    // Content filter tests

    #[test]
    fn test_content_filter_blocked_account() {
        let mut prefs = FilterPreferences::new();
        prefs.block_account("did:plc:blocked");

        let filter = ContentFilter::with_preferences(prefs);
        let content = FilterableContent::new("did:plc:blocked");

        let result = filter.filter(&content);
        assert!(result.is_removed());
        assert!(matches!(result.reasons[0], FilterReason::BlockedAccount));
    }

    #[test]
    fn test_content_filter_muted_account() {
        let mut prefs = FilterPreferences::new();
        prefs.mute_account("did:plc:muted");

        let filter = ContentFilter::with_preferences(prefs);
        let content = FilterableContent::new("did:plc:muted");

        let result = filter.filter(&content);
        assert!(result.is_removed());
        assert!(matches!(result.reasons[0], FilterReason::MutedAccount));
    }

    #[test]
    fn test_content_filter_muted_word() {
        let mut prefs = FilterPreferences::new();
        prefs.mute_word("spoiler").unwrap();

        let filter = ContentFilter::with_preferences(prefs);
        let content =
            FilterableContent::new("did:plc:author").with_text("This contains a spoiler alert");

        let result = filter.filter(&content);
        assert!(result.is_removed());
        assert!(matches!(&result.reasons[0], FilterReason::MutedWord(w) if w == "spoiler"));
    }

    #[test]
    fn test_content_filter_adult_content() {
        let prefs = FilterPreferences::new(); // show_adult_content is false by default

        let filter = ContentFilter::with_preferences(prefs);
        let content = FilterableContent::new("did:plc:author").adult();

        let result = filter.filter(&content);
        assert_eq!(result.action, FilterAction::Hide);
        assert!(matches!(result.reasons[0], FilterReason::AdultContent));
    }

    #[test]
    fn test_content_filter_hidden_replies() {
        let mut prefs = FilterPreferences::new();
        prefs.hide_replies = true;

        let filter = ContentFilter::with_preferences(prefs);
        let content = FilterableContent::new("did:plc:author").reply();

        let result = filter.filter(&content);
        assert!(result.is_removed());
        assert!(matches!(result.reasons[0], FilterReason::HiddenReplies));
    }

    #[test]
    fn test_content_filter_hidden_reposts() {
        let mut prefs = FilterPreferences::new();
        prefs.hide_reposts = true;

        let filter = ContentFilter::with_preferences(prefs);
        let content = FilterableContent::new("did:plc:author").repost();

        let result = filter.filter(&content);
        assert!(result.is_removed());
        assert!(matches!(result.reasons[0], FilterReason::HiddenReposts));
    }

    #[test]
    fn test_content_filter_no_filter() {
        let prefs = FilterPreferences::new();
        let filter = ContentFilter::with_preferences(prefs);
        let content = FilterableContent::new("did:plc:author").with_text("Normal content");

        let result = filter.filter(&content);
        assert!(!result.is_filtered());
        assert!(result.should_show());
    }

    #[test]
    fn test_content_filter_with_labels() {
        let mut prefs = FilterPreferences::new();
        prefs.show_adult_content = true;

        let mut subs = LabelerSubscriptions::new();
        subs.subscribe("did:plc:labeler");

        let mut filter = ContentFilter::new(prefs, subs);
        filter.label_applicator_mut().add_definitions(
            "did:plc:labeler",
            vec![LabelDefinition {
                identifier: "nsfw".to_string(),
                locales: Some(vec![LabelLocale {
                    lang: "en".to_string(),
                    name: "Adult Content".to_string(),
                    description: None,
                }]),
                severity: 3,
                applies_to_account: false,
                applies_to_content: true,
                default_behavior: LabelBehavior::Hide,
                user_configurable: true,
            }],
        );

        let labels = vec![Label {
            src: "did:plc:labeler".to_string(),
            uri: "at://did:plc:user/post/123".to_string(),
            cid: None,
            val: "nsfw".to_string(),
            neg: false,
            cts: "2024-01-01T00:00:00Z".to_string(),
            exp: None,
            sig: None,
        }];

        let content = FilterableContent::new("did:plc:author").with_labels(&labels);

        let result = filter.filter(&content);
        assert_eq!(result.action, FilterAction::Hide);
        assert!(!result.labels.is_empty());
    }

    // Content visibility tests

    #[test]
    fn test_content_visibility_show() {
        let mut visibility = ContentVisibility::new();

        visibility.show("at://post/1");
        assert!(visibility.is_shown("at://post/1"));
        assert!(!visibility.is_hidden("at://post/1"));
    }

    #[test]
    fn test_content_visibility_hide() {
        let mut visibility = ContentVisibility::new();

        visibility.hide("at://post/1");
        assert!(visibility.is_hidden("at://post/1"));
        assert!(!visibility.is_shown("at://post/1"));
    }

    #[test]
    fn test_content_visibility_toggle() {
        let mut visibility = ContentVisibility::new();

        visibility.show("at://post/1");
        assert!(visibility.is_shown("at://post/1"));

        visibility.hide("at://post/1");
        assert!(visibility.is_hidden("at://post/1"));
        assert!(!visibility.is_shown("at://post/1"));
    }

    #[test]
    fn test_content_visibility_reset() {
        let mut visibility = ContentVisibility::new();

        visibility.show("at://post/1");
        visibility.reset("at://post/1");

        assert!(!visibility.is_shown("at://post/1"));
        assert!(!visibility.is_hidden("at://post/1"));
    }

    #[test]
    fn test_content_visibility_apply() {
        let mut visibility = ContentVisibility::new();
        visibility.show("at://post/1");

        let result = FilterResult::remove(FilterReason::MutedWord("test".to_string()));
        let overridden = visibility.apply("at://post/1", result);

        assert_eq!(overridden.action, FilterAction::Show);
    }

    #[test]
    fn test_content_visibility_clear() {
        let mut visibility = ContentVisibility::new();
        visibility.show("at://post/1");
        visibility.hide("at://post/2");

        visibility.clear();

        assert!(!visibility.is_shown("at://post/1"));
        assert!(!visibility.is_hidden("at://post/2"));
    }

    // Serialization tests

    #[test]
    fn test_filter_preferences_serialization() {
        let mut prefs = FilterPreferences::new();
        prefs.mute_word("test").unwrap();
        prefs.block_account("did:plc:blocked");

        let json = serde_json::to_string(&prefs).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("did:plc:blocked"));

        let deserialized: FilterPreferences = serde_json::from_str(&json).unwrap();
        assert!(deserialized.is_word_muted("test"));
        assert!(deserialized.is_account_blocked("did:plc:blocked"));
    }

    #[test]
    fn test_filter_action_serialization() {
        let action = FilterAction::Hide;
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, "\"hide\"");

        let deserialized: FilterAction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, FilterAction::Hide);
    }

    // Error tests

    #[test]
    fn test_filter_error_display() {
        let error = FilterError::InvalidConfig("bad config".to_string());
        assert!(format!("{}", error).contains("bad config"));

        let error = FilterError::TooManyWords { count: 600, max: 500 };
        assert!(format!("{}", error).contains("600"));
    }
}

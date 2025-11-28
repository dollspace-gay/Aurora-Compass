//! Label management and labeler services
//!
//! This module provides functionality for working with AT Protocol labeling services.
//! IMPORTANT: Unlike the official Bluesky client, subscription to Bluesky's moderation
//! service is OPTIONAL. Users can choose which labeling services to subscribe to.

use atproto_client::xrpc::{XrpcClient, XrpcRequest};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// Errors that can occur during label operations
#[derive(Debug, Error)]
pub enum LabelError {
    /// XRPC error
    #[error("XRPC error: {0}")]
    Xrpc(String),

    /// Labeler not found
    #[error("Labeler not found: {0}")]
    LabelerNotFound(String),

    /// Label not found
    #[error("Label not found: {0}")]
    LabelNotFound(String),

    /// Already subscribed
    #[error("Already subscribed to labeler: {0}")]
    AlreadySubscribed(String),

    /// Not subscribed
    #[error("Not subscribed to labeler: {0}")]
    NotSubscribed(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result type for label operations
pub type Result<T> = std::result::Result<T, LabelError>;

/// A content label applied to content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    /// Source DID of the labeler that applied this label
    pub src: String,
    /// URI of the labeled content
    pub uri: String,
    /// CID of the labeled content (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cid: Option<String>,
    /// Label value/identifier
    pub val: String,
    /// Whether this label negates a previous label
    #[serde(default)]
    pub neg: bool,
    /// Timestamp when label was created
    pub cts: String,
    /// Expiration timestamp (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<String>,
    /// Signature (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sig: Option<String>,
}

impl Label {
    /// Check if this label is expired
    pub fn is_expired(&self) -> bool {
        if let Some(exp) = &self.exp {
            // Simple string comparison works for ISO 8601 dates
            let now = chrono::Utc::now().to_rfc3339();
            exp < &now
        } else {
            false
        }
    }

    /// Check if this is a negation label
    pub fn is_negation(&self) -> bool {
        self.neg
    }

    /// Get the label value
    pub fn value(&self) -> &str {
        &self.val
    }
}

/// Behavior to apply when a label is encountered
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LabelBehavior {
    /// Show content normally (ignore label)
    #[default]
    Ignore,
    /// Show with a warning
    Warn,
    /// Hide content behind a click-through
    Hide,
    /// Completely block/remove content
    Block,
    /// Blur media but show text
    BlurMedia,
}

impl LabelBehavior {
    /// Check if this behavior blocks content
    pub fn is_blocking(&self) -> bool {
        matches!(self, LabelBehavior::Block)
    }

    /// Check if this behavior hides content
    pub fn is_hiding(&self) -> bool {
        matches!(self, LabelBehavior::Hide | LabelBehavior::Block)
    }

    /// Check if this behavior shows a warning
    pub fn is_warning(&self) -> bool {
        matches!(self, LabelBehavior::Warn | LabelBehavior::Hide)
    }
}

/// Definition of a label type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelDefinition {
    /// Label identifier
    pub identifier: String,
    /// Human-readable name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locales: Option<Vec<LabelLocale>>,
    /// Severity level (1-5, higher = more severe)
    #[serde(default = "default_severity")]
    pub severity: u8,
    /// Whether this label can be applied to accounts
    #[serde(default)]
    pub applies_to_account: bool,
    /// Whether this label can be applied to content
    #[serde(default = "default_true")]
    pub applies_to_content: bool,
    /// Default behavior for this label
    #[serde(default)]
    pub default_behavior: LabelBehavior,
    /// Whether users can override the behavior
    #[serde(default = "default_true")]
    pub user_configurable: bool,
}

fn default_severity() -> u8 {
    1
}

fn default_true() -> bool {
    true
}

impl LabelDefinition {
    /// Create a new label definition
    pub fn new(identifier: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            locales: None,
            severity: 1,
            applies_to_account: false,
            applies_to_content: true,
            default_behavior: LabelBehavior::Warn,
            user_configurable: true,
        }
    }

    /// Get localized name for the label
    pub fn get_name(&self, locale: &str) -> Option<&str> {
        self.locales.as_ref().and_then(|locales| {
            locales
                .iter()
                .find(|l| l.lang == locale)
                .or_else(|| locales.first())
                .map(|l| l.name.as_str())
        })
    }

    /// Get localized description for the label
    pub fn get_description(&self, locale: &str) -> Option<&str> {
        self.locales.as_ref().and_then(|locales| {
            locales
                .iter()
                .find(|l| l.lang == locale)
                .or_else(|| locales.first())
                .and_then(|l| l.description.as_deref())
        })
    }
}

/// Localized label information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelLocale {
    /// Language code (e.g., "en", "es")
    pub lang: String,
    /// Localized name
    pub name: String,
    /// Localized description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A labeler service profile
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelerProfile {
    /// DID of the labeler
    pub did: String,
    /// Handle of the labeler
    pub handle: String,
    /// Display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Avatar URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// Number of subscribers
    #[serde(default)]
    pub subscriber_count: u64,
    /// Labels this labeler provides
    #[serde(default)]
    pub labels: Vec<LabelDefinition>,
    /// Whether the current user is subscribed
    #[serde(default)]
    pub is_subscribed: bool,
}

impl LabelerProfile {
    /// Create a new labeler profile
    pub fn new(did: impl Into<String>, handle: impl Into<String>) -> Self {
        Self {
            did: did.into(),
            handle: handle.into(),
            display_name: None,
            description: None,
            avatar: None,
            subscriber_count: 0,
            labels: Vec::new(),
            is_subscribed: false,
        }
    }

    /// Get the display name or handle
    pub fn name(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.handle)
    }
}

/// User preferences for a specific labeler
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LabelerPreferences {
    /// Custom behavior overrides for specific labels
    #[serde(default)]
    pub label_behaviors: HashMap<String, LabelBehavior>,
    /// Whether to show adult content from this labeler
    #[serde(default)]
    pub show_adult_content: bool,
}

impl LabelerPreferences {
    /// Create new default preferences
    pub fn new() -> Self {
        Self::default()
    }

    /// Get behavior for a specific label
    pub fn get_behavior(&self, label: &str, default: LabelBehavior) -> LabelBehavior {
        self.label_behaviors.get(label).copied().unwrap_or(default)
    }

    /// Set behavior for a specific label
    pub fn set_behavior(&mut self, label: impl Into<String>, behavior: LabelBehavior) {
        self.label_behaviors.insert(label.into(), behavior);
    }

    /// Reset behavior for a label to default
    pub fn reset_behavior(&mut self, label: &str) {
        self.label_behaviors.remove(label);
    }
}

/// Manager for labeler subscriptions
#[derive(Debug, Clone, Default)]
pub struct LabelerSubscriptions {
    /// Set of subscribed labeler DIDs
    subscribed: HashSet<String>,
    /// Preferences for each labeler
    preferences: HashMap<String, LabelerPreferences>,
}

impl LabelerSubscriptions {
    /// Create a new subscription manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if subscribed to a labeler
    pub fn is_subscribed(&self, did: &str) -> bool {
        self.subscribed.contains(did)
    }

    /// Subscribe to a labeler
    pub fn subscribe(&mut self, did: impl Into<String>) {
        let did = did.into();
        self.subscribed.insert(did.clone());
        self.preferences.entry(did).or_default();
    }

    /// Unsubscribe from a labeler
    pub fn unsubscribe(&mut self, did: &str) {
        self.subscribed.remove(did);
        self.preferences.remove(did);
    }

    /// Get list of subscribed labeler DIDs
    pub fn subscribed_labelers(&self) -> Vec<&str> {
        self.subscribed.iter().map(|s| s.as_str()).collect()
    }

    /// Get preferences for a labeler
    pub fn get_preferences(&self, did: &str) -> Option<&LabelerPreferences> {
        self.preferences.get(did)
    }

    /// Get mutable preferences for a labeler
    pub fn get_preferences_mut(&mut self, did: &str) -> Option<&mut LabelerPreferences> {
        self.preferences.get_mut(did)
    }

    /// Set preferences for a labeler
    pub fn set_preferences(&mut self, did: impl Into<String>, prefs: LabelerPreferences) {
        self.preferences.insert(did.into(), prefs);
    }

    /// Count of subscribed labelers
    pub fn count(&self) -> usize {
        self.subscribed.len()
    }

    /// Check if no labelers are subscribed
    pub fn is_empty(&self) -> bool {
        self.subscribed.is_empty()
    }
}

/// Result of applying labels to content
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelResult {
    /// The most restrictive behavior to apply
    pub behavior: LabelBehavior,
    /// All labels that apply to this content
    pub labels: Vec<Label>,
    /// Warning message to display (if any)
    pub warning: Option<String>,
}

impl LabelResult {
    /// Create a new label result with no labels
    pub fn none() -> Self {
        Self {
            behavior: LabelBehavior::Ignore,
            labels: Vec::new(),
            warning: None,
        }
    }

    /// Check if content should be shown
    pub fn should_show(&self) -> bool {
        !self.behavior.is_hiding()
    }

    /// Check if content should show a warning
    pub fn should_warn(&self) -> bool {
        self.behavior.is_warning()
    }

    /// Check if media should be blurred
    pub fn should_blur_media(&self) -> bool {
        matches!(self.behavior, LabelBehavior::BlurMedia)
    }
}

/// Label applicator that determines how labels affect content display
pub struct LabelApplicator {
    /// Subscribed labelers
    subscriptions: LabelerSubscriptions,
    /// Label definitions by labeler DID and label identifier
    definitions: HashMap<String, HashMap<String, LabelDefinition>>,
}

impl LabelApplicator {
    /// Create a new label applicator
    pub fn new(subscriptions: LabelerSubscriptions) -> Self {
        Self {
            subscriptions,
            definitions: HashMap::new(),
        }
    }

    /// Add label definitions for a labeler
    pub fn add_definitions(&mut self, labeler_did: impl Into<String>, definitions: Vec<LabelDefinition>) {
        let defs = self.definitions.entry(labeler_did.into()).or_default();
        for def in definitions {
            defs.insert(def.identifier.clone(), def);
        }
    }

    /// Apply labels to determine content behavior
    pub fn apply(&self, labels: &[Label]) -> LabelResult {
        let mut result = LabelResult::none();
        let mut most_severe_behavior = LabelBehavior::Ignore;
        let mut warnings = Vec::new();

        for label in labels {
            // Skip if not subscribed to this labeler
            if !self.subscriptions.is_subscribed(&label.src) {
                continue;
            }

            // Skip expired or negation labels
            if label.is_expired() || label.is_negation() {
                continue;
            }

            // Get label definition
            let behavior = self.get_label_behavior(&label.src, &label.val);

            // Track the most restrictive behavior
            if Self::behavior_priority(behavior) > Self::behavior_priority(most_severe_behavior) {
                most_severe_behavior = behavior;
            }

            // Collect warnings
            if behavior.is_warning() {
                if let Some(def) = self.get_definition(&label.src, &label.val) {
                    if let Some(name) = def.get_name("en") {
                        warnings.push(name.to_string());
                    }
                }
            }

            result.labels.push(label.clone());
        }

        result.behavior = most_severe_behavior;
        if !warnings.is_empty() {
            result.warning = Some(warnings.join(", "));
        }

        result
    }

    /// Get behavior for a specific label
    fn get_label_behavior(&self, labeler_did: &str, label: &str) -> LabelBehavior {
        // Check user preferences first
        if let Some(prefs) = self.subscriptions.get_preferences(labeler_did) {
            if let Some(behavior) = prefs.label_behaviors.get(label) {
                return *behavior;
            }
        }

        // Fall back to definition default
        if let Some(def) = self.get_definition(labeler_did, label) {
            return def.default_behavior;
        }

        LabelBehavior::Warn
    }

    /// Get label definition
    fn get_definition(&self, labeler_did: &str, label: &str) -> Option<&LabelDefinition> {
        self.definitions
            .get(labeler_did)
            .and_then(|defs| defs.get(label))
    }

    /// Get priority of behavior (higher = more restrictive)
    fn behavior_priority(behavior: LabelBehavior) -> u8 {
        match behavior {
            LabelBehavior::Ignore => 0,
            LabelBehavior::BlurMedia => 1,
            LabelBehavior::Warn => 2,
            LabelBehavior::Hide => 3,
            LabelBehavior::Block => 4,
        }
    }
}

/// Service for managing labeler interactions
pub struct LabelerService {
    /// XRPC client
    client: Arc<RwLock<XrpcClient>>,
}

impl LabelerService {
    /// Create a new labeler service
    pub fn new(client: XrpcClient) -> Self {
        Self {
            client: Arc::new(RwLock::new(client)),
        }
    }

    /// Get labeler profile
    pub async fn get_labeler(&self, did: &str) -> Result<LabelerProfile> {
        let request = XrpcRequest::query("app.bsky.labeler.getServices")
            .param("dids", did);

        let client = self.client.read().await;

        #[derive(Deserialize)]
        struct Response {
            views: Vec<LabelerProfile>,
        }

        let response = client
            .query::<Response>(request)
            .await
            .map_err(|e| LabelError::Xrpc(e.to_string()))?;

        response
            .data
            .views
            .into_iter()
            .next()
            .ok_or_else(|| LabelError::LabelerNotFound(did.to_string()))
    }

    /// Get multiple labeler profiles
    pub async fn get_labelers(&self, dids: &[&str]) -> Result<Vec<LabelerProfile>> {
        if dids.is_empty() {
            return Ok(Vec::new());
        }

        let dids_str = dids.join(",");
        let request = XrpcRequest::query("app.bsky.labeler.getServices")
            .param("dids", dids_str);

        let client = self.client.read().await;

        #[derive(Deserialize)]
        struct Response {
            views: Vec<LabelerProfile>,
        }

        let response = client
            .query::<Response>(request)
            .await
            .map_err(|e| LabelError::Xrpc(e.to_string()))?;

        Ok(response.data.views)
    }

    /// Get labels for a subject (account or content)
    pub async fn get_labels(&self, uri: &str) -> Result<Vec<Label>> {
        let request = XrpcRequest::query("com.atproto.label.queryLabels")
            .param("uriPatterns", uri);

        let client = self.client.read().await;

        #[derive(Deserialize)]
        struct Response {
            labels: Vec<Label>,
        }

        let response = client
            .query::<Response>(request)
            .await
            .map_err(|e| LabelError::Xrpc(e.to_string()))?;

        Ok(response.data.labels)
    }

    /// Subscribe to labeler notifications (register for label updates)
    pub async fn subscribe_to_labeler(&self, repo: &str, labeler_did: &str) -> Result<()> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct SubscribeRequest {
            labeler_did: String,
        }

        let request_body = SubscribeRequest {
            labeler_did: labeler_did.to_string(),
        };

        let request = XrpcRequest::procedure("app.bsky.labeler.subscribe")
            .param("repo", repo)
            .json_body(&request_body)
            .map_err(LabelError::Serialization)?;

        let client = self.client.read().await;
        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| LabelError::Xrpc(e.to_string()))?;

        Ok(())
    }

    /// Unsubscribe from labeler notifications
    pub async fn unsubscribe_from_labeler(&self, repo: &str, labeler_did: &str) -> Result<()> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct UnsubscribeRequest {
            labeler_did: String,
        }

        let request_body = UnsubscribeRequest {
            labeler_did: labeler_did.to_string(),
        };

        let request = XrpcRequest::procedure("app.bsky.labeler.unsubscribe")
            .param("repo", repo)
            .json_body(&request_body)
            .map_err(LabelError::Serialization)?;

        let client = self.client.read().await;
        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| LabelError::Xrpc(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Label tests

    #[test]
    fn test_label_value() {
        let label = Label {
            src: "did:plc:labeler".to_string(),
            uri: "at://did:plc:user/app.bsky.feed.post/123".to_string(),
            cid: None,
            val: "nsfw".to_string(),
            neg: false,
            cts: "2024-01-01T00:00:00Z".to_string(),
            exp: None,
            sig: None,
        };

        assert_eq!(label.value(), "nsfw");
        assert!(!label.is_negation());
        assert!(!label.is_expired());
    }

    #[test]
    fn test_label_negation() {
        let label = Label {
            src: "did:plc:labeler".to_string(),
            uri: "at://did:plc:user/app.bsky.feed.post/123".to_string(),
            cid: None,
            val: "nsfw".to_string(),
            neg: true,
            cts: "2024-01-01T00:00:00Z".to_string(),
            exp: None,
            sig: None,
        };

        assert!(label.is_negation());
    }

    #[test]
    fn test_label_expired() {
        let label = Label {
            src: "did:plc:labeler".to_string(),
            uri: "at://did:plc:user/app.bsky.feed.post/123".to_string(),
            cid: None,
            val: "temporary".to_string(),
            neg: false,
            cts: "2020-01-01T00:00:00Z".to_string(),
            exp: Some("2020-01-02T00:00:00Z".to_string()), // Past date
            sig: None,
        };

        assert!(label.is_expired());
    }

    // Label behavior tests

    #[test]
    fn test_label_behavior_default() {
        assert_eq!(LabelBehavior::default(), LabelBehavior::Ignore);
    }

    #[test]
    fn test_label_behavior_is_blocking() {
        assert!(LabelBehavior::Block.is_blocking());
        assert!(!LabelBehavior::Hide.is_blocking());
        assert!(!LabelBehavior::Warn.is_blocking());
    }

    #[test]
    fn test_label_behavior_is_hiding() {
        assert!(LabelBehavior::Block.is_hiding());
        assert!(LabelBehavior::Hide.is_hiding());
        assert!(!LabelBehavior::Warn.is_hiding());
    }

    #[test]
    fn test_label_behavior_is_warning() {
        assert!(LabelBehavior::Warn.is_warning());
        assert!(LabelBehavior::Hide.is_warning());
        assert!(!LabelBehavior::Block.is_warning());
    }

    // Label definition tests

    #[test]
    fn test_label_definition_new() {
        let def = LabelDefinition::new("nsfw");
        assert_eq!(def.identifier, "nsfw");
        assert_eq!(def.severity, 1);
        assert!(def.applies_to_content);
        assert!(!def.applies_to_account);
        assert!(def.user_configurable);
    }

    #[test]
    fn test_label_definition_localization() {
        let def = LabelDefinition {
            identifier: "nsfw".to_string(),
            locales: Some(vec![
                LabelLocale {
                    lang: "en".to_string(),
                    name: "Adult Content".to_string(),
                    description: Some("Contains adult material".to_string()),
                },
                LabelLocale {
                    lang: "es".to_string(),
                    name: "Contenido Adulto".to_string(),
                    description: Some("Contiene material para adultos".to_string()),
                },
            ]),
            severity: 3,
            applies_to_account: false,
            applies_to_content: true,
            default_behavior: LabelBehavior::Hide,
            user_configurable: true,
        };

        assert_eq!(def.get_name("en"), Some("Adult Content"));
        assert_eq!(def.get_name("es"), Some("Contenido Adulto"));
        assert_eq!(def.get_name("fr"), Some("Adult Content")); // Fallback to first
        assert_eq!(def.get_description("en"), Some("Contains adult material"));
    }

    // Labeler profile tests

    #[test]
    fn test_labeler_profile_new() {
        let profile = LabelerProfile::new("did:plc:labeler", "labeler.bsky.social");
        assert_eq!(profile.did, "did:plc:labeler");
        assert_eq!(profile.handle, "labeler.bsky.social");
        assert!(!profile.is_subscribed);
    }

    #[test]
    fn test_labeler_profile_name() {
        let mut profile = LabelerProfile::new("did:plc:labeler", "labeler.bsky.social");
        assert_eq!(profile.name(), "labeler.bsky.social");

        profile.display_name = Some("Safety Labeler".to_string());
        assert_eq!(profile.name(), "Safety Labeler");
    }

    // Labeler preferences tests

    #[test]
    fn test_labeler_preferences_default() {
        let prefs = LabelerPreferences::new();
        assert!(prefs.label_behaviors.is_empty());
        assert!(!prefs.show_adult_content);
    }

    #[test]
    fn test_labeler_preferences_behavior() {
        let mut prefs = LabelerPreferences::new();

        assert_eq!(prefs.get_behavior("nsfw", LabelBehavior::Warn), LabelBehavior::Warn);

        prefs.set_behavior("nsfw", LabelBehavior::Hide);
        assert_eq!(prefs.get_behavior("nsfw", LabelBehavior::Warn), LabelBehavior::Hide);

        prefs.reset_behavior("nsfw");
        assert_eq!(prefs.get_behavior("nsfw", LabelBehavior::Warn), LabelBehavior::Warn);
    }

    // Subscription tests

    #[test]
    fn test_subscriptions_new() {
        let subs = LabelerSubscriptions::new();
        assert!(subs.is_empty());
        assert_eq!(subs.count(), 0);
    }

    #[test]
    fn test_subscriptions_subscribe() {
        let mut subs = LabelerSubscriptions::new();

        subs.subscribe("did:plc:labeler1");
        assert!(subs.is_subscribed("did:plc:labeler1"));
        assert!(!subs.is_subscribed("did:plc:labeler2"));
        assert_eq!(subs.count(), 1);
    }

    #[test]
    fn test_subscriptions_unsubscribe() {
        let mut subs = LabelerSubscriptions::new();

        subs.subscribe("did:plc:labeler1");
        subs.subscribe("did:plc:labeler2");
        assert_eq!(subs.count(), 2);

        subs.unsubscribe("did:plc:labeler1");
        assert!(!subs.is_subscribed("did:plc:labeler1"));
        assert!(subs.is_subscribed("did:plc:labeler2"));
        assert_eq!(subs.count(), 1);
    }

    #[test]
    fn test_subscriptions_preferences() {
        let mut subs = LabelerSubscriptions::new();
        subs.subscribe("did:plc:labeler1");

        let prefs = subs.get_preferences("did:plc:labeler1");
        assert!(prefs.is_some());

        if let Some(prefs) = subs.get_preferences_mut("did:plc:labeler1") {
            prefs.set_behavior("nsfw", LabelBehavior::Hide);
        }

        let prefs = subs.get_preferences("did:plc:labeler1").unwrap();
        assert_eq!(prefs.get_behavior("nsfw", LabelBehavior::Warn), LabelBehavior::Hide);
    }

    #[test]
    fn test_subscriptions_list() {
        let mut subs = LabelerSubscriptions::new();
        subs.subscribe("did:plc:labeler1");
        subs.subscribe("did:plc:labeler2");

        let list = subs.subscribed_labelers();
        assert_eq!(list.len(), 2);
        assert!(list.contains(&"did:plc:labeler1"));
        assert!(list.contains(&"did:plc:labeler2"));
    }

    // Label result tests

    #[test]
    fn test_label_result_none() {
        let result = LabelResult::none();
        assert_eq!(result.behavior, LabelBehavior::Ignore);
        assert!(result.labels.is_empty());
        assert!(result.warning.is_none());
        assert!(result.should_show());
        assert!(!result.should_warn());
    }

    #[test]
    fn test_label_result_should_show() {
        let mut result = LabelResult::none();

        result.behavior = LabelBehavior::Warn;
        assert!(result.should_show());

        result.behavior = LabelBehavior::Hide;
        assert!(!result.should_show());

        result.behavior = LabelBehavior::Block;
        assert!(!result.should_show());
    }

    #[test]
    fn test_label_result_should_blur() {
        let mut result = LabelResult::none();

        result.behavior = LabelBehavior::BlurMedia;
        assert!(result.should_blur_media());

        result.behavior = LabelBehavior::Hide;
        assert!(!result.should_blur_media());
    }

    // Label applicator tests

    #[test]
    fn test_label_applicator_no_subscriptions() {
        let subs = LabelerSubscriptions::new();
        let applicator = LabelApplicator::new(subs);

        let labels = vec![Label {
            src: "did:plc:labeler".to_string(),
            uri: "at://did:plc:user/app.bsky.feed.post/123".to_string(),
            cid: None,
            val: "nsfw".to_string(),
            neg: false,
            cts: "2024-01-01T00:00:00Z".to_string(),
            exp: None,
            sig: None,
        }];

        let result = applicator.apply(&labels);
        // Should ignore labels from unsubscribed labelers
        assert!(result.labels.is_empty());
        assert_eq!(result.behavior, LabelBehavior::Ignore);
    }

    #[test]
    fn test_label_applicator_with_subscription() {
        let mut subs = LabelerSubscriptions::new();
        subs.subscribe("did:plc:labeler");

        let mut applicator = LabelApplicator::new(subs);
        applicator.add_definitions("did:plc:labeler", vec![
            LabelDefinition {
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
            },
        ]);

        let labels = vec![Label {
            src: "did:plc:labeler".to_string(),
            uri: "at://did:plc:user/app.bsky.feed.post/123".to_string(),
            cid: None,
            val: "nsfw".to_string(),
            neg: false,
            cts: "2024-01-01T00:00:00Z".to_string(),
            exp: None,
            sig: None,
        }];

        let result = applicator.apply(&labels);
        assert_eq!(result.labels.len(), 1);
        assert_eq!(result.behavior, LabelBehavior::Hide);
    }

    #[test]
    fn test_label_applicator_most_restrictive() {
        let mut subs = LabelerSubscriptions::new();
        subs.subscribe("did:plc:labeler");

        let mut applicator = LabelApplicator::new(subs);
        applicator.add_definitions("did:plc:labeler", vec![
            LabelDefinition {
                identifier: "warn".to_string(),
                locales: None,
                severity: 1,
                applies_to_account: false,
                applies_to_content: true,
                default_behavior: LabelBehavior::Warn,
                user_configurable: true,
            },
            LabelDefinition {
                identifier: "block".to_string(),
                locales: None,
                severity: 5,
                applies_to_account: false,
                applies_to_content: true,
                default_behavior: LabelBehavior::Block,
                user_configurable: true,
            },
        ]);

        let labels = vec![
            Label {
                src: "did:plc:labeler".to_string(),
                uri: "at://did:plc:user/post/123".to_string(),
                cid: None,
                val: "warn".to_string(),
                neg: false,
                cts: "2024-01-01T00:00:00Z".to_string(),
                exp: None,
                sig: None,
            },
            Label {
                src: "did:plc:labeler".to_string(),
                uri: "at://did:plc:user/post/123".to_string(),
                cid: None,
                val: "block".to_string(),
                neg: false,
                cts: "2024-01-01T00:00:00Z".to_string(),
                exp: None,
                sig: None,
            },
        ];

        let result = applicator.apply(&labels);
        // Should use the most restrictive behavior
        assert_eq!(result.behavior, LabelBehavior::Block);
    }

    #[test]
    fn test_label_applicator_skips_negation() {
        let mut subs = LabelerSubscriptions::new();
        subs.subscribe("did:plc:labeler");

        let mut applicator = LabelApplicator::new(subs);
        applicator.add_definitions("did:plc:labeler", vec![
            LabelDefinition::new("nsfw"),
        ]);

        let labels = vec![Label {
            src: "did:plc:labeler".to_string(),
            uri: "at://did:plc:user/post/123".to_string(),
            cid: None,
            val: "nsfw".to_string(),
            neg: true, // Negation label
            cts: "2024-01-01T00:00:00Z".to_string(),
            exp: None,
            sig: None,
        }];

        let result = applicator.apply(&labels);
        assert!(result.labels.is_empty());
    }

    // Serialization tests

    #[test]
    fn test_label_serialization() {
        let label = Label {
            src: "did:plc:labeler".to_string(),
            uri: "at://did:plc:user/post/123".to_string(),
            cid: Some("bafyreig...".to_string()),
            val: "nsfw".to_string(),
            neg: false,
            cts: "2024-01-01T00:00:00Z".to_string(),
            exp: None,
            sig: None,
        };

        let json = serde_json::to_string(&label).unwrap();
        assert!(json.contains("nsfw"));

        let deserialized: Label = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.val, "nsfw");
    }

    #[test]
    fn test_label_behavior_serialization() {
        let behavior = LabelBehavior::Hide;
        let json = serde_json::to_string(&behavior).unwrap();
        assert_eq!(json, "\"hide\"");

        let deserialized: LabelBehavior = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, LabelBehavior::Hide);
    }

    #[test]
    fn test_labeler_profile_serialization() {
        let profile = LabelerProfile::new("did:plc:labeler", "labeler.bsky.social");
        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("labeler.bsky.social"));

        let deserialized: LabelerProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.did, "did:plc:labeler");
    }

    // Error tests

    #[test]
    fn test_label_error_display() {
        let error = LabelError::LabelerNotFound("did:plc:unknown".to_string());
        assert!(format!("{}", error).contains("did:plc:unknown"));

        let error = LabelError::AlreadySubscribed("did:plc:labeler".to_string());
        assert!(format!("{}", error).contains("Already subscribed"));
    }

    // Service tests

    #[test]
    fn test_labeler_service_creation() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let _service = LabelerService::new(client);
    }
}

//! User preferences for notifications, messages, and other features
//!
//! This module defines preference structures that can be stored in AppPersistedState
//! for configuring application behavior.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Notification preferences for different notification types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPreferences {
    /// Enable/disable notifications globally
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Play sounds for notifications
    #[serde(default = "default_true")]
    pub sound_enabled: bool,

    /// Show badge count on app icon
    #[serde(default = "default_true")]
    pub badge_enabled: bool,

    /// Per-notification-type settings
    #[serde(default)]
    pub type_settings: HashMap<String, NotificationTypeSettings>,
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        let mut type_settings = HashMap::new();

        // Default all notification types to enabled
        for notif_type in
            &["like", "repost", "follow", "mention", "reply", "quote", "starterpack-joined"]
        {
            type_settings.insert(
                notif_type.to_string(),
                NotificationTypeSettings {
                    enabled: true,
                    push_enabled: true,
                    sound_enabled: true,
                },
            );
        }

        Self {
            enabled: true,
            sound_enabled: true,
            badge_enabled: true,
            type_settings,
        }
    }
}

impl NotificationPreferences {
    /// Check if a notification type is enabled
    pub fn is_type_enabled(&self, notification_type: &str) -> bool {
        if !self.enabled {
            return false;
        }

        self.type_settings
            .get(notification_type)
            .map(|s| s.enabled)
            .unwrap_or(true) // Default to enabled if not configured
    }

    /// Check if push notifications are enabled for a type
    pub fn is_push_enabled_for_type(&self, notification_type: &str) -> bool {
        if !self.enabled {
            return false;
        }

        self.type_settings
            .get(notification_type)
            .map(|s| s.push_enabled)
            .unwrap_or(true)
    }

    /// Check if sounds are enabled for a type
    pub fn is_sound_enabled_for_type(&self, notification_type: &str) -> bool {
        if !self.enabled || !self.sound_enabled {
            return false;
        }

        self.type_settings
            .get(notification_type)
            .map(|s| s.sound_enabled)
            .unwrap_or(true)
    }

    /// Enable all notification types
    pub fn enable_all(&mut self) {
        self.enabled = true;
        for settings in self.type_settings.values_mut() {
            settings.enabled = true;
        }
    }

    /// Disable all notification types
    pub fn disable_all(&mut self) {
        self.enabled = false;
    }

    /// Set preferences for a specific notification type
    pub fn set_type_settings(
        &mut self,
        notification_type: String,
        settings: NotificationTypeSettings,
    ) {
        self.type_settings.insert(notification_type, settings);
    }
}

/// Settings for a specific notification type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationTypeSettings {
    /// Whether this notification type is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Whether push notifications are enabled for this type
    #[serde(default = "default_true")]
    pub push_enabled: bool,

    /// Whether sounds are enabled for this type
    #[serde(default = "default_true")]
    pub sound_enabled: bool,
}

impl Default for NotificationTypeSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            push_enabled: true,
            sound_enabled: true,
        }
    }
}

/// Message privacy settings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessagePrivacy {
    /// Anyone can message you
    Everyone,
    /// Only people you follow can message you
    #[default]
    Following,
    /// Only people in your network (mutual follows) can message you
    Mutuals,
    /// Nobody can message you
    Nobody,
}

impl MessagePrivacy {
    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            MessagePrivacy::Everyone => "Anyone can message you",
            MessagePrivacy::Following => "Only people you follow",
            MessagePrivacy::Mutuals => "Only mutual follows",
            MessagePrivacy::Nobody => "Nobody can message you",
        }
    }

    /// Check if messages are allowed from a user
    pub fn allows_messages_from(&self, is_following: bool, is_followed_by: bool) -> bool {
        match self {
            MessagePrivacy::Everyone => true,
            MessagePrivacy::Following => is_following,
            MessagePrivacy::Mutuals => is_following && is_followed_by,
            MessagePrivacy::Nobody => false,
        }
    }
}

/// Message preferences
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagePreferences {
    /// Who can send you messages
    #[serde(default)]
    pub privacy: MessagePrivacy,

    /// Enable message notifications
    #[serde(default = "default_true")]
    pub notifications_enabled: bool,

    /// Play sounds for new messages
    #[serde(default = "default_true")]
    pub sound_enabled: bool,

    /// Show message previews in notifications
    #[serde(default = "default_true")]
    pub show_previews: bool,

    /// Enable push notifications for messages
    #[serde(default = "default_true")]
    pub push_enabled: bool,

    /// Automatically mark messages as read when viewed
    #[serde(default = "default_true")]
    pub auto_mark_read: bool,

    /// Show typing indicators
    #[serde(default = "default_true")]
    pub typing_indicators: bool,

    /// Show read receipts
    #[serde(default = "default_true")]
    pub read_receipts: bool,
}

impl Default for MessagePreferences {
    fn default() -> Self {
        Self {
            privacy: MessagePrivacy::default(),
            notifications_enabled: true,
            sound_enabled: true,
            show_previews: true,
            push_enabled: true,
            auto_mark_read: true,
            typing_indicators: true,
            read_receipts: true,
        }
    }
}

impl MessagePreferences {
    /// Check if notifications are fully enabled
    pub fn are_notifications_enabled(&self) -> bool {
        self.notifications_enabled
    }

    /// Check if sounds should play for new messages
    pub fn should_play_sound(&self) -> bool {
        self.notifications_enabled && self.sound_enabled
    }

    /// Check if push notifications should be sent
    pub fn should_send_push(&self) -> bool {
        self.notifications_enabled && self.push_enabled
    }

    /// Disable all message notifications
    pub fn disable_all_notifications(&mut self) {
        self.notifications_enabled = false;
        self.push_enabled = false;
        self.sound_enabled = false;
    }

    /// Enable all message notifications
    pub fn enable_all_notifications(&mut self) {
        self.notifications_enabled = true;
        self.push_enabled = true;
        self.sound_enabled = true;
    }
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_preferences_default() {
        let prefs = NotificationPreferences::default();
        assert!(prefs.enabled);
        assert!(prefs.sound_enabled);
        assert!(prefs.badge_enabled);
        assert!(prefs.is_type_enabled("like"));
        assert!(prefs.is_type_enabled("follow"));
    }

    #[test]
    fn test_notification_preferences_is_type_enabled() {
        let mut prefs = NotificationPreferences::default();

        assert!(prefs.is_type_enabled("like"));

        // Disable globally
        prefs.enabled = false;
        assert!(!prefs.is_type_enabled("like"));

        // Re-enable globally, disable specific type
        prefs.enabled = true;
        prefs.type_settings.get_mut("like").unwrap().enabled = false;
        assert!(!prefs.is_type_enabled("like"));
        assert!(prefs.is_type_enabled("follow"));
    }

    #[test]
    fn test_notification_preferences_push_enabled() {
        let prefs = NotificationPreferences::default();
        assert!(prefs.is_push_enabled_for_type("like"));

        let mut prefs = prefs;
        prefs.type_settings.get_mut("like").unwrap().push_enabled = false;
        assert!(!prefs.is_push_enabled_for_type("like"));
        assert!(prefs.is_push_enabled_for_type("follow"));
    }

    #[test]
    fn test_notification_preferences_sound_enabled() {
        let prefs = NotificationPreferences::default();
        assert!(prefs.is_sound_enabled_for_type("like"));

        let mut prefs = prefs;
        prefs.sound_enabled = false;
        assert!(!prefs.is_sound_enabled_for_type("like"));
    }

    #[test]
    fn test_notification_preferences_enable_all() {
        let mut prefs = NotificationPreferences::default();
        prefs.enabled = false;
        prefs.type_settings.get_mut("like").unwrap().enabled = false;

        prefs.enable_all();
        assert!(prefs.enabled);
        assert!(prefs.is_type_enabled("like"));
    }

    #[test]
    fn test_notification_preferences_disable_all() {
        let mut prefs = NotificationPreferences::default();
        prefs.disable_all();
        assert!(!prefs.enabled);
        assert!(!prefs.is_type_enabled("like"));
    }

    #[test]
    fn test_notification_preferences_set_type_settings() {
        let mut prefs = NotificationPreferences::default();

        prefs.set_type_settings(
            "custom".to_string(),
            NotificationTypeSettings {
                enabled: false,
                push_enabled: false,
                sound_enabled: true,
            },
        );

        assert!(!prefs.is_type_enabled("custom"));
        assert!(!prefs.is_push_enabled_for_type("custom"));
    }

    #[test]
    fn test_message_privacy_default() {
        assert_eq!(MessagePrivacy::default(), MessagePrivacy::Following);
    }

    #[test]
    fn test_message_privacy_description() {
        assert_eq!(MessagePrivacy::Everyone.description(), "Anyone can message you");
        assert_eq!(MessagePrivacy::Following.description(), "Only people you follow");
        assert_eq!(MessagePrivacy::Mutuals.description(), "Only mutual follows");
        assert_eq!(MessagePrivacy::Nobody.description(), "Nobody can message you");
    }

    #[test]
    fn test_message_privacy_allows_messages() {
        // Everyone
        assert!(MessagePrivacy::Everyone.allows_messages_from(false, false));
        assert!(MessagePrivacy::Everyone.allows_messages_from(true, false));
        assert!(MessagePrivacy::Everyone.allows_messages_from(false, true));

        // Following
        assert!(!MessagePrivacy::Following.allows_messages_from(false, false));
        assert!(MessagePrivacy::Following.allows_messages_from(true, false));
        assert!(!MessagePrivacy::Following.allows_messages_from(false, true));

        // Mutuals
        assert!(!MessagePrivacy::Mutuals.allows_messages_from(false, false));
        assert!(!MessagePrivacy::Mutuals.allows_messages_from(true, false));
        assert!(!MessagePrivacy::Mutuals.allows_messages_from(false, true));
        assert!(MessagePrivacy::Mutuals.allows_messages_from(true, true));

        // Nobody
        assert!(!MessagePrivacy::Nobody.allows_messages_from(false, false));
        assert!(!MessagePrivacy::Nobody.allows_messages_from(true, true));
    }

    #[test]
    fn test_message_preferences_default() {
        let prefs = MessagePreferences::default();
        assert_eq!(prefs.privacy, MessagePrivacy::Following);
        assert!(prefs.notifications_enabled);
        assert!(prefs.sound_enabled);
        assert!(prefs.push_enabled);
        assert!(prefs.auto_mark_read);
    }

    #[test]
    fn test_message_preferences_are_notifications_enabled() {
        let prefs = MessagePreferences::default();
        assert!(prefs.are_notifications_enabled());

        let mut prefs = prefs;
        prefs.notifications_enabled = false;
        assert!(!prefs.are_notifications_enabled());
    }

    #[test]
    fn test_message_preferences_should_play_sound() {
        let prefs = MessagePreferences::default();
        assert!(prefs.should_play_sound());

        let mut prefs = prefs;
        prefs.sound_enabled = false;
        assert!(!prefs.should_play_sound());

        prefs.sound_enabled = true;
        prefs.notifications_enabled = false;
        assert!(!prefs.should_play_sound());
    }

    #[test]
    fn test_message_preferences_should_send_push() {
        let prefs = MessagePreferences::default();
        assert!(prefs.should_send_push());

        let mut prefs = prefs;
        prefs.push_enabled = false;
        assert!(!prefs.should_send_push());
    }

    #[test]
    fn test_message_preferences_disable_all() {
        let mut prefs = MessagePreferences::default();
        prefs.disable_all_notifications();

        assert!(!prefs.notifications_enabled);
        assert!(!prefs.push_enabled);
        assert!(!prefs.sound_enabled);
    }

    #[test]
    fn test_message_preferences_enable_all() {
        let mut prefs = MessagePreferences::default();
        prefs.notifications_enabled = false;
        prefs.push_enabled = false;
        prefs.sound_enabled = false;

        prefs.enable_all_notifications();

        assert!(prefs.notifications_enabled);
        assert!(prefs.push_enabled);
        assert!(prefs.sound_enabled);
    }

    #[test]
    fn test_notification_type_settings_default() {
        let settings = NotificationTypeSettings::default();
        assert!(settings.enabled);
        assert!(settings.push_enabled);
        assert!(settings.sound_enabled);
    }

    #[test]
    fn test_notification_preferences_serialization() {
        let prefs = NotificationPreferences::default();
        let json = serde_json::to_string(&prefs).unwrap();
        let parsed: NotificationPreferences = serde_json::from_str(&json).unwrap();
        assert_eq!(prefs, parsed);
    }

    #[test]
    fn test_message_preferences_serialization() {
        let prefs = MessagePreferences::default();
        let json = serde_json::to_string(&prefs).unwrap();
        let parsed: MessagePreferences = serde_json::from_str(&json).unwrap();
        assert_eq!(prefs, parsed);
    }
}

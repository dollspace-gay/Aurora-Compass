//! Direct message management
//!
//! This module provides functionality for managing direct message conversations
//! in the AT Protocol / Bluesky ecosystem. Features include:
//! - Conversation list management
//! - Message thread viewing
//! - Sending and receiving messages
//! - Message persistence and caching
//! - Real-time message updates

use atproto_client::xrpc::{XrpcClient, XrpcRequest};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// Errors that can occur during messaging operations
#[derive(Debug, Error)]
pub enum MessageError {
    /// XRPC error
    #[error("XRPC error: {0}")]
    Xrpc(String),

    /// Conversation not found
    #[error("Conversation not found: {0}")]
    ConversationNotFound(String),

    /// Message not found
    #[error("Message not found: {0}")]
    MessageNotFound(String),

    /// Invalid recipient
    #[error("Invalid recipient: {0}")]
    InvalidRecipient(String),

    /// Message too long
    #[error("Message too long: {length} exceeds maximum {max}")]
    MessageTooLong {
        /// Actual message length
        length: usize,
        /// Maximum allowed length
        max: usize,
    },

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// No active session
    #[error("No active session")]
    NoSession,
}

/// Result type for messaging operations
pub type Result<T> = std::result::Result<T, MessageError>;

/// Maximum message length (10,000 characters for chat messages)
pub const MAX_MESSAGE_LENGTH: usize = 10_000;

/// Message sender information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageSender {
    /// DID of the sender
    pub did: String,
    /// Handle of the sender
    pub handle: String,
    /// Display name of the sender
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Avatar URL of the sender
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}

/// A direct message in a conversation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// Message ID
    pub id: String,
    /// Message text content
    pub text: String,
    /// Sender information
    pub sender: MessageSender,
    /// Timestamp when message was sent
    pub sent_at: DateTime<Utc>,
    /// Whether this message was sent by the current user
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_from_self: Option<bool>,
}

impl Message {
    /// Create a new message
    pub fn new(
        id: impl Into<String>,
        text: impl Into<String>,
        sender: MessageSender,
        sent_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            sender,
            sent_at,
            is_from_self: None,
        }
    }

    /// Check if the message text is within size limits
    pub fn validate_length(&self) -> Result<()> {
        if self.text.len() > MAX_MESSAGE_LENGTH {
            return Err(MessageError::MessageTooLong {
                length: self.text.len(),
                max: MAX_MESSAGE_LENGTH,
            });
        }
        Ok(())
    }
}

/// A conversation/chat thread
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    /// Conversation ID
    pub id: String,
    /// Members of the conversation (excluding self)
    pub members: Vec<MessageSender>,
    /// Most recent message in the conversation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message: Option<Message>,
    /// Number of unread messages
    #[serde(default)]
    pub unread_count: u32,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Whether this conversation is muted
    #[serde(default)]
    pub muted: bool,
}

impl Conversation {
    /// Create a new conversation
    pub fn new(id: impl Into<String>, members: Vec<MessageSender>) -> Self {
        Self {
            id: id.into(),
            members,
            last_message: None,
            unread_count: 0,
            updated_at: Utc::now(),
            muted: false,
        }
    }

    /// Get the other participant's display name (for 1-on-1 chats)
    pub fn other_participant_name(&self) -> Option<String> {
        self.members.first().and_then(|m| {
            m.display_name
                .clone()
                .or_else(|| Some(m.handle.clone()))
        })
    }

    /// Check if this conversation has unread messages
    pub fn has_unread(&self) -> bool {
        self.unread_count > 0
    }

    /// Mark conversation as read
    pub fn mark_read(&mut self) {
        self.unread_count = 0;
    }
}

/// Request to send a new message
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    /// Conversation ID
    pub conversation_id: String,
    /// Message text
    pub message: String,
}

/// Response from sending a message
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageResponse {
    /// ID of the sent message
    pub message_id: String,
    /// Timestamp when message was sent
    pub sent_at: DateTime<Utc>,
}

/// Request to create a new conversation
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateConversationRequest {
    /// DIDs of the participants
    pub members: Vec<String>,
}

/// Response from creating a conversation
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateConversationResponse {
    /// ID of the created conversation
    pub conversation_id: String,
    /// The created conversation
    pub conversation: Conversation,
}

/// Parameters for listing conversations
#[derive(Debug, Clone, Default)]
pub struct ListConversationsParams {
    /// Maximum number of conversations to return
    pub limit: Option<u32>,
    /// Cursor for pagination
    pub cursor: Option<String>,
}

/// Response from listing conversations
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListConversationsResponse {
    /// List of conversations
    pub conversations: Vec<Conversation>,
    /// Cursor for next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Parameters for listing messages in a conversation
#[derive(Debug, Clone, Default)]
pub struct ListMessagesParams {
    /// Conversation ID
    pub conversation_id: String,
    /// Maximum number of messages to return
    pub limit: Option<u32>,
    /// Cursor for pagination
    pub cursor: Option<String>,
}

/// Response from listing messages
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMessagesResponse {
    /// List of messages
    pub messages: Vec<Message>,
    /// Cursor for next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Service for managing direct message conversations
///
/// Provides methods for listing conversations, viewing message threads,
/// and sending messages.
///
/// # Example
///
/// ```rust,no_run
/// use app_core::messages::{ConversationService, ListConversationsParams};
/// use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = XrpcClientConfig::new("https://bsky.social");
///     let client = XrpcClient::new(config);
///     let service = ConversationService::new(client);
///
///     // List conversations
///     let params = ListConversationsParams::default();
///     let response = service.list_conversations(params).await?;
///     println!("Found {} conversations", response.conversations.len());
///
///     Ok(())
/// }
/// ```
pub struct ConversationService {
    /// XRPC client
    client: Arc<RwLock<XrpcClient>>,
}

impl ConversationService {
    /// Create a new conversation service
    pub fn new(client: XrpcClient) -> Self {
        Self {
            client: Arc::new(RwLock::new(client)),
        }
    }

    /// List all conversations for the current user
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters for listing conversations
    ///
    /// # Returns
    ///
    /// List of conversations with pagination cursor
    pub async fn list_conversations(
        &self,
        params: ListConversationsParams,
    ) -> Result<ListConversationsResponse> {
        let mut request = XrpcRequest::query("chat.bsky.convo.listConvos");

        if let Some(limit) = params.limit {
            request = request.param("limit", limit.to_string());
        }
        if let Some(cursor) = params.cursor {
            request = request.param("cursor", cursor);
        }

        let client = self.client.read().await;
        let response = client
            .query(request)
            .await
            .map_err(|e| MessageError::Xrpc(e.to_string()))?;

        Ok(response.data)
    }

    /// Get messages in a conversation
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters for listing messages
    ///
    /// # Returns
    ///
    /// List of messages with pagination cursor
    pub async fn list_messages(
        &self,
        params: ListMessagesParams,
    ) -> Result<ListMessagesResponse> {
        let mut request = XrpcRequest::query("chat.bsky.convo.getMessages")
            .param("convoId", params.conversation_id);

        if let Some(limit) = params.limit {
            request = request.param("limit", limit.to_string());
        }
        if let Some(cursor) = params.cursor {
            request = request.param("cursor", cursor);
        }

        let client = self.client.read().await;
        let response = client
            .query(request)
            .await
            .map_err(|e| MessageError::Xrpc(e.to_string()))?;

        Ok(response.data)
    }

    /// Send a message to a conversation
    ///
    /// # Arguments
    ///
    /// * `conversation_id` - ID of the conversation
    /// * `message` - Message text to send
    ///
    /// # Returns
    ///
    /// Response containing message ID and timestamp
    pub async fn send_message(
        &self,
        conversation_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Result<SendMessageResponse> {
        let message_text = message.into();

        // Validate message length
        if message_text.len() > MAX_MESSAGE_LENGTH {
            return Err(MessageError::MessageTooLong {
                length: message_text.len(),
                max: MAX_MESSAGE_LENGTH,
            });
        }

        let request_body = SendMessageRequest {
            conversation_id: conversation_id.into(),
            message: message_text,
        };

        let request = XrpcRequest::procedure("chat.bsky.convo.sendMessage")
            .json_body(&request_body)
            .map_err(MessageError::Serialization)?;

        let client = self.client.read().await;
        let response = client
            .procedure(request)
            .await
            .map_err(|e| MessageError::Xrpc(e.to_string()))?;

        Ok(response.data)
    }

    /// Create a new conversation
    ///
    /// # Arguments
    ///
    /// * `members` - DIDs of the participants
    ///
    /// # Returns
    ///
    /// Response containing the created conversation
    pub async fn create_conversation(
        &self,
        members: Vec<String>,
    ) -> Result<CreateConversationResponse> {
        let request_body = CreateConversationRequest { members };

        let request = XrpcRequest::procedure("chat.bsky.convo.createConvo")
            .json_body(&request_body)
            .map_err(MessageError::Serialization)?;

        let client = self.client.read().await;
        let response = client
            .procedure(request)
            .await
            .map_err(|e| MessageError::Xrpc(e.to_string()))?;

        Ok(response.data)
    }

    /// Mark a conversation as read
    ///
    /// # Arguments
    ///
    /// * `conversation_id` - ID of the conversation to mark as read
    pub async fn mark_conversation_read(
        &self,
        conversation_id: impl Into<String>,
    ) -> Result<()> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct MarkReadRequest {
            conversation_id: String,
        }

        let request_body = MarkReadRequest {
            conversation_id: conversation_id.into(),
        };

        let request = XrpcRequest::procedure("chat.bsky.convo.updateRead")
            .json_body(&request_body)
            .map_err(MessageError::Serialization)?;

        let client = self.client.read().await;
        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| MessageError::Xrpc(e.to_string()))?;

        Ok(())
    }

    /// Mute a conversation
    ///
    /// # Arguments
    ///
    /// * `conversation_id` - ID of the conversation to mute
    pub async fn mute_conversation(&self, conversation_id: impl Into<String>) -> Result<()> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct MuteRequest {
            conversation_id: String,
        }

        let request_body = MuteRequest {
            conversation_id: conversation_id.into(),
        };

        let request = XrpcRequest::procedure("chat.bsky.convo.muteConvo")
            .json_body(&request_body)
            .map_err(MessageError::Serialization)?;

        let client = self.client.read().await;
        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| MessageError::Xrpc(e.to_string()))?;

        Ok(())
    }

    /// Unmute a conversation
    ///
    /// # Arguments
    ///
    /// * `conversation_id` - ID of the conversation to unmute
    pub async fn unmute_conversation(&self, conversation_id: impl Into<String>) -> Result<()> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct UnmuteRequest {
            conversation_id: String,
        }

        let request_body = UnmuteRequest {
            conversation_id: conversation_id.into(),
        };

        let request = XrpcRequest::procedure("chat.bsky.convo.unmuteConvo")
            .json_body(&request_body)
            .map_err(MessageError::Serialization)?;

        let client = self.client.read().await;
        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| MessageError::Xrpc(e.to_string()))?;

        Ok(())
    }
}

/// Sort order for conversations in inbox
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversationSortBy {
    /// Sort by most recent activity (default)
    Recent,
    /// Sort by unread count (unread first)
    Unread,
    /// Sort alphabetically by participant name
    Name,
}

/// Filter options for inbox conversations
#[derive(Debug, Clone, Default)]
pub struct InboxFilter {
    /// Show only conversations with unread messages
    pub only_unread: bool,
    /// Exclude muted conversations
    pub exclude_muted: bool,
    /// Search query to filter by participant name or message content
    pub search_query: Option<String>,
}

/// Statistics about the inbox
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InboxStats {
    /// Total number of conversations
    pub total_conversations: usize,
    /// Number of conversations with unread messages
    pub unread_conversations: usize,
    /// Total unread message count
    pub total_unread: u32,
}

/// Inbox view for managing conversations
///
/// Provides sorting, filtering, and preview generation for conversation lists.
///
/// # Example
///
/// ```rust
/// use app_core::messages::{InboxView, Conversation, ConversationSortBy, InboxFilter};
///
/// let conversations = vec![/* your conversations */];
/// let mut inbox = InboxView::new(conversations);
///
/// // Sort by most recent
/// inbox.sort_by(ConversationSortBy::Recent);
///
/// // Filter to show only unread
/// let filter = InboxFilter {
///     only_unread: true,
///     ..Default::default()
/// };
/// let unread = inbox.filter(&filter);
///
/// // Get stats
/// let stats = inbox.stats();
/// println!("Total unread: {}", stats.total_unread);
/// ```
pub struct InboxView {
    conversations: Vec<Conversation>,
}

impl InboxView {
    /// Create a new inbox view
    pub fn new(conversations: Vec<Conversation>) -> Self {
        Self { conversations }
    }

    /// Get all conversations
    pub fn conversations(&self) -> &[Conversation] {
        &self.conversations
    }

    /// Get a mutable reference to conversations
    pub fn conversations_mut(&mut self) -> &mut Vec<Conversation> {
        &mut self.conversations
    }

    /// Sort conversations by the specified criterion
    pub fn sort_by(&mut self, sort_by: ConversationSortBy) {
        match sort_by {
            ConversationSortBy::Recent => {
                self.conversations
                    .sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            }
            ConversationSortBy::Unread => {
                self.conversations.sort_by(|a, b| {
                    // First sort by unread status (unread first)
                    match (a.has_unread(), b.has_unread()) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => {
                            // Then by unread count (descending)
                            match b.unread_count.cmp(&a.unread_count) {
                                std::cmp::Ordering::Equal => {
                                    // Finally by most recent
                                    b.updated_at.cmp(&a.updated_at)
                                }
                                other => other,
                            }
                        }
                    }
                });
            }
            ConversationSortBy::Name => {
                self.conversations.sort_by(|a, b| {
                    let name_a = a.other_participant_name().unwrap_or_default().to_lowercase();
                    let name_b = b.other_participant_name().unwrap_or_default().to_lowercase();
                    name_a.cmp(&name_b)
                });
            }
        }
    }

    /// Filter conversations based on criteria
    pub fn filter(&self, filter: &InboxFilter) -> Vec<&Conversation> {
        self.conversations
            .iter()
            .filter(|conv| {
                // Filter by unread status
                if filter.only_unread && !conv.has_unread() {
                    return false;
                }

                // Filter by muted status
                if filter.exclude_muted && conv.muted {
                    return false;
                }

                // Filter by search query
                if let Some(query) = &filter.search_query {
                    let query_lower = query.to_lowercase();

                    // Search in participant names
                    let name_match = conv.members.iter().any(|member| {
                        member
                            .display_name
                            .as_ref()
                            .map(|n| n.to_lowercase().contains(&query_lower))
                            .unwrap_or(false)
                            || member.handle.to_lowercase().contains(&query_lower)
                    });

                    // Search in last message text
                    let message_match = conv
                        .last_message
                        .as_ref()
                        .map(|msg| msg.text.to_lowercase().contains(&query_lower))
                        .unwrap_or(false);

                    if !name_match && !message_match {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Get inbox statistics
    pub fn stats(&self) -> InboxStats {
        let total_conversations = self.conversations.len();
        let unread_conversations = self
            .conversations
            .iter()
            .filter(|c| c.has_unread())
            .count();
        let total_unread = self.conversations.iter().map(|c| c.unread_count).sum();

        InboxStats {
            total_conversations,
            unread_conversations,
            total_unread,
        }
    }

    /// Get a preview text for a conversation
    pub fn conversation_preview(&self, conversation: &Conversation) -> ConversationPreview {
        ConversationPreview::from_conversation(conversation)
    }

    /// Get only unread conversations
    pub fn unread_conversations(&self) -> Vec<&Conversation> {
        self.conversations.iter().filter(|c| c.has_unread()).collect()
    }

    /// Get only read conversations
    pub fn read_conversations(&self) -> Vec<&Conversation> {
        self.conversations.iter().filter(|c| !c.has_unread()).collect()
    }
}

/// Preview information for a conversation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationPreview {
    /// Conversation ID
    pub conversation_id: String,
    /// Display title (participant names)
    pub title: String,
    /// Preview text (last message or empty state)
    pub preview_text: String,
    /// Relative timestamp ("2 hours ago", etc.)
    pub relative_time: String,
    /// Whether there are unread messages
    pub has_unread: bool,
    /// Unread count
    pub unread_count: u32,
    /// Whether the conversation is muted
    pub is_muted: bool,
}

impl ConversationPreview {
    /// Generate a preview from a conversation
    pub fn from_conversation(conversation: &Conversation) -> Self {
        let title = if conversation.members.len() == 1 {
            conversation
                .other_participant_name()
                .unwrap_or_else(|| "Unknown".to_string())
        } else {
            // Group chat - show comma-separated names
            conversation
                .members
                .iter()
                .filter_map(|m| {
                    m.display_name
                        .as_ref()
                        .cloned()
                        .or_else(|| Some(m.handle.clone()))
                })
                .take(3)
                .collect::<Vec<_>>()
                .join(", ")
        };

        let preview_text = if let Some(last_msg) = &conversation.last_message {
            // Truncate long messages
            const MAX_PREVIEW_LENGTH: usize = 100;
            if last_msg.text.len() > MAX_PREVIEW_LENGTH {
                format!("{}...", &last_msg.text[..MAX_PREVIEW_LENGTH])
            } else {
                last_msg.text.clone()
            }
        } else {
            "No messages yet".to_string()
        };

        let relative_time = format_relative_time(&conversation.updated_at);

        Self {
            conversation_id: conversation.id.clone(),
            title,
            preview_text,
            relative_time,
            has_unread: conversation.has_unread(),
            unread_count: conversation.unread_count,
            is_muted: conversation.muted,
        }
    }
}

/// Format a timestamp as relative time
///
/// Returns strings like "Just now", "5 minutes ago", "2 hours ago", "Yesterday", "3 days ago", etc.
pub fn format_relative_time(timestamp: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(*timestamp);

    if duration.num_seconds() < 60 {
        "Just now".to_string()
    } else if duration.num_minutes() < 60 {
        let mins = duration.num_minutes();
        if mins == 1 {
            "1 minute ago".to_string()
        } else {
            format!("{} minutes ago", mins)
        }
    } else if duration.num_hours() < 24 {
        let hours = duration.num_hours();
        if hours == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{} hours ago", hours)
        }
    } else if duration.num_days() == 1 {
        "Yesterday".to_string()
    } else if duration.num_days() < 7 {
        format!("{} days ago", duration.num_days())
    } else if duration.num_weeks() == 1 {
        "1 week ago".to_string()
    } else if duration.num_weeks() < 4 {
        format!("{} weeks ago", duration.num_weeks())
    } else {
        // Show actual date for older messages
        timestamp.format("%b %d, %Y").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_sender_creation() {
        let sender = MessageSender {
            did: "did:plc:test123".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: Some("Alice".to_string()),
            avatar: Some("https://example.com/avatar.jpg".to_string()),
        };

        assert_eq!(sender.did, "did:plc:test123");
        assert_eq!(sender.handle, "alice.bsky.social");
        assert_eq!(sender.display_name, Some("Alice".to_string()));
    }

    #[test]
    fn test_message_creation() {
        let sender = MessageSender {
            did: "did:plc:test123".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: None,
            avatar: None,
        };

        let now = Utc::now();
        let message = Message::new("msg123", "Hello world", sender, now);

        assert_eq!(message.id, "msg123");
        assert_eq!(message.text, "Hello world");
        assert_eq!(message.sent_at, now);
    }

    #[test]
    fn test_message_validation() {
        let sender = MessageSender {
            did: "did:plc:test123".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: None,
            avatar: None,
        };

        let short_message = Message::new("msg1", "Hello", sender.clone(), Utc::now());
        assert!(short_message.validate_length().is_ok());

        let long_text = "a".repeat(MAX_MESSAGE_LENGTH + 1);
        let long_message = Message::new("msg2", long_text, sender, Utc::now());
        assert!(long_message.validate_length().is_err());
    }

    #[test]
    fn test_conversation_creation() {
        let member = MessageSender {
            did: "did:plc:test123".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: Some("Alice".to_string()),
            avatar: None,
        };

        let convo = Conversation::new("convo123", vec![member]);

        assert_eq!(convo.id, "convo123");
        assert_eq!(convo.members.len(), 1);
        assert_eq!(convo.unread_count, 0);
        assert!(!convo.has_unread());
    }

    #[test]
    fn test_conversation_unread() {
        let member = MessageSender {
            did: "did:plc:test123".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: None,
            avatar: None,
        };

        let mut convo = Conversation::new("convo123", vec![member]);
        convo.unread_count = 5;

        assert!(convo.has_unread());

        convo.mark_read();
        assert_eq!(convo.unread_count, 0);
        assert!(!convo.has_unread());
    }

    #[test]
    fn test_conversation_other_participant_name() {
        let member = MessageSender {
            did: "did:plc:test123".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: Some("Alice".to_string()),
            avatar: None,
        };

        let convo = Conversation::new("convo123", vec![member]);
        assert_eq!(
            convo.other_participant_name(),
            Some("Alice".to_string())
        );

        let member_no_display = MessageSender {
            did: "did:plc:test456".to_string(),
            handle: "bob.bsky.social".to_string(),
            display_name: None,
            avatar: None,
        };

        let convo2 = Conversation::new("convo456", vec![member_no_display]);
        assert_eq!(
            convo2.other_participant_name(),
            Some("bob.bsky.social".to_string())
        );
    }

    #[test]
    fn test_send_message_request_serialization() {
        let request = SendMessageRequest {
            conversation_id: "convo123".to_string(),
            message: "Hello world".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("convo123"));
        assert!(json.contains("Hello world"));
    }

    #[test]
    fn test_list_conversations_params() {
        let params = ListConversationsParams {
            limit: Some(50),
            cursor: Some("cursor123".to_string()),
        };

        assert_eq!(params.limit, Some(50));
        assert_eq!(params.cursor, Some("cursor123".to_string()));
    }

    #[test]
    fn test_list_messages_params() {
        let params = ListMessagesParams {
            conversation_id: "convo123".to_string(),
            limit: Some(100),
            cursor: None,
        };

        assert_eq!(params.conversation_id, "convo123");
        assert_eq!(params.limit, Some(100));
        assert!(params.cursor.is_none());
    }

    #[test]
    fn test_message_error_display() {
        let error = MessageError::MessageTooLong {
            length: 15000,
            max: 10000,
        };
        let msg = format!("{}", error);
        assert!(msg.contains("15000"));
        assert!(msg.contains("10000"));
    }

    #[test]
    fn test_conversation_service_creation() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let _service = ConversationService::new(client);
    }

    // Inbox tests

    fn create_test_conversation(
        id: &str,
        name: &str,
        last_message: Option<&str>,
        unread_count: u32,
        hours_ago: i64,
    ) -> Conversation {
        let member = MessageSender {
            did: format!("did:plc:{}", id),
            handle: format!("{}.bsky.social", name),
            display_name: Some(name.to_string()),
            avatar: None,
        };

        let updated_at = Utc::now() - chrono::Duration::hours(hours_ago);

        let last_msg = last_message.map(|text| Message {
            id: "msg1".to_string(),
            text: text.to_string(),
            sender: member.clone(),
            sent_at: updated_at,
            is_from_self: None,
        });

        Conversation {
            id: id.to_string(),
            members: vec![member],
            last_message: last_msg,
            unread_count,
            updated_at,
            muted: false,
        }
    }

    #[test]
    fn test_inbox_view_creation() {
        let conversations = vec![
            create_test_conversation("1", "alice", Some("Hello!"), 2, 1),
            create_test_conversation("2", "bob", Some("Hey there"), 0, 5),
        ];

        let inbox = InboxView::new(conversations);
        assert_eq!(inbox.conversations().len(), 2);
    }

    #[test]
    fn test_inbox_sort_by_recent() {
        let mut inbox = InboxView::new(vec![
            create_test_conversation("1", "alice", Some("Hello!"), 0, 5),
            create_test_conversation("2", "bob", Some("Hey"), 0, 1),
            create_test_conversation("3", "charlie", Some("Hi"), 0, 10),
        ]);

        inbox.sort_by(ConversationSortBy::Recent);

        let ids: Vec<&str> = inbox
            .conversations()
            .iter()
            .map(|c| c.id.as_str())
            .collect();

        assert_eq!(ids, vec!["2", "1", "3"]); // bob (1h), alice (5h), charlie (10h)
    }

    #[test]
    fn test_inbox_sort_by_unread() {
        let mut inbox = InboxView::new(vec![
            create_test_conversation("1", "alice", Some("Hello"), 0, 1),
            create_test_conversation("2", "bob", Some("Hey"), 5, 2),
            create_test_conversation("3", "charlie", Some("Hi"), 3, 3),
        ]);

        inbox.sort_by(ConversationSortBy::Unread);

        let ids: Vec<&str> = inbox
            .conversations()
            .iter()
            .map(|c| c.id.as_str())
            .collect();

        // bob (5 unread) > charlie (3 unread) > alice (0 unread)
        assert_eq!(ids, vec!["2", "3", "1"]);
    }

    #[test]
    fn test_inbox_sort_by_name() {
        let mut inbox = InboxView::new(vec![
            create_test_conversation("1", "Charlie", Some("Hello"), 0, 1),
            create_test_conversation("2", "alice", Some("Hey"), 0, 2),
            create_test_conversation("3", "Bob", Some("Hi"), 0, 3),
        ]);

        inbox.sort_by(ConversationSortBy::Name);

        let names: Vec<String> = inbox
            .conversations()
            .iter()
            .filter_map(|c| c.other_participant_name())
            .collect();

        assert_eq!(names, vec!["alice", "Bob", "Charlie"]);
    }

    #[test]
    fn test_inbox_filter_only_unread() {
        let inbox = InboxView::new(vec![
            create_test_conversation("1", "alice", Some("Hello"), 2, 1),
            create_test_conversation("2", "bob", Some("Hey"), 0, 2),
            create_test_conversation("3", "charlie", Some("Hi"), 1, 3),
        ]);

        let filter = InboxFilter {
            only_unread: true,
            ..Default::default()
        };

        let unread = inbox.filter(&filter);
        assert_eq!(unread.len(), 2); // alice and charlie have unread
        assert!(unread.iter().all(|c| c.has_unread()));
    }

    #[test]
    fn test_inbox_filter_exclude_muted() {
        let mut conv1 = create_test_conversation("1", "alice", Some("Hello"), 2, 1);
        conv1.muted = true;

        let conv2 = create_test_conversation("2", "bob", Some("Hey"), 0, 2);

        let inbox = InboxView::new(vec![conv1, conv2]);

        let filter = InboxFilter {
            exclude_muted: true,
            ..Default::default()
        };

        let unmuted = inbox.filter(&filter);
        assert_eq!(unmuted.len(), 1);
        assert_eq!(unmuted[0].id, "2");
    }

    #[test]
    fn test_inbox_filter_search_by_name() {
        let inbox = InboxView::new(vec![
            create_test_conversation("1", "alice", Some("Hello"), 0, 1),
            create_test_conversation("2", "bob", Some("Hey"), 0, 2),
            create_test_conversation("3", "alicia", Some("Hi"), 0, 3),
        ]);

        let filter = InboxFilter {
            search_query: Some("ali".to_string()),
            ..Default::default()
        };

        let results = inbox.filter(&filter);
        assert_eq!(results.len(), 2); // alice and alicia
    }

    #[test]
    fn test_inbox_filter_search_by_message() {
        let inbox = InboxView::new(vec![
            create_test_conversation("1", "alice", Some("Hello world"), 0, 1),
            create_test_conversation("2", "bob", Some("Hey there"), 0, 2),
            create_test_conversation("3", "charlie", Some("Hello everyone"), 0, 3),
        ]);

        let filter = InboxFilter {
            search_query: Some("hello".to_string()),
            ..Default::default()
        };

        let results = inbox.filter(&filter);
        assert_eq!(results.len(), 2); // alice and charlie
    }

    #[test]
    fn test_inbox_stats() {
        let inbox = InboxView::new(vec![
            create_test_conversation("1", "alice", Some("Hello"), 2, 1),
            create_test_conversation("2", "bob", Some("Hey"), 0, 2),
            create_test_conversation("3", "charlie", Some("Hi"), 3, 3),
        ]);

        let stats = inbox.stats();
        assert_eq!(stats.total_conversations, 3);
        assert_eq!(stats.unread_conversations, 2); // alice and charlie
        assert_eq!(stats.total_unread, 5); // 2 + 3
    }

    #[test]
    fn test_inbox_unread_conversations() {
        let inbox = InboxView::new(vec![
            create_test_conversation("1", "alice", Some("Hello"), 2, 1),
            create_test_conversation("2", "bob", Some("Hey"), 0, 2),
            create_test_conversation("3", "charlie", Some("Hi"), 1, 3),
        ]);

        let unread = inbox.unread_conversations();
        assert_eq!(unread.len(), 2);
        assert!(unread.iter().all(|c| c.has_unread()));
    }

    #[test]
    fn test_inbox_read_conversations() {
        let inbox = InboxView::new(vec![
            create_test_conversation("1", "alice", Some("Hello"), 2, 1),
            create_test_conversation("2", "bob", Some("Hey"), 0, 2),
            create_test_conversation("3", "charlie", Some("Hi"), 0, 3),
        ]);

        let read = inbox.read_conversations();
        assert_eq!(read.len(), 2); // bob and charlie
        assert!(read.iter().all(|c| !c.has_unread()));
    }

    #[test]
    fn test_conversation_preview() {
        let conv = create_test_conversation("1", "alice", Some("Hello world!"), 3, 2);

        let preview = ConversationPreview::from_conversation(&conv);

        assert_eq!(preview.conversation_id, "1");
        assert_eq!(preview.title, "alice");
        assert_eq!(preview.preview_text, "Hello world!");
        assert!(preview.has_unread);
        assert_eq!(preview.unread_count, 3);
        assert!(!preview.is_muted);
    }

    #[test]
    fn test_conversation_preview_long_message() {
        let long_message = "a".repeat(150);
        let conv = create_test_conversation("1", "alice", Some(&long_message), 0, 1);

        let preview = ConversationPreview::from_conversation(&conv);

        assert_eq!(preview.preview_text.len(), 103); // 100 + "..."
        assert!(preview.preview_text.ends_with("..."));
    }

    #[test]
    fn test_conversation_preview_no_message() {
        let conv = create_test_conversation("1", "alice", None, 0, 1);

        let preview = ConversationPreview::from_conversation(&conv);

        assert_eq!(preview.preview_text, "No messages yet");
    }

    #[test]
    fn test_conversation_preview_group_chat() {
        let member1 = MessageSender {
            did: "did:plc:1".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: Some("Alice".to_string()),
            avatar: None,
        };

        let member2 = MessageSender {
            did: "did:plc:2".to_string(),
            handle: "bob.bsky.social".to_string(),
            display_name: Some("Bob".to_string()),
            avatar: None,
        };

        let member3 = MessageSender {
            did: "did:plc:3".to_string(),
            handle: "charlie.bsky.social".to_string(),
            display_name: Some("Charlie".to_string()),
            avatar: None,
        };

        let conv = Conversation {
            id: "group1".to_string(),
            members: vec![member1, member2, member3],
            last_message: Some(Message {
                id: "msg1".to_string(),
                text: "Group message".to_string(),
                sender: MessageSender {
                    did: "did:plc:1".to_string(),
                    handle: "alice.bsky.social".to_string(),
                    display_name: Some("Alice".to_string()),
                    avatar: None,
                },
                sent_at: Utc::now(),
                is_from_self: None,
            }),
            unread_count: 0,
            updated_at: Utc::now(),
            muted: false,
        };

        let preview = ConversationPreview::from_conversation(&conv);

        assert_eq!(preview.title, "Alice, Bob, Charlie");
    }

    #[test]
    fn test_format_relative_time_just_now() {
        let now = Utc::now();
        let formatted = format_relative_time(&now);
        assert_eq!(formatted, "Just now");
    }

    #[test]
    fn test_format_relative_time_minutes() {
        let timestamp = Utc::now() - chrono::Duration::minutes(5);
        let formatted = format_relative_time(&timestamp);
        assert_eq!(formatted, "5 minutes ago");

        let timestamp = Utc::now() - chrono::Duration::minutes(1);
        let formatted = format_relative_time(&timestamp);
        assert_eq!(formatted, "1 minute ago");
    }

    #[test]
    fn test_format_relative_time_hours() {
        let timestamp = Utc::now() - chrono::Duration::hours(3);
        let formatted = format_relative_time(&timestamp);
        assert_eq!(formatted, "3 hours ago");

        let timestamp = Utc::now() - chrono::Duration::hours(1);
        let formatted = format_relative_time(&timestamp);
        assert_eq!(formatted, "1 hour ago");
    }

    #[test]
    fn test_format_relative_time_yesterday() {
        let timestamp = Utc::now() - chrono::Duration::days(1);
        let formatted = format_relative_time(&timestamp);
        assert_eq!(formatted, "Yesterday");
    }

    #[test]
    fn test_format_relative_time_days() {
        let timestamp = Utc::now() - chrono::Duration::days(3);
        let formatted = format_relative_time(&timestamp);
        assert_eq!(formatted, "3 days ago");
    }

    #[test]
    fn test_format_relative_time_weeks() {
        let timestamp = Utc::now() - chrono::Duration::weeks(2);
        let formatted = format_relative_time(&timestamp);
        assert_eq!(formatted, "2 weeks ago");

        let timestamp = Utc::now() - chrono::Duration::weeks(1);
        let formatted = format_relative_time(&timestamp);
        assert_eq!(formatted, "1 week ago");
    }

    #[test]
    fn test_format_relative_time_old() {
        let timestamp = Utc::now() - chrono::Duration::weeks(5);
        let formatted = format_relative_time(&timestamp);
        // Should show actual date for messages older than 4 weeks
        assert!(formatted.contains(","));
    }

    #[test]
    fn test_inbox_filter_default() {
        let filter = InboxFilter::default();
        assert!(!filter.only_unread);
        assert!(!filter.exclude_muted);
        assert!(filter.search_query.is_none());
    }

    #[test]
    fn test_inbox_stats_default() {
        let stats = InboxStats::default();
        assert_eq!(stats.total_conversations, 0);
        assert_eq!(stats.unread_conversations, 0);
        assert_eq!(stats.total_unread, 0);
    }

    #[test]
    fn test_conversation_sort_by_variants() {
        assert_eq!(ConversationSortBy::Recent, ConversationSortBy::Recent);
        assert_ne!(ConversationSortBy::Recent, ConversationSortBy::Unread);
        assert_ne!(ConversationSortBy::Unread, ConversationSortBy::Name);
    }
}

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
        self.members
            .first()
            .and_then(|m| m.display_name.clone().or_else(|| Some(m.handle.clone())))
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
        Self { client: Arc::new(RwLock::new(client)) }
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
    pub async fn list_messages(&self, params: ListMessagesParams) -> Result<ListMessagesResponse> {
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
    pub async fn mark_conversation_read(&self, conversation_id: impl Into<String>) -> Result<()> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct MarkReadRequest {
            conversation_id: String,
        }

        let request_body = MarkReadRequest { conversation_id: conversation_id.into() };

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

        let request_body = MuteRequest { conversation_id: conversation_id.into() };

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

        let request_body = UnmuteRequest { conversation_id: conversation_id.into() };

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
                    let name_a = a
                        .other_participant_name()
                        .unwrap_or_default()
                        .to_lowercase();
                    let name_b = b
                        .other_participant_name()
                        .unwrap_or_default()
                        .to_lowercase();
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
        let unread_conversations = self.conversations.iter().filter(|c| c.has_unread()).count();
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
        self.conversations
            .iter()
            .filter(|c| c.has_unread())
            .collect()
    }

    /// Get only read conversations
    pub fn read_conversations(&self) -> Vec<&Conversation> {
        self.conversations
            .iter()
            .filter(|c| !c.has_unread())
            .collect()
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

// ============================================================================
// Real-time Message Events
// ============================================================================

/// Type of message event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MessageEventType {
    /// A new message was received
    NewMessage,
    /// A message was deleted
    MessageDeleted,
    /// Conversation metadata was updated
    ConversationUpdated,
    /// A new conversation was created
    ConversationCreated,
    /// Conversation was marked as read
    ConversationRead,
    /// Someone is typing
    TypingIndicator,
}

/// Message event payload
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageEvent {
    /// Type of event
    pub event_type: MessageEventType,
    /// Conversation ID
    pub conversation_id: String,
    /// Optional message (for NewMessage events)
    pub message: Option<Message>,
    /// Optional typing indicator data
    pub typing: Option<TypingIndicator>,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
}

impl MessageEvent {
    /// Create a new message event
    pub fn new(event_type: MessageEventType, conversation_id: impl Into<String>) -> Self {
        Self {
            event_type,
            conversation_id: conversation_id.into(),
            message: None,
            typing: None,
            timestamp: Utc::now(),
        }
    }

    /// Create a new message event with message data
    pub fn with_message(mut self, message: Message) -> Self {
        self.message = Some(message);
        self
    }

    /// Create a new message event with typing data
    pub fn with_typing(mut self, typing: TypingIndicator) -> Self {
        self.typing = Some(typing);
        self
    }
}

/// Typing indicator data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypingIndicator {
    /// User who is typing
    pub user_did: String,
    /// Whether user is currently typing
    pub is_typing: bool,
}

/// Connection mode for event listening
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionMode {
    /// Use polling-based connection
    Polling,
    /// Use WebSocket connection (not yet implemented)
    WebSocket,
}

/// Configuration for event listener
#[derive(Debug, Clone)]
pub struct EventListenerConfig {
    /// Connection mode
    pub mode: ConnectionMode,
    /// Polling interval (for polling mode)
    pub poll_interval: std::time::Duration,
    /// Whether to enable typing indicators
    pub enable_typing: bool,
    /// Event buffer size
    pub buffer_size: usize,
}

impl Default for EventListenerConfig {
    fn default() -> Self {
        Self {
            mode: ConnectionMode::Polling,
            poll_interval: std::time::Duration::from_secs(5),
            enable_typing: false,
            buffer_size: 100,
        }
    }
}

/// Message event listener for real-time updates
///
/// Manages connections to receive real-time message events and
/// distributes them to subscribers via a broadcast channel.
pub struct MessageEventListener {
    client: Arc<RwLock<XrpcClient>>,
    config: EventListenerConfig,
    event_tx: tokio::sync::broadcast::Sender<MessageEvent>,
    running: Arc<RwLock<bool>>,
    last_cursor: Arc<RwLock<Option<String>>>,
}

impl MessageEventListener {
    /// Create a new message event listener
    ///
    /// # Arguments
    ///
    /// * `client` - XRPC client for making requests
    /// * `config` - Event listener configuration
    ///
    /// # Returns
    ///
    /// A new `MessageEventListener` instance
    pub fn new(client: Arc<RwLock<XrpcClient>>, config: EventListenerConfig) -> Self {
        let (event_tx, _) = tokio::sync::broadcast::channel(config.buffer_size);

        Self {
            client,
            config,
            event_tx,
            running: Arc::new(RwLock::new(false)),
            last_cursor: Arc::new(RwLock::new(None)),
        }
    }

    /// Subscribe to message events
    ///
    /// # Returns
    ///
    /// A broadcast receiver for message events
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<MessageEvent> {
        self.event_tx.subscribe()
    }

    /// Check if the listener is currently running
    ///
    /// # Returns
    ///
    /// True if listener is running, false otherwise
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Start listening for events
    ///
    /// Spawns a background task that continuously polls for new events
    /// and broadcasts them to subscribers.
    ///
    /// # Returns
    ///
    /// Result indicating success or error
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Ok(()); // Already running
        }
        *running = true;
        drop(running);

        match self.config.mode {
            ConnectionMode::Polling => self.start_polling().await,
            ConnectionMode::WebSocket => {
                Err(MessageError::Xrpc("WebSocket mode not yet implemented".to_string()))
            }
        }
    }

    /// Start polling-based event listening
    async fn start_polling(&self) -> Result<()> {
        let client = Arc::clone(&self.client);
        let running = Arc::clone(&self.running);
        let last_cursor = Arc::clone(&self.last_cursor);
        let event_tx = self.event_tx.clone();
        let poll_interval = self.config.poll_interval;

        tokio::spawn(async move {
            while *running.read().await {
                // Poll for new messages
                if let Ok(events) = Self::poll_events(&client, &last_cursor).await {
                    for event in events {
                        let _ = event_tx.send(event);
                    }
                }

                tokio::time::sleep(poll_interval).await;
            }
        });

        Ok(())
    }

    /// Poll for new events
    async fn poll_events(
        client: &Arc<RwLock<XrpcClient>>,
        last_cursor: &Arc<RwLock<Option<String>>>,
    ) -> Result<Vec<MessageEvent>> {
        let cursor = last_cursor.read().await.clone();

        let client_guard = client.read().await;
        let mut request = XrpcRequest::query("chat.bsky.convo.listConvos");

        if let Some(cursor_val) = cursor {
            request = request.param("cursor", cursor_val);
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ConvoListResponse {
            convos: Vec<Conversation>,
            cursor: Option<String>,
        }

        let response = client_guard
            .query::<ConvoListResponse>(request)
            .await
            .map_err(|e| MessageError::Xrpc(e.to_string()))?;

        // Update cursor for next poll
        if let Some(new_cursor) = response.data.cursor {
            *last_cursor.write().await = Some(new_cursor);
        }

        // Convert conversations to events
        let mut events = Vec::new();
        for convo in response.data.convos {
            // Check if this is a new conversation or updated one
            if let Some(last_message) = convo.last_message {
                let event = MessageEvent::new(MessageEventType::NewMessage, convo.id.clone())
                    .with_message(Message {
                        id: last_message.id,
                        text: last_message.text,
                        sender: MessageSender {
                            did: last_message.sender.did,
                            handle: last_message.sender.handle,
                            display_name: last_message.sender.display_name,
                            avatar: last_message.sender.avatar,
                        },
                        sent_at: last_message.sent_at,
                        is_from_self: None,
                    });
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Stop listening for events
    ///
    /// # Returns
    ///
    /// Result indicating success or error
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        Ok(())
    }

    /// Reset the polling cursor
    ///
    /// This will cause the next poll to fetch from the beginning
    pub async fn reset_cursor(&self) {
        *self.last_cursor.write().await = None;
    }
}

impl Clone for MessageEventListener {
    fn clone(&self) -> Self {
        Self {
            client: Arc::clone(&self.client),
            config: self.config.clone(),
            event_tx: self.event_tx.clone(),
            running: Arc::clone(&self.running),
            last_cursor: Arc::clone(&self.last_cursor),
        }
    }
}

// ============================================================================
// Message Requests
// ============================================================================

/// Status of a message request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRequestStatus {
    /// Request is pending review
    Pending,
    /// Request has been accepted
    Accepted,
    /// Request has been declined
    Declined,
}

impl Default for MessageRequestStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// A message request from a non-followed account
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageRequest {
    /// Request ID
    pub id: String,
    /// Sender information
    pub sender: MessageSender,
    /// Preview of the message text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_preview: Option<String>,
    /// Timestamp when request was received
    pub received_at: DateTime<Utc>,
    /// Request status
    #[serde(default)]
    pub status: MessageRequestStatus,
    /// Conversation ID (if accepted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
}

impl MessageRequest {
    /// Create a new message request
    pub fn new(id: impl Into<String>, sender: MessageSender, received_at: DateTime<Utc>) -> Self {
        Self {
            id: id.into(),
            sender,
            message_preview: None,
            received_at,
            status: MessageRequestStatus::Pending,
            conversation_id: None,
        }
    }

    /// Check if the request is pending
    pub fn is_pending(&self) -> bool {
        self.status == MessageRequestStatus::Pending
    }

    /// Check if the request has been accepted
    pub fn is_accepted(&self) -> bool {
        self.status == MessageRequestStatus::Accepted
    }

    /// Check if the request has been declined
    pub fn is_declined(&self) -> bool {
        self.status == MessageRequestStatus::Declined
    }
}

/// Filter for message requests
#[derive(Debug, Clone, Default)]
pub struct MessageRequestFilter {
    /// Filter by status
    pub status: Option<MessageRequestStatus>,
    /// Search query for sender name or handle
    pub search_query: Option<String>,
    /// Limit number of results
    pub limit: Option<u32>,
}

/// Parameters for listing message requests
#[derive(Debug, Clone, Default)]
pub struct ListMessageRequestsParams {
    /// Maximum number of requests to return
    pub limit: Option<u32>,
    /// Cursor for pagination
    pub cursor: Option<String>,
}

/// Response from listing message requests
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMessageRequestsResponse {
    /// List of message requests
    pub requests: Vec<MessageRequest>,
    /// Cursor for next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Request to accept a message request
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptMessageRequestParams {
    /// Request ID to accept
    pub request_id: String,
}

/// Response from accepting a message request
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptMessageRequestResponse {
    /// ID of the created conversation
    pub conversation_id: String,
}

/// Request to decline a message request
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclineMessageRequestParams {
    /// Request ID to decline
    pub request_id: String,
}

/// Message request queue manager
///
/// Manages incoming message requests from non-followed accounts.
/// Provides filtering, sorting, and batch operations for handling requests.
///
/// # Example
///
/// ```rust
/// use app_core::messages::{MessageRequestQueue, MessageRequest, MessageRequestFilter, MessageRequestStatus};
///
/// let requests = vec![/* your message requests */];
/// let queue = MessageRequestQueue::new(requests);
///
/// // Filter to show only pending
/// let filter = MessageRequestFilter {
///     status: Some(MessageRequestStatus::Pending),
///     ..Default::default()
/// };
/// let pending = queue.filter(&filter);
///
/// println!("Pending requests: {}", pending.len());
/// ```
pub struct MessageRequestQueue {
    requests: Vec<MessageRequest>,
}

impl MessageRequestQueue {
    /// Create a new message request queue
    pub fn new(requests: Vec<MessageRequest>) -> Self {
        Self { requests }
    }

    /// Get all requests
    pub fn requests(&self) -> &[MessageRequest] {
        &self.requests
    }

    /// Get a mutable reference to requests
    pub fn requests_mut(&mut self) -> &mut Vec<MessageRequest> {
        &mut self.requests
    }

    /// Filter requests based on criteria
    pub fn filter(&self, filter: &MessageRequestFilter) -> Vec<&MessageRequest> {
        let mut filtered: Vec<&MessageRequest> = self
            .requests
            .iter()
            .filter(|req| {
                // Filter by status
                if let Some(status) = filter.status {
                    if req.status != status {
                        return false;
                    }
                }

                // Filter by search query
                if let Some(query) = &filter.search_query {
                    let query_lower = query.to_lowercase();

                    let name_match = req
                        .sender
                        .display_name
                        .as_ref()
                        .map(|n| n.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
                        || req.sender.handle.to_lowercase().contains(&query_lower);

                    let message_match = req
                        .message_preview
                        .as_ref()
                        .map(|m| m.to_lowercase().contains(&query_lower))
                        .unwrap_or(false);

                    if !name_match && !message_match {
                        return false;
                    }
                }

                true
            })
            .collect();

        // Apply limit if specified
        if let Some(limit) = filter.limit {
            filtered.truncate(limit as usize);
        }

        filtered
    }

    /// Get only pending requests
    pub fn pending_requests(&self) -> Vec<&MessageRequest> {
        self.requests.iter().filter(|r| r.is_pending()).collect()
    }

    /// Get only accepted requests
    pub fn accepted_requests(&self) -> Vec<&MessageRequest> {
        self.requests.iter().filter(|r| r.is_accepted()).collect()
    }

    /// Get only declined requests
    pub fn declined_requests(&self) -> Vec<&MessageRequest> {
        self.requests.iter().filter(|r| r.is_declined()).collect()
    }

    /// Count requests by status
    pub fn count_by_status(&self) -> (usize, usize, usize) {
        let pending = self.requests.iter().filter(|r| r.is_pending()).count();
        let accepted = self.requests.iter().filter(|r| r.is_accepted()).count();
        let declined = self.requests.iter().filter(|r| r.is_declined()).count();
        (pending, accepted, declined)
    }

    /// Find a request by ID
    pub fn find_by_id(&self, id: &str) -> Option<&MessageRequest> {
        self.requests.iter().find(|r| r.id == id)
    }

    /// Find a request by sender DID
    pub fn find_by_sender(&self, did: &str) -> Option<&MessageRequest> {
        self.requests.iter().find(|r| r.sender.did == did)
    }

    /// Update request status
    pub fn update_status(&mut self, request_id: &str, status: MessageRequestStatus) -> bool {
        if let Some(request) = self.requests.iter_mut().find(|r| r.id == request_id) {
            request.status = status;
            true
        } else {
            false
        }
    }

    /// Remove a request from the queue
    pub fn remove(&mut self, request_id: &str) -> Option<MessageRequest> {
        if let Some(index) = self.requests.iter().position(|r| r.id == request_id) {
            Some(self.requests.remove(index))
        } else {
            None
        }
    }

    /// Sort requests by received time (newest first)
    pub fn sort_by_recent(&mut self) {
        self.requests
            .sort_by(|a, b| b.received_at.cmp(&a.received_at));
    }

    /// Sort requests by sender name
    pub fn sort_by_sender(&mut self) {
        self.requests.sort_by(|a, b| {
            let name_a = a
                .sender
                .display_name
                .as_ref()
                .unwrap_or(&a.sender.handle)
                .to_lowercase();
            let name_b = b
                .sender
                .display_name
                .as_ref()
                .unwrap_or(&b.sender.handle)
                .to_lowercase();
            name_a.cmp(&name_b)
        });
    }
}

/// Service for managing message requests
impl ConversationService {
    /// List message requests
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters for listing message requests
    ///
    /// # Returns
    ///
    /// List of message requests with pagination cursor
    pub async fn list_message_requests(
        &self,
        params: ListMessageRequestsParams,
    ) -> Result<ListMessageRequestsResponse> {
        let mut request = XrpcRequest::query("chat.bsky.convo.listMessageRequests");

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

    /// Accept a message request
    ///
    /// # Arguments
    ///
    /// * `request_id` - ID of the request to accept
    ///
    /// # Returns
    ///
    /// Response containing the created conversation ID
    pub async fn accept_message_request(
        &self,
        request_id: impl Into<String>,
    ) -> Result<AcceptMessageRequestResponse> {
        let request_body = AcceptMessageRequestParams { request_id: request_id.into() };

        let request = XrpcRequest::procedure("chat.bsky.convo.acceptMessageRequest")
            .json_body(&request_body)
            .map_err(MessageError::Serialization)?;

        let client = self.client.read().await;
        let response = client
            .procedure(request)
            .await
            .map_err(|e| MessageError::Xrpc(e.to_string()))?;

        Ok(response.data)
    }

    /// Decline a message request
    ///
    /// # Arguments
    ///
    /// * `request_id` - ID of the request to decline
    pub async fn decline_message_request(&self, request_id: impl Into<String>) -> Result<()> {
        let request_body = DeclineMessageRequestParams { request_id: request_id.into() };

        let request = XrpcRequest::procedure("chat.bsky.convo.declineMessageRequest")
            .json_body(&request_body)
            .map_err(MessageError::Serialization)?;

        let client = self.client.read().await;
        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| MessageError::Xrpc(e.to_string()))?;

        Ok(())
    }

    /// Accept multiple message requests
    ///
    /// # Arguments
    ///
    /// * `request_ids` - IDs of requests to accept
    ///
    /// # Returns
    ///
    /// Vector of conversation IDs for accepted requests
    pub async fn accept_message_requests_batch(
        &self,
        request_ids: Vec<String>,
    ) -> Result<Vec<AcceptMessageRequestResponse>> {
        let mut responses = Vec::new();

        for request_id in request_ids {
            match self.accept_message_request(request_id).await {
                Ok(response) => responses.push(response),
                Err(e) => {
                    // Continue with other requests even if one fails
                    eprintln!("Failed to accept message request: {}", e);
                }
            }
        }

        Ok(responses)
    }

    /// Decline multiple message requests
    ///
    /// # Arguments
    ///
    /// * `request_ids` - IDs of requests to decline
    ///
    /// # Returns
    ///
    /// Number of successfully declined requests
    pub async fn decline_message_requests_batch(&self, request_ids: Vec<String>) -> Result<usize> {
        let mut declined_count = 0;

        for request_id in request_ids {
            match self.decline_message_request(request_id).await {
                Ok(_) => declined_count += 1,
                Err(e) => {
                    // Continue with other requests even if one fails
                    eprintln!("Failed to decline message request: {}", e);
                }
            }
        }

        Ok(declined_count)
    }
}

/// Extension trait for InboxView to integrate with event listener
pub trait InboxViewExt {
    /// Apply a message event to the inbox
    ///
    /// Updates conversations based on incoming events
    ///
    /// # Arguments
    ///
    /// * `event` - The message event to apply
    ///
    /// # Returns
    ///
    /// Result indicating success or error
    fn apply_event(&mut self, event: &MessageEvent) -> Result<()>;
}

impl InboxViewExt for InboxView {
    fn apply_event(&mut self, event: &MessageEvent) -> Result<()> {
        match event.event_type {
            MessageEventType::NewMessage => {
                // Find the conversation and update it
                if let Some(convo) = self
                    .conversations
                    .iter_mut()
                    .find(|c| c.id == event.conversation_id)
                {
                    if let Some(ref message) = event.message {
                        convo.last_message = Some(message.clone());
                        convo.updated_at = message.sent_at;
                        convo.unread_count += 1;
                    }
                }
            }
            MessageEventType::ConversationRead => {
                // Mark conversation as read
                if let Some(convo) = self
                    .conversations
                    .iter_mut()
                    .find(|c| c.id == event.conversation_id)
                {
                    convo.unread_count = 0;
                }
            }
            MessageEventType::ConversationUpdated => {
                // Update conversation metadata (would need more details in event)
            }
            MessageEventType::ConversationCreated => {
                // Would need full conversation data in event to add new conversation
            }
            MessageEventType::MessageDeleted => {
                // Handle message deletion (would need message ID in event)
            }
            MessageEventType::TypingIndicator => {
                // Handle typing indicator (UI concern, no state change needed)
            }
        }
        Ok(())
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
        assert_eq!(convo.other_participant_name(), Some("Alice".to_string()));

        let member_no_display = MessageSender {
            did: "did:plc:test456".to_string(),
            handle: "bob.bsky.social".to_string(),
            display_name: None,
            avatar: None,
        };

        let convo2 = Conversation::new("convo456", vec![member_no_display]);
        assert_eq!(convo2.other_participant_name(), Some("bob.bsky.social".to_string()));
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
        let error = MessageError::MessageTooLong { length: 15000, max: 10000 };
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

        let filter = InboxFilter { only_unread: true, ..Default::default() };

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

        let filter = InboxFilter { exclude_muted: true, ..Default::default() };

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

    // Event system tests
    #[test]
    fn test_message_event_type_variants() {
        assert_eq!(MessageEventType::NewMessage, MessageEventType::NewMessage);
        assert_ne!(MessageEventType::NewMessage, MessageEventType::MessageDeleted);
        assert_ne!(MessageEventType::ConversationUpdated, MessageEventType::TypingIndicator);
    }

    #[test]
    fn test_message_event_creation() {
        let event = MessageEvent::new(MessageEventType::NewMessage, "convo123");

        assert_eq!(event.event_type, MessageEventType::NewMessage);
        assert_eq!(event.conversation_id, "convo123");
        assert!(event.message.is_none());
        assert!(event.typing.is_none());
    }

    #[test]
    fn test_message_event_with_message() {
        let sender = MessageSender {
            did: "did:plc:test".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: Some("Alice".to_string()),
            avatar: None,
        };

        let message = Message::new("msg1", "Hello", sender, Utc::now());

        let event = MessageEvent::new(MessageEventType::NewMessage, "convo123")
            .with_message(message.clone());

        assert_eq!(event.event_type, MessageEventType::NewMessage);
        assert!(event.message.is_some());
        assert_eq!(event.message.as_ref().unwrap().text, "Hello");
    }

    #[test]
    fn test_message_event_with_typing() {
        let typing = TypingIndicator {
            user_did: "did:plc:test".to_string(),
            is_typing: true,
        };

        let event = MessageEvent::new(MessageEventType::TypingIndicator, "convo123")
            .with_typing(typing.clone());

        assert_eq!(event.event_type, MessageEventType::TypingIndicator);
        assert!(event.typing.is_some());
        assert!(event.typing.as_ref().unwrap().is_typing);
    }

    #[test]
    fn test_typing_indicator_creation() {
        let typing = TypingIndicator {
            user_did: "did:plc:test123".to_string(),
            is_typing: true,
        };

        assert_eq!(typing.user_did, "did:plc:test123");
        assert!(typing.is_typing);

        let not_typing = TypingIndicator {
            user_did: "did:plc:test456".to_string(),
            is_typing: false,
        };

        assert!(!not_typing.is_typing);
    }

    #[test]
    fn test_connection_mode_variants() {
        assert_eq!(ConnectionMode::Polling, ConnectionMode::Polling);
        assert_ne!(ConnectionMode::Polling, ConnectionMode::WebSocket);
    }

    #[test]
    fn test_event_listener_config_default() {
        let config = EventListenerConfig::default();

        assert_eq!(config.mode, ConnectionMode::Polling);
        assert_eq!(config.poll_interval, std::time::Duration::from_secs(5));
        assert!(!config.enable_typing);
        assert_eq!(config.buffer_size, 100);
    }

    #[test]
    fn test_event_listener_config_custom() {
        let config = EventListenerConfig {
            mode: ConnectionMode::Polling,
            poll_interval: std::time::Duration::from_secs(10),
            enable_typing: true,
            buffer_size: 200,
        };

        assert_eq!(config.mode, ConnectionMode::Polling);
        assert_eq!(config.poll_interval, std::time::Duration::from_secs(10));
        assert!(config.enable_typing);
        assert_eq!(config.buffer_size, 200);
    }

    #[tokio::test]
    async fn test_message_event_listener_creation() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config_xrpc = XrpcClientConfig::new("https://bsky.social");
        let client = Arc::new(RwLock::new(XrpcClient::new(config_xrpc)));
        let config = EventListenerConfig::default();

        let listener = MessageEventListener::new(client, config);

        assert!(!listener.is_running().await);
    }

    #[tokio::test]
    async fn test_message_event_listener_subscribe() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config_xrpc = XrpcClientConfig::new("https://bsky.social");
        let client = Arc::new(RwLock::new(XrpcClient::new(config_xrpc)));
        let config = EventListenerConfig::default();

        let listener = MessageEventListener::new(client, config);

        let _rx = listener.subscribe();
        // Just verify we can subscribe
    }

    #[tokio::test]
    async fn test_message_event_listener_clone() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config_xrpc = XrpcClientConfig::new("https://bsky.social");
        let client = Arc::new(RwLock::new(XrpcClient::new(config_xrpc)));
        let config = EventListenerConfig::default();

        let listener = MessageEventListener::new(client, config);
        let listener2 = listener.clone();

        assert_eq!(listener.is_running().await, listener2.is_running().await);
    }

    #[tokio::test]
    async fn test_message_event_listener_reset_cursor() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config_xrpc = XrpcClientConfig::new("https://bsky.social");
        let client = Arc::new(RwLock::new(XrpcClient::new(config_xrpc)));
        let config = EventListenerConfig::default();

        let listener = MessageEventListener::new(client, config);

        listener.reset_cursor().await;
        // Just verify the method runs without error
    }

    #[tokio::test]
    async fn test_inbox_view_apply_new_message_event() {
        let sender = MessageSender {
            did: "did:plc:alice".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: Some("Alice".to_string()),
            avatar: None,
        };

        let mut inbox = InboxView::new(vec![Conversation::new("convo1", vec![sender.clone()])]);

        let message = Message::new("msg1", "Hello", sender, Utc::now());
        let event =
            MessageEvent::new(MessageEventType::NewMessage, "convo1").with_message(message.clone());

        inbox.apply_event(&event).unwrap();

        // Verify the conversation was updated
        assert_eq!(inbox.conversations[0].unread_count, 1);
        assert!(inbox.conversations[0].last_message.is_some());
        assert_eq!(inbox.conversations[0].last_message.as_ref().unwrap().text, "Hello");
    }

    #[tokio::test]
    async fn test_inbox_view_apply_read_event() {
        let sender = MessageSender {
            did: "did:plc:alice".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: Some("Alice".to_string()),
            avatar: None,
        };

        let mut conversation = Conversation::new("convo1", vec![sender]);
        conversation.unread_count = 5;

        let mut inbox = InboxView::new(vec![conversation]);

        let event = MessageEvent::new(MessageEventType::ConversationRead, "convo1");
        inbox.apply_event(&event).unwrap();

        // Verify the conversation was marked as read
        assert_eq!(inbox.conversations[0].unread_count, 0);
    }

    #[tokio::test]
    async fn test_inbox_view_apply_event_nonexistent_conversation() {
        let sender = MessageSender {
            did: "did:plc:alice".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: Some("Alice".to_string()),
            avatar: None,
        };

        let mut inbox = InboxView::new(vec![Conversation::new("convo1", vec![sender.clone()])]);

        let message = Message::new("msg1", "Hello", sender, Utc::now());
        let event =
            MessageEvent::new(MessageEventType::NewMessage, "convo999").with_message(message);

        // Should not error, just no-op for nonexistent conversation
        inbox.apply_event(&event).unwrap();

        // Original conversation should be unchanged
        assert_eq!(inbox.conversations[0].unread_count, 0);
        assert!(inbox.conversations[0].last_message.is_none());
    }

    #[tokio::test]
    async fn test_inbox_view_apply_typing_event() {
        let sender = MessageSender {
            did: "did:plc:alice".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: Some("Alice".to_string()),
            avatar: None,
        };

        let mut inbox = InboxView::new(vec![Conversation::new("convo1", vec![sender])]);

        let typing = TypingIndicator {
            user_did: "did:plc:alice".to_string(),
            is_typing: true,
        };

        let event =
            MessageEvent::new(MessageEventType::TypingIndicator, "convo1").with_typing(typing);

        // Should not error or modify state (typing is UI concern)
        inbox.apply_event(&event).unwrap();
    }

    #[test]
    fn test_message_event_serde() {
        let sender = MessageSender {
            did: "did:plc:test".to_string(),
            handle: "test.bsky.social".to_string(),
            display_name: None,
            avatar: None,
        };

        let message = Message::new("msg1", "Test", sender, Utc::now());
        let event = MessageEvent::new(MessageEventType::NewMessage, "convo1").with_message(message);

        // Test serialization
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("newMessage"));
        assert!(json.contains("convo1"));

        // Test deserialization
        let deserialized: MessageEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.event_type, MessageEventType::NewMessage);
        assert_eq!(deserialized.conversation_id, "convo1");
    }

    #[test]
    fn test_typing_indicator_serde() {
        let typing = TypingIndicator {
            user_did: "did:plc:test".to_string(),
            is_typing: true,
        };

        let json = serde_json::to_string(&typing).unwrap();
        assert!(json.contains("did:plc:test"));
        assert!(json.contains("true"));

        let deserialized: TypingIndicator = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.user_did, "did:plc:test");
        assert!(deserialized.is_typing);
    }

    // Message Request Tests

    fn create_test_message_request(
        id: &str,
        sender_name: &str,
        message_preview: Option<&str>,
        hours_ago: i64,
        status: MessageRequestStatus,
    ) -> MessageRequest {
        let sender = MessageSender {
            did: format!("did:plc:{}", id),
            handle: format!("{}.bsky.social", sender_name.to_lowercase()),
            display_name: Some(sender_name.to_string()),
            avatar: None,
        };

        let received_at = Utc::now() - chrono::Duration::hours(hours_ago);

        MessageRequest {
            id: id.to_string(),
            sender,
            message_preview: message_preview.map(|s| s.to_string()),
            received_at,
            status,
            conversation_id: None,
        }
    }

    #[test]
    fn test_message_request_creation() {
        let sender = MessageSender {
            did: "did:plc:test123".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: Some("Alice".to_string()),
            avatar: None,
        };

        let now = Utc::now();
        let request = MessageRequest::new("req1", sender.clone(), now);

        assert_eq!(request.id, "req1");
        assert_eq!(request.sender.did, "did:plc:test123");
        assert_eq!(request.status, MessageRequestStatus::Pending);
        assert!(request.is_pending());
        assert!(!request.is_accepted());
        assert!(!request.is_declined());
    }

    #[test]
    fn test_message_request_status() {
        let sender = MessageSender {
            did: "did:plc:test".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: None,
            avatar: None,
        };

        let mut request = MessageRequest::new("req1", sender, Utc::now());
        assert!(request.is_pending());

        request.status = MessageRequestStatus::Accepted;
        assert!(request.is_accepted());
        assert!(!request.is_pending());

        request.status = MessageRequestStatus::Declined;
        assert!(request.is_declined());
        assert!(!request.is_accepted());
    }

    #[test]
    fn test_message_request_status_default() {
        assert_eq!(MessageRequestStatus::default(), MessageRequestStatus::Pending);
    }

    #[test]
    fn test_message_request_queue_creation() {
        let requests = vec![
            create_test_message_request(
                "1",
                "Alice",
                Some("Hello!"),
                1,
                MessageRequestStatus::Pending,
            ),
            create_test_message_request(
                "2",
                "Bob",
                Some("Hi there"),
                2,
                MessageRequestStatus::Pending,
            ),
        ];

        let queue = MessageRequestQueue::new(requests);
        assert_eq!(queue.requests().len(), 2);
    }

    #[test]
    fn test_message_request_queue_pending_requests() {
        let requests = vec![
            create_test_message_request(
                "1",
                "Alice",
                Some("Hello"),
                1,
                MessageRequestStatus::Pending,
            ),
            create_test_message_request("2", "Bob", Some("Hi"), 2, MessageRequestStatus::Accepted),
            create_test_message_request(
                "3",
                "Charlie",
                Some("Hey"),
                3,
                MessageRequestStatus::Pending,
            ),
        ];

        let queue = MessageRequestQueue::new(requests);
        let pending = queue.pending_requests();

        assert_eq!(pending.len(), 2);
        assert!(pending.iter().all(|r| r.is_pending()));
    }

    #[test]
    fn test_message_request_queue_accepted_requests() {
        let requests = vec![
            create_test_message_request(
                "1",
                "Alice",
                Some("Hello"),
                1,
                MessageRequestStatus::Pending,
            ),
            create_test_message_request("2", "Bob", Some("Hi"), 2, MessageRequestStatus::Accepted),
            create_test_message_request(
                "3",
                "Charlie",
                Some("Hey"),
                3,
                MessageRequestStatus::Accepted,
            ),
        ];

        let queue = MessageRequestQueue::new(requests);
        let accepted = queue.accepted_requests();

        assert_eq!(accepted.len(), 2);
        assert!(accepted.iter().all(|r| r.is_accepted()));
    }

    #[test]
    fn test_message_request_queue_declined_requests() {
        let requests = vec![
            create_test_message_request(
                "1",
                "Alice",
                Some("Hello"),
                1,
                MessageRequestStatus::Declined,
            ),
            create_test_message_request("2", "Bob", Some("Hi"), 2, MessageRequestStatus::Pending),
        ];

        let queue = MessageRequestQueue::new(requests);
        let declined = queue.declined_requests();

        assert_eq!(declined.len(), 1);
        assert_eq!(declined[0].id, "1");
    }

    #[test]
    fn test_message_request_queue_count_by_status() {
        let requests = vec![
            create_test_message_request(
                "1",
                "Alice",
                Some("Hello"),
                1,
                MessageRequestStatus::Pending,
            ),
            create_test_message_request("2", "Bob", Some("Hi"), 2, MessageRequestStatus::Pending),
            create_test_message_request(
                "3",
                "Charlie",
                Some("Hey"),
                3,
                MessageRequestStatus::Accepted,
            ),
            create_test_message_request("4", "Dave", Some("Yo"), 4, MessageRequestStatus::Declined),
        ];

        let queue = MessageRequestQueue::new(requests);
        let (pending, accepted, declined) = queue.count_by_status();

        assert_eq!(pending, 2);
        assert_eq!(accepted, 1);
        assert_eq!(declined, 1);
    }

    #[test]
    fn test_message_request_queue_find_by_id() {
        let requests = vec![
            create_test_message_request(
                "1",
                "Alice",
                Some("Hello"),
                1,
                MessageRequestStatus::Pending,
            ),
            create_test_message_request("2", "Bob", Some("Hi"), 2, MessageRequestStatus::Pending),
        ];

        let queue = MessageRequestQueue::new(requests);

        let found = queue.find_by_id("1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "1");

        let not_found = queue.find_by_id("999");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_message_request_queue_find_by_sender() {
        let requests = vec![
            create_test_message_request(
                "1",
                "Alice",
                Some("Hello"),
                1,
                MessageRequestStatus::Pending,
            ),
            create_test_message_request("2", "Bob", Some("Hi"), 2, MessageRequestStatus::Pending),
        ];

        let queue = MessageRequestQueue::new(requests);

        let found = queue.find_by_sender("did:plc:1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().sender.display_name, Some("Alice".to_string()));

        let not_found = queue.find_by_sender("did:plc:999");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_message_request_queue_update_status() {
        let requests = vec![create_test_message_request(
            "1",
            "Alice",
            Some("Hello"),
            1,
            MessageRequestStatus::Pending,
        )];

        let mut queue = MessageRequestQueue::new(requests);

        assert!(queue.update_status("1", MessageRequestStatus::Accepted));
        assert_eq!(queue.find_by_id("1").unwrap().status, MessageRequestStatus::Accepted);

        assert!(!queue.update_status("999", MessageRequestStatus::Declined));
    }

    #[test]
    fn test_message_request_queue_remove() {
        let requests = vec![
            create_test_message_request(
                "1",
                "Alice",
                Some("Hello"),
                1,
                MessageRequestStatus::Pending,
            ),
            create_test_message_request("2", "Bob", Some("Hi"), 2, MessageRequestStatus::Pending),
        ];

        let mut queue = MessageRequestQueue::new(requests);

        let removed = queue.remove("1");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, "1");
        assert_eq!(queue.requests().len(), 1);

        let not_found = queue.remove("999");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_message_request_queue_sort_by_recent() {
        let mut queue = MessageRequestQueue::new(vec![
            create_test_message_request(
                "1",
                "Alice",
                Some("Hello"),
                5,
                MessageRequestStatus::Pending,
            ),
            create_test_message_request("2", "Bob", Some("Hi"), 1, MessageRequestStatus::Pending),
            create_test_message_request(
                "3",
                "Charlie",
                Some("Hey"),
                10,
                MessageRequestStatus::Pending,
            ),
        ]);

        queue.sort_by_recent();

        let ids: Vec<&str> = queue.requests().iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["2", "1", "3"]); // Most recent first
    }

    #[test]
    fn test_message_request_queue_sort_by_sender() {
        let mut queue = MessageRequestQueue::new(vec![
            create_test_message_request(
                "1",
                "Charlie",
                Some("Hello"),
                1,
                MessageRequestStatus::Pending,
            ),
            create_test_message_request("2", "Alice", Some("Hi"), 2, MessageRequestStatus::Pending),
            create_test_message_request("3", "Bob", Some("Hey"), 3, MessageRequestStatus::Pending),
        ]);

        queue.sort_by_sender();

        let names: Vec<String> = queue
            .requests()
            .iter()
            .map(|r| r.sender.display_name.as_ref().unwrap().clone())
            .collect();

        assert_eq!(names, vec!["Alice", "Bob", "Charlie"]);
    }

    #[test]
    fn test_message_request_queue_filter_by_status() {
        let requests = vec![
            create_test_message_request(
                "1",
                "Alice",
                Some("Hello"),
                1,
                MessageRequestStatus::Pending,
            ),
            create_test_message_request("2", "Bob", Some("Hi"), 2, MessageRequestStatus::Accepted),
            create_test_message_request(
                "3",
                "Charlie",
                Some("Hey"),
                3,
                MessageRequestStatus::Pending,
            ),
        ];

        let queue = MessageRequestQueue::new(requests);

        let filter = MessageRequestFilter {
            status: Some(MessageRequestStatus::Pending),
            ..Default::default()
        };

        let filtered = queue.filter(&filter);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|r| r.is_pending()));
    }

    #[test]
    fn test_message_request_queue_filter_by_search_name() {
        let requests = vec![
            create_test_message_request(
                "1",
                "Alice",
                Some("Hello"),
                1,
                MessageRequestStatus::Pending,
            ),
            create_test_message_request("2", "Bob", Some("Hi"), 2, MessageRequestStatus::Pending),
            create_test_message_request(
                "3",
                "Alicia",
                Some("Hey"),
                3,
                MessageRequestStatus::Pending,
            ),
        ];

        let queue = MessageRequestQueue::new(requests);

        let filter = MessageRequestFilter {
            search_query: Some("ali".to_string()),
            ..Default::default()
        };

        let filtered = queue.filter(&filter);
        assert_eq!(filtered.len(), 2); // Alice and Alicia
    }

    #[test]
    fn test_message_request_queue_filter_by_search_message() {
        let requests = vec![
            create_test_message_request(
                "1",
                "Alice",
                Some("Hello world"),
                1,
                MessageRequestStatus::Pending,
            ),
            create_test_message_request(
                "2",
                "Bob",
                Some("Hi there"),
                2,
                MessageRequestStatus::Pending,
            ),
            create_test_message_request(
                "3",
                "Charlie",
                Some("Hello everyone"),
                3,
                MessageRequestStatus::Pending,
            ),
        ];

        let queue = MessageRequestQueue::new(requests);

        let filter = MessageRequestFilter {
            search_query: Some("hello".to_string()),
            ..Default::default()
        };

        let filtered = queue.filter(&filter);
        assert_eq!(filtered.len(), 2); // Alice and Charlie
    }

    #[test]
    fn test_message_request_queue_filter_with_limit() {
        let requests = vec![
            create_test_message_request(
                "1",
                "Alice",
                Some("Hello"),
                1,
                MessageRequestStatus::Pending,
            ),
            create_test_message_request("2", "Bob", Some("Hi"), 2, MessageRequestStatus::Pending),
            create_test_message_request(
                "3",
                "Charlie",
                Some("Hey"),
                3,
                MessageRequestStatus::Pending,
            ),
        ];

        let queue = MessageRequestQueue::new(requests);

        let filter = MessageRequestFilter { limit: Some(2), ..Default::default() };

        let filtered = queue.filter(&filter);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_message_request_queue_filter_combined() {
        let requests = vec![
            create_test_message_request(
                "1",
                "Alice",
                Some("Hello"),
                1,
                MessageRequestStatus::Pending,
            ),
            create_test_message_request(
                "2",
                "Bob",
                Some("Hello"),
                2,
                MessageRequestStatus::Accepted,
            ),
            create_test_message_request(
                "3",
                "Charlie",
                Some("Hello"),
                3,
                MessageRequestStatus::Pending,
            ),
        ];

        let queue = MessageRequestQueue::new(requests);

        let filter = MessageRequestFilter {
            status: Some(MessageRequestStatus::Pending),
            search_query: Some("hello".to_string()),
            limit: Some(1),
        };

        let filtered = queue.filter(&filter);
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].is_pending());
    }

    #[test]
    fn test_message_request_filter_default() {
        let filter = MessageRequestFilter::default();
        assert!(filter.status.is_none());
        assert!(filter.search_query.is_none());
        assert!(filter.limit.is_none());
    }

    #[test]
    fn test_message_request_status_variants() {
        assert_eq!(MessageRequestStatus::Pending, MessageRequestStatus::Pending);
        assert_ne!(MessageRequestStatus::Pending, MessageRequestStatus::Accepted);
        assert_ne!(MessageRequestStatus::Accepted, MessageRequestStatus::Declined);
    }

    #[test]
    fn test_message_request_serde() {
        let sender = MessageSender {
            did: "did:plc:test".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: Some("Alice".to_string()),
            avatar: None,
        };

        let request = MessageRequest::new("req1", sender, Utc::now());

        // Test serialization
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("req1"));
        assert!(json.contains("pending"));

        // Test deserialization
        let deserialized: MessageRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "req1");
        assert_eq!(deserialized.status, MessageRequestStatus::Pending);
    }

    #[test]
    fn test_list_message_requests_params_default() {
        let params = ListMessageRequestsParams::default();
        assert!(params.limit.is_none());
        assert!(params.cursor.is_none());
    }

    #[test]
    fn test_accept_message_request_params() {
        let params = AcceptMessageRequestParams { request_id: "req123".to_string() };

        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("req123"));
    }

    #[test]
    fn test_decline_message_request_params() {
        let params = DeclineMessageRequestParams { request_id: "req123".to_string() };

        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("req123"));
    }
}

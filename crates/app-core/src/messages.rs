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
}

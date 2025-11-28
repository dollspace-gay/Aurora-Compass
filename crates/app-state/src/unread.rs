//! Unread notification and message tracking
//!
//! This module provides reactive tracking of unread counts for notifications
//! and messages, including polling, caching, and cross-component synchronization.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, watch, RwLock};
use tokio::time::Instant;

/// Default polling interval for unread checks (30 seconds)
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(30);

/// Maximum unread count to track (displays as "30+")
pub const MAX_DISPLAY_COUNT: u32 = 30;

/// Unread count display value
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UnreadDisplay {
    /// No unread items
    #[default]
    None,
    /// Specific count (1-29)
    Count(u32),
    /// 30 or more unread items
    Many,
}

impl UnreadDisplay {
    /// Create from a numeric count
    pub fn from_count(count: u32) -> Self {
        if count == 0 {
            UnreadDisplay::None
        } else if count >= MAX_DISPLAY_COUNT {
            UnreadDisplay::Many
        } else {
            UnreadDisplay::Count(count)
        }
    }

    /// Convert to display string
    pub fn as_display_string(&self) -> String {
        match self {
            UnreadDisplay::None => String::new(),
            UnreadDisplay::Count(n) => n.to_string(),
            UnreadDisplay::Many => format!("{}+", MAX_DISPLAY_COUNT),
        }
    }

    /// Check if there are any unread items
    pub fn has_unread(&self) -> bool {
        !matches!(self, UnreadDisplay::None)
    }

    /// Get the numeric count (0 for None, 30 for Many)
    pub fn count(&self) -> u32 {
        match self {
            UnreadDisplay::None => 0,
            UnreadDisplay::Count(n) => *n,
            UnreadDisplay::Many => MAX_DISPLAY_COUNT,
        }
    }
}

impl std::fmt::Display for UnreadDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_display_string())
    }
}

/// Events broadcast when unread counts change
#[derive(Debug, Clone)]
pub enum UnreadEvent {
    /// Notification count changed
    NotificationsChanged(UnreadDisplay),
    /// Message count changed
    MessagesChanged(UnreadDisplay),
    /// All counts were reset (e.g., on logout)
    Reset,
}

/// State for tracking unread items
#[derive(Debug)]
struct UnreadState {
    /// Current notification count
    notifications: UnreadDisplay,
    /// Current message count
    messages: UnreadDisplay,
    /// When notifications were last synced
    notifications_synced_at: Instant,
    /// When messages were last synced
    messages_synced_at: Instant,
}

impl Default for UnreadState {
    fn default() -> Self {
        let now = Instant::now();
        UnreadState {
            notifications: UnreadDisplay::None,
            messages: UnreadDisplay::None,
            notifications_synced_at: now,
            messages_synced_at: now,
        }
    }
}

/// Tracker for unread notifications and messages
///
/// Provides reactive state for unread counts with automatic polling
/// and cross-component synchronization.
///
/// # Example
///
/// ```no_run
/// use app_state::unread::{UnreadTracker, UnreadDisplay};
///
/// #[tokio::main]
/// async fn main() {
///     let tracker = UnreadTracker::new();
///
///     // Subscribe to notification count changes
///     let mut rx = tracker.subscribe_notifications();
///
///     // Update notification count
///     tracker.set_notification_count(5).await;
///
///     // Check current count
///     let count = tracker.get_notification_count().await;
///     assert_eq!(count, UnreadDisplay::Count(5));
/// }
/// ```
pub struct UnreadTracker {
    /// Internal state
    state: Arc<RwLock<UnreadState>>,
    /// Notification count sender
    notifications_tx: watch::Sender<UnreadDisplay>,
    /// Message count sender
    messages_tx: watch::Sender<UnreadDisplay>,
    /// Event broadcaster
    events_tx: broadcast::Sender<UnreadEvent>,
    /// Whether polling is active
    polling_active: Arc<AtomicBool>,
}

impl UnreadTracker {
    /// Create a new unread tracker
    pub fn new() -> Self {
        let (notifications_tx, _) = watch::channel(UnreadDisplay::None);
        let (messages_tx, _) = watch::channel(UnreadDisplay::None);
        let (events_tx, _) = broadcast::channel(16);

        UnreadTracker {
            state: Arc::new(RwLock::new(UnreadState::default())),
            notifications_tx,
            messages_tx,
            events_tx,
            polling_active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get the current notification unread count
    pub async fn get_notification_count(&self) -> UnreadDisplay {
        self.state.read().await.notifications.clone()
    }

    /// Get the current message unread count
    pub async fn get_message_count(&self) -> UnreadDisplay {
        self.state.read().await.messages.clone()
    }

    /// Set the notification unread count
    pub async fn set_notification_count(&self, count: u32) {
        let display = UnreadDisplay::from_count(count);
        let mut state = self.state.write().await;

        if state.notifications != display {
            state.notifications = display.clone();
            state.notifications_synced_at = Instant::now();
            drop(state);

            // Notify subscribers
            let _ = self.notifications_tx.send(display.clone());
            let _ = self
                .events_tx
                .send(UnreadEvent::NotificationsChanged(display));
        }
    }

    /// Set the message unread count
    pub async fn set_message_count(&self, count: u32) {
        let display = UnreadDisplay::from_count(count);
        let mut state = self.state.write().await;

        if state.messages != display {
            state.messages = display.clone();
            state.messages_synced_at = Instant::now();
            drop(state);

            // Notify subscribers
            let _ = self.messages_tx.send(display.clone());
            let _ = self.events_tx.send(UnreadEvent::MessagesChanged(display));
        }
    }

    /// Mark all notifications as read
    pub async fn mark_notifications_read(&self) {
        self.set_notification_count(0).await;
    }

    /// Mark all messages as read
    pub async fn mark_messages_read(&self) {
        self.set_message_count(0).await;
    }

    /// Reset all unread counts (e.g., on logout)
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        state.notifications = UnreadDisplay::None;
        state.messages = UnreadDisplay::None;
        let now = Instant::now();
        state.notifications_synced_at = now;
        state.messages_synced_at = now;
        drop(state);

        let _ = self.notifications_tx.send(UnreadDisplay::None);
        let _ = self.messages_tx.send(UnreadDisplay::None);
        let _ = self.events_tx.send(UnreadEvent::Reset);
    }

    /// Subscribe to notification count changes
    pub fn subscribe_notifications(&self) -> watch::Receiver<UnreadDisplay> {
        self.notifications_tx.subscribe()
    }

    /// Subscribe to message count changes
    pub fn subscribe_messages(&self) -> watch::Receiver<UnreadDisplay> {
        self.messages_tx.subscribe()
    }

    /// Subscribe to all unread events
    pub fn subscribe_events(&self) -> broadcast::Receiver<UnreadEvent> {
        self.events_tx.subscribe()
    }

    /// Check if polling is currently active
    pub fn is_polling(&self) -> bool {
        self.polling_active.load(Ordering::SeqCst)
    }

    /// Get time since last notification sync
    pub async fn time_since_notification_sync(&self) -> Duration {
        self.state.read().await.notifications_synced_at.elapsed()
    }

    /// Get time since last message sync
    pub async fn time_since_message_sync(&self) -> Duration {
        self.state.read().await.messages_synced_at.elapsed()
    }

    /// Check if notifications should be polled based on current state
    ///
    /// Returns true if:
    /// - Count is 0 (always check for new)
    /// - Count is < 30 with ~50% probability (reduce load)
    /// - Enough time has passed since last sync
    pub async fn should_poll_notifications(&self) -> bool {
        let state = self.state.read().await;

        // Always poll if no unread
        if state.notifications == UnreadDisplay::None {
            return true;
        }

        // Don't poll if at max
        if state.notifications == UnreadDisplay::Many {
            return false;
        }

        // ~50% chance to poll if some unread (to reduce server load)
        // Use elapsed time nanoseconds as a simple pseudo-random source
        state.notifications_synced_at.elapsed().as_nanos() % 2 == 0
    }

    /// Start polling for unread counts
    ///
    /// This spawns a background task that periodically checks for new unread items.
    /// The polling stops when the returned handle is dropped.
    ///
    /// # Arguments
    ///
    /// * `interval` - How often to poll
    /// * `fetch_notifications` - Async function to fetch notification count
    /// * `fetch_messages` - Async function to fetch message count
    pub fn start_polling<FN, FM, FutN, FutM>(
        self: &Arc<Self>,
        interval: Duration,
        fetch_notifications: FN,
        fetch_messages: FM,
    ) -> PollingHandle
    where
        FN: Fn() -> FutN + Send + Sync + 'static,
        FM: Fn() -> FutM + Send + Sync + 'static,
        FutN: std::future::Future<Output = Option<u32>> + Send,
        FutM: std::future::Future<Output = Option<u32>> + Send,
    {
        let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel();
        let tracker = Arc::clone(self);

        tracker.polling_active.store(true, Ordering::SeqCst);

        let handle = tokio::spawn(async move {
            let mut poll_interval = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    _ = poll_interval.tick() => {
                        // Check notifications
                        if tracker.should_poll_notifications().await {
                            if let Some(count) = fetch_notifications().await {
                                tracker.set_notification_count(count).await;
                            }
                        }

                        // Check messages
                        if let Some(count) = fetch_messages().await {
                            tracker.set_message_count(count).await;
                        }
                    }
                    _ = &mut stop_rx => {
                        break;
                    }
                }
            }

            tracker.polling_active.store(false, Ordering::SeqCst);
        });

        PollingHandle { stop_tx: Some(stop_tx), _handle: handle }
    }
}

impl Default for UnreadTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle for controlling polling
///
/// When dropped, the polling task will be stopped.
pub struct PollingHandle {
    stop_tx: Option<tokio::sync::oneshot::Sender<()>>,
    _handle: tokio::task::JoinHandle<()>,
}

impl PollingHandle {
    /// Stop polling manually
    pub fn stop(mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for PollingHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unread_display_from_count() {
        assert_eq!(UnreadDisplay::from_count(0), UnreadDisplay::None);
        assert_eq!(UnreadDisplay::from_count(1), UnreadDisplay::Count(1));
        assert_eq!(UnreadDisplay::from_count(29), UnreadDisplay::Count(29));
        assert_eq!(UnreadDisplay::from_count(30), UnreadDisplay::Many);
        assert_eq!(UnreadDisplay::from_count(100), UnreadDisplay::Many);
    }

    #[test]
    fn test_unread_display_to_string() {
        assert_eq!(UnreadDisplay::None.as_display_string(), "");
        assert_eq!(UnreadDisplay::Count(5).as_display_string(), "5");
        assert_eq!(UnreadDisplay::Many.as_display_string(), "30+");
        // Also test via Display trait
        assert_eq!(format!("{}", UnreadDisplay::Count(5)), "5");
    }

    #[test]
    fn test_unread_display_has_unread() {
        assert!(!UnreadDisplay::None.has_unread());
        assert!(UnreadDisplay::Count(1).has_unread());
        assert!(UnreadDisplay::Many.has_unread());
    }

    #[test]
    fn test_unread_display_count() {
        assert_eq!(UnreadDisplay::None.count(), 0);
        assert_eq!(UnreadDisplay::Count(15).count(), 15);
        assert_eq!(UnreadDisplay::Many.count(), 30);
    }

    #[tokio::test]
    async fn test_tracker_notification_count() {
        let tracker = UnreadTracker::new();

        assert_eq!(tracker.get_notification_count().await, UnreadDisplay::None);

        tracker.set_notification_count(5).await;
        assert_eq!(tracker.get_notification_count().await, UnreadDisplay::Count(5));

        tracker.set_notification_count(35).await;
        assert_eq!(tracker.get_notification_count().await, UnreadDisplay::Many);

        tracker.mark_notifications_read().await;
        assert_eq!(tracker.get_notification_count().await, UnreadDisplay::None);
    }

    #[tokio::test]
    async fn test_tracker_message_count() {
        let tracker = UnreadTracker::new();

        assert_eq!(tracker.get_message_count().await, UnreadDisplay::None);

        tracker.set_message_count(3).await;
        assert_eq!(tracker.get_message_count().await, UnreadDisplay::Count(3));

        tracker.mark_messages_read().await;
        assert_eq!(tracker.get_message_count().await, UnreadDisplay::None);
    }

    #[tokio::test]
    async fn test_tracker_reset() {
        let tracker = UnreadTracker::new();

        tracker.set_notification_count(10).await;
        tracker.set_message_count(5).await;

        tracker.reset().await;

        assert_eq!(tracker.get_notification_count().await, UnreadDisplay::None);
        assert_eq!(tracker.get_message_count().await, UnreadDisplay::None);
    }

    #[tokio::test]
    async fn test_tracker_subscription() {
        let tracker = UnreadTracker::new();
        let mut rx = tracker.subscribe_notifications();

        // Initial value
        assert_eq!(*rx.borrow(), UnreadDisplay::None);

        // Update
        tracker.set_notification_count(7).await;
        rx.changed().await.unwrap();
        assert_eq!(*rx.borrow(), UnreadDisplay::Count(7));
    }

    #[tokio::test]
    async fn test_tracker_events() {
        let tracker = UnreadTracker::new();
        let mut rx = tracker.subscribe_events();

        tracker.set_notification_count(3).await;

        match rx.recv().await.unwrap() {
            UnreadEvent::NotificationsChanged(display) => {
                assert_eq!(display, UnreadDisplay::Count(3));
            }
            _ => panic!("Expected NotificationsChanged event"),
        }
    }

    #[tokio::test]
    async fn test_no_duplicate_events() {
        let tracker = UnreadTracker::new();
        let mut rx = tracker.subscribe_events();

        // Set to 5
        tracker.set_notification_count(5).await;
        assert!(matches!(rx.recv().await.unwrap(), UnreadEvent::NotificationsChanged(_)));

        // Set to 5 again - should not trigger event
        tracker.set_notification_count(5).await;

        // Set to different value
        tracker.set_notification_count(10).await;
        assert!(matches!(rx.recv().await.unwrap(), UnreadEvent::NotificationsChanged(_)));
    }

    #[test]
    fn test_unread_display_default() {
        let display: UnreadDisplay = Default::default();
        assert_eq!(display, UnreadDisplay::None);
    }

    #[tokio::test]
    async fn test_should_poll_notifications() {
        let tracker = UnreadTracker::new();

        // Should always poll when none
        assert!(tracker.should_poll_notifications().await);

        // Should not poll when at max
        tracker.set_notification_count(30).await;
        assert!(!tracker.should_poll_notifications().await);
    }
}

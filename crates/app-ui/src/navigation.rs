//! Navigation system for Aurora Compass
//!
//! This module provides a type-safe navigation framework with:
//! - Navigation stack management
//! - Tab navigation
//! - Route definitions with deep linking support
//! - Navigation state management
//! - Transition animations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Route Parameters
// =============================================================================

/// Parameters for a route
pub type RouteParams = HashMap<String, String>;

/// Result of matching a route
#[derive(Debug, Clone, PartialEq)]
pub struct MatchResult {
    /// The matched parameters
    pub params: RouteParams,
}

// =============================================================================
// Route Definitions
// =============================================================================

/// All possible routes in the application
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "route", content = "params")]
pub enum Route {
    // Main tabs
    /// Home feed
    Home,
    /// Search/Explore
    Search {
        /// Search query
        #[serde(skip_serializing_if = "Option::is_none")]
        q: Option<String>,
        /// Search tab (user, profile, feed)
        #[serde(skip_serializing_if = "Option::is_none")]
        tab: Option<SearchTab>,
    },
    /// Feeds list
    Feeds,
    /// Notifications
    Notifications,
    /// Messages/Chat list
    Messages {
        /// Conversation to push to
        #[serde(skip_serializing_if = "Option::is_none")]
        push_to_conversation: Option<String>,
    },

    // Profile routes
    /// Profile view
    Profile {
        /// Handle or DID
        name: String,
        /// Hide back button (for tab root)
        #[serde(skip_serializing_if = "Option::is_none")]
        hide_back_button: Option<bool>,
    },
    /// Profile followers list
    ProfileFollowers {
        /// Handle or DID
        name: String,
    },
    /// Profile follows list
    ProfileFollows {
        /// Handle or DID
        name: String,
    },
    /// Known followers
    ProfileKnownFollowers {
        /// Handle or DID
        name: String,
    },
    /// Profile search
    ProfileSearch {
        /// Handle or DID
        name: String,
        /// Search query
        #[serde(skip_serializing_if = "Option::is_none")]
        q: Option<String>,
    },
    /// Profile list
    ProfileList {
        /// Handle or DID
        name: String,
        /// Record key
        rkey: String,
    },

    // Post routes
    /// Post thread view
    PostThread {
        /// Author handle or DID
        name: String,
        /// Record key
        rkey: String,
    },
    /// Users who liked the post
    PostLikedBy {
        /// Author handle or DID
        name: String,
        /// Record key
        rkey: String,
    },
    /// Users who reposted
    PostRepostedBy {
        /// Author handle or DID
        name: String,
        /// Record key
        rkey: String,
    },
    /// Quote posts
    PostQuotes {
        /// Author handle or DID
        name: String,
        /// Record key
        rkey: String,
    },

    // Feed routes
    /// Custom feed view
    ProfileFeed {
        /// Creator handle or DID
        name: String,
        /// Record key
        rkey: String,
    },
    /// Feed liked by
    ProfileFeedLikedBy {
        /// Creator handle or DID
        name: String,
        /// Record key
        rkey: String,
    },

    // Hashtag/Topic
    /// Hashtag feed
    Hashtag {
        /// Tag name
        tag: String,
        /// Filter by author
        #[serde(skip_serializing_if = "Option::is_none")]
        author: Option<String>,
    },
    /// Topic feed
    Topic {
        /// Topic name
        topic: String,
    },

    // Messages
    /// Chat conversation
    MessagesConversation {
        /// Conversation ID
        conversation: String,
        /// Embed to share
        #[serde(skip_serializing_if = "Option::is_none")]
        embed: Option<String>,
    },
    /// Messages settings
    MessagesSettings,
    /// Messages inbox (requests)
    MessagesInbox,

    // Lists
    /// User lists
    Lists,

    // Moderation
    /// Moderation settings
    Moderation,
    /// Mod lists
    ModerationModlists,
    /// Muted accounts
    ModerationMutedAccounts,
    /// Blocked accounts
    ModerationBlockedAccounts,
    /// Interaction settings
    ModerationInteractionSettings,

    // Settings
    /// Main settings
    Settings,
    /// Language settings
    LanguageSettings,
    /// App passwords
    AppPasswords,
    /// Saved feeds
    SavedFeeds,
    /// Following feed preferences
    PreferencesFollowingFeed,
    /// Thread preferences
    PreferencesThreads,
    /// External embeds preferences
    PreferencesExternalEmbeds,
    /// Accessibility settings
    AccessibilitySettings,
    /// Appearance settings
    AppearanceSettings,
    /// Account settings
    AccountSettings,
    /// Privacy and security
    PrivacyAndSecuritySettings,
    /// Content and media
    ContentAndMediaSettings,
    /// Notification settings
    NotificationSettings,
    /// About
    AboutSettings,

    // Starter packs
    /// View starter pack
    StarterPack {
        /// Creator handle or DID
        name: String,
        /// Record key
        rkey: String,
    },
    /// Create starter pack wizard
    StarterPackWizard,
    /// Edit starter pack
    StarterPackEdit {
        /// Record key
        rkey: String,
    },

    // Bookmarks
    /// Saved posts
    Bookmarks,

    // Support
    /// Support page
    Support,
    /// Privacy policy
    PrivacyPolicy,
    /// Terms of service
    TermsOfService,
    /// Community guidelines
    CommunityGuidelines,
    /// Copyright policy
    CopyrightPolicy,

    // Composer
    /// Post composer
    Composer {
        /// Reply to post URI
        #[serde(skip_serializing_if = "Option::is_none")]
        reply_to: Option<String>,
        /// Quote post URI
        #[serde(skip_serializing_if = "Option::is_none")]
        quote: Option<String>,
        /// Initial text
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
    },

    // Auth
    /// Login screen
    Login,
    /// Create account
    CreateAccount,

    // Error
    /// Not found
    NotFound,
}

/// Search tab options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchTab {
    /// Search users
    User,
    /// Search profiles
    Profile,
    /// Search feeds
    Feed,
}

impl Default for Route {
    fn default() -> Self {
        Route::Home
    }
}

impl Route {
    /// Get the URL path for this route
    pub fn to_path(&self) -> String {
        match self {
            Route::Home => "/".to_string(),
            Route::Search { q, tab } => {
                let mut path = "/search".to_string();
                let mut params = vec![];
                if let Some(q) = q {
                    params.push(format!("q={}", urlencoding::encode(q)));
                }
                if let Some(tab) = tab {
                    params.push(format!(
                        "tab={}",
                        match tab {
                            SearchTab::User => "user",
                            SearchTab::Profile => "profile",
                            SearchTab::Feed => "feed",
                        }
                    ));
                }
                if !params.is_empty() {
                    path.push('?');
                    path.push_str(&params.join("&"));
                }
                path
            }
            Route::Feeds => "/feeds".to_string(),
            Route::Notifications => "/notifications".to_string(),
            Route::Messages { .. } => "/messages".to_string(),
            Route::Profile { name, .. } => format!("/profile/{}", urlencoding::encode(name)),
            Route::ProfileFollowers { name } => {
                format!("/profile/{}/followers", urlencoding::encode(name))
            }
            Route::ProfileFollows { name } => {
                format!("/profile/{}/follows", urlencoding::encode(name))
            }
            Route::ProfileKnownFollowers { name } => {
                format!("/profile/{}/known-followers", urlencoding::encode(name))
            }
            Route::ProfileSearch { name, q } => {
                let mut path = format!("/profile/{}/search", urlencoding::encode(name));
                if let Some(q) = q {
                    path.push_str(&format!("?q={}", urlencoding::encode(q)));
                }
                path
            }
            Route::ProfileList { name, rkey } => format!(
                "/profile/{}/lists/{}",
                urlencoding::encode(name),
                urlencoding::encode(rkey)
            ),
            Route::PostThread { name, rkey } => format!(
                "/profile/{}/post/{}",
                urlencoding::encode(name),
                urlencoding::encode(rkey)
            ),
            Route::PostLikedBy { name, rkey } => format!(
                "/profile/{}/post/{}/liked-by",
                urlencoding::encode(name),
                urlencoding::encode(rkey)
            ),
            Route::PostRepostedBy { name, rkey } => format!(
                "/profile/{}/post/{}/reposted-by",
                urlencoding::encode(name),
                urlencoding::encode(rkey)
            ),
            Route::PostQuotes { name, rkey } => format!(
                "/profile/{}/post/{}/quotes",
                urlencoding::encode(name),
                urlencoding::encode(rkey)
            ),
            Route::ProfileFeed { name, rkey } => format!(
                "/profile/{}/feed/{}",
                urlencoding::encode(name),
                urlencoding::encode(rkey)
            ),
            Route::ProfileFeedLikedBy { name, rkey } => format!(
                "/profile/{}/feed/{}/liked-by",
                urlencoding::encode(name),
                urlencoding::encode(rkey)
            ),
            Route::Hashtag { tag, author } => {
                let mut path = format!("/hashtag/{}", urlencoding::encode(tag));
                if let Some(author) = author {
                    path.push_str(&format!("?author={}", urlencoding::encode(author)));
                }
                path
            }
            Route::Topic { topic } => format!("/topic/{}", urlencoding::encode(topic)),
            Route::MessagesConversation { conversation, .. } => {
                format!("/messages/{}", urlencoding::encode(conversation))
            }
            Route::MessagesSettings => "/messages/settings".to_string(),
            Route::MessagesInbox => "/messages/inbox".to_string(),
            Route::Lists => "/lists".to_string(),
            Route::Moderation => "/moderation".to_string(),
            Route::ModerationModlists => "/moderation/modlists".to_string(),
            Route::ModerationMutedAccounts => "/moderation/muted-accounts".to_string(),
            Route::ModerationBlockedAccounts => "/moderation/blocked-accounts".to_string(),
            Route::ModerationInteractionSettings => "/moderation/interaction-settings".to_string(),
            Route::Settings => "/settings".to_string(),
            Route::LanguageSettings => "/settings/language".to_string(),
            Route::AppPasswords => "/settings/app-passwords".to_string(),
            Route::SavedFeeds => "/settings/saved-feeds".to_string(),
            Route::PreferencesFollowingFeed => "/settings/following-feed".to_string(),
            Route::PreferencesThreads => "/settings/threads".to_string(),
            Route::PreferencesExternalEmbeds => "/settings/external-embeds".to_string(),
            Route::AccessibilitySettings => "/settings/accessibility".to_string(),
            Route::AppearanceSettings => "/settings/appearance".to_string(),
            Route::AccountSettings => "/settings/account".to_string(),
            Route::PrivacyAndSecuritySettings => "/settings/privacy-and-security".to_string(),
            Route::ContentAndMediaSettings => "/settings/content-and-media".to_string(),
            Route::NotificationSettings => "/settings/notifications".to_string(),
            Route::AboutSettings => "/settings/about".to_string(),
            Route::StarterPack { name, rkey } => format!(
                "/starter-pack/{}/{}",
                urlencoding::encode(name),
                urlencoding::encode(rkey)
            ),
            Route::StarterPackWizard => "/starter-pack/create".to_string(),
            Route::StarterPackEdit { rkey } => {
                format!("/starter-pack/edit/{}", urlencoding::encode(rkey))
            }
            Route::Bookmarks => "/saved".to_string(),
            Route::Support => "/support".to_string(),
            Route::PrivacyPolicy => "/support/privacy".to_string(),
            Route::TermsOfService => "/support/tos".to_string(),
            Route::CommunityGuidelines => "/support/community-guidelines".to_string(),
            Route::CopyrightPolicy => "/support/copyright".to_string(),
            Route::Composer { .. } => "/compose".to_string(),
            Route::Login => "/login".to_string(),
            Route::CreateAccount => "/create-account".to_string(),
            Route::NotFound => "/not-found".to_string(),
        }
    }

    /// Check if this route requires authentication
    pub fn requires_auth(&self) -> bool {
        matches!(
            self,
            Route::Notifications
                | Route::Messages { .. }
                | Route::MessagesConversation { .. }
                | Route::MessagesSettings
                | Route::MessagesInbox
                | Route::Lists
                | Route::Moderation
                | Route::ModerationModlists
                | Route::ModerationMutedAccounts
                | Route::ModerationBlockedAccounts
                | Route::ModerationInteractionSettings
                | Route::Settings
                | Route::LanguageSettings
                | Route::AppPasswords
                | Route::SavedFeeds
                | Route::PreferencesFollowingFeed
                | Route::PreferencesThreads
                | Route::PreferencesExternalEmbeds
                | Route::AccessibilitySettings
                | Route::AppearanceSettings
                | Route::AccountSettings
                | Route::PrivacyAndSecuritySettings
                | Route::ContentAndMediaSettings
                | Route::NotificationSettings
                | Route::AboutSettings
                | Route::Bookmarks
                | Route::Composer { .. }
                | Route::StarterPackWizard
                | Route::StarterPackEdit { .. }
        )
    }

    /// Get a display title for this route
    pub fn title(&self) -> &'static str {
        match self {
            Route::Home => "Home",
            Route::Search { .. } => "Search",
            Route::Feeds => "Feeds",
            Route::Notifications => "Notifications",
            Route::Messages { .. } => "Messages",
            Route::Profile { .. } => "Profile",
            Route::ProfileFollowers { .. } => "Followers",
            Route::ProfileFollows { .. } => "Following",
            Route::ProfileKnownFollowers { .. } => "Known Followers",
            Route::ProfileSearch { .. } => "Search Posts",
            Route::ProfileList { .. } => "List",
            Route::PostThread { .. } => "Post",
            Route::PostLikedBy { .. } => "Liked By",
            Route::PostRepostedBy { .. } => "Reposted By",
            Route::PostQuotes { .. } => "Quotes",
            Route::ProfileFeed { .. } => "Feed",
            Route::ProfileFeedLikedBy { .. } => "Liked By",
            Route::Hashtag { .. } => "Hashtag",
            Route::Topic { .. } => "Topic",
            Route::MessagesConversation { .. } => "Chat",
            Route::MessagesSettings => "Chat Settings",
            Route::MessagesInbox => "Message Requests",
            Route::Lists => "Lists",
            Route::Moderation => "Moderation",
            Route::ModerationModlists => "Moderation Lists",
            Route::ModerationMutedAccounts => "Muted Accounts",
            Route::ModerationBlockedAccounts => "Blocked Accounts",
            Route::ModerationInteractionSettings => "Interaction Settings",
            Route::Settings => "Settings",
            Route::LanguageSettings => "Language",
            Route::AppPasswords => "App Passwords",
            Route::SavedFeeds => "Saved Feeds",
            Route::PreferencesFollowingFeed => "Following Feed",
            Route::PreferencesThreads => "Threads",
            Route::PreferencesExternalEmbeds => "External Media",
            Route::AccessibilitySettings => "Accessibility",
            Route::AppearanceSettings => "Appearance",
            Route::AccountSettings => "Account",
            Route::PrivacyAndSecuritySettings => "Privacy & Security",
            Route::ContentAndMediaSettings => "Content & Media",
            Route::NotificationSettings => "Notifications",
            Route::AboutSettings => "About",
            Route::StarterPack { .. } => "Starter Pack",
            Route::StarterPackWizard => "Create Starter Pack",
            Route::StarterPackEdit { .. } => "Edit Starter Pack",
            Route::Bookmarks => "Saved Posts",
            Route::Support => "Support",
            Route::PrivacyPolicy => "Privacy Policy",
            Route::TermsOfService => "Terms of Service",
            Route::CommunityGuidelines => "Community Guidelines",
            Route::CopyrightPolicy => "Copyright Policy",
            Route::Composer { .. } => "New Post",
            Route::Login => "Log In",
            Route::CreateAccount => "Create Account",
            Route::NotFound => "Not Found",
        }
    }
}

// =============================================================================
// Navigation Tabs
// =============================================================================

/// Main navigation tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NavigationTab {
    /// Home tab
    #[default]
    Home,
    /// Search/Explore tab
    Search,
    /// Messages/Chat tab
    Messages,
    /// Notifications tab
    Notifications,
    /// Profile tab
    Profile,
}

impl NavigationTab {
    /// Get the root route for this tab
    pub fn root_route(&self) -> Route {
        match self {
            NavigationTab::Home => Route::Home,
            NavigationTab::Search => Route::Search { q: None, tab: None },
            NavigationTab::Messages => Route::Messages {
                push_to_conversation: None,
            },
            NavigationTab::Notifications => Route::Notifications,
            NavigationTab::Profile => Route::Profile {
                name: "me".to_string(),
                hide_back_button: Some(true),
            },
        }
    }

    /// Get icon name for this tab
    pub fn icon(&self) -> &'static str {
        match self {
            NavigationTab::Home => "home",
            NavigationTab::Search => "search",
            NavigationTab::Messages => "chat",
            NavigationTab::Notifications => "bell",
            NavigationTab::Profile => "user",
        }
    }

    /// Get label for this tab
    pub fn label(&self) -> &'static str {
        match self {
            NavigationTab::Home => "Home",
            NavigationTab::Search => "Search",
            NavigationTab::Messages => "Chat",
            NavigationTab::Notifications => "Notifications",
            NavigationTab::Profile => "Profile",
        }
    }

    /// Get all tabs in order
    pub fn all() -> [NavigationTab; 5] {
        [
            NavigationTab::Home,
            NavigationTab::Search,
            NavigationTab::Messages,
            NavigationTab::Notifications,
            NavigationTab::Profile,
        ]
    }
}

// =============================================================================
// Navigation Stack
// =============================================================================

/// A navigation stack entry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StackEntry {
    /// The route
    pub route: Route,
    /// Unique key for this entry
    pub key: String,
    /// Scroll position to restore
    #[serde(default)]
    pub scroll_position: f32,
}

impl StackEntry {
    /// Create a new stack entry
    pub fn new(route: Route) -> Self {
        Self {
            route,
            key: uuid::Uuid::new_v4().to_string(),
            scroll_position: 0.0,
        }
    }
}

/// Navigation stack for a tab
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavigationStack {
    /// Stack entries (bottom to top)
    entries: Vec<StackEntry>,
    /// Root route for this stack
    root: Route,
}

impl NavigationStack {
    /// Create a new navigation stack with a root route
    pub fn new(root: Route) -> Self {
        Self {
            entries: vec![StackEntry::new(root.clone())],
            root,
        }
    }

    /// Push a route onto the stack
    pub fn push(&mut self, route: Route) {
        self.entries.push(StackEntry::new(route));
    }

    /// Pop the top route (returns true if popped, false if at root)
    pub fn pop(&mut self) -> bool {
        if self.entries.len() > 1 {
            self.entries.pop();
            true
        } else {
            false
        }
    }

    /// Pop to root
    pub fn pop_to_root(&mut self) {
        self.entries.truncate(1);
    }

    /// Replace the top route
    pub fn replace(&mut self, route: Route) {
        if let Some(last) = self.entries.last_mut() {
            *last = StackEntry::new(route);
        }
    }

    /// Get the current (top) route
    pub fn current(&self) -> &Route {
        &self.entries.last().expect("Stack should never be empty").route
    }

    /// Get the current stack entry
    pub fn current_entry(&self) -> &StackEntry {
        self.entries.last().expect("Stack should never be empty")
    }

    /// Get mutable reference to current entry
    pub fn current_entry_mut(&mut self) -> &mut StackEntry {
        self.entries.last_mut().expect("Stack should never be empty")
    }

    /// Check if we can go back
    pub fn can_go_back(&self) -> bool {
        self.entries.len() > 1
    }

    /// Get stack depth
    pub fn depth(&self) -> usize {
        self.entries.len()
    }

    /// Get all entries
    pub fn entries(&self) -> &[StackEntry] {
        &self.entries
    }

    /// Reset to a new root
    pub fn reset(&mut self, route: Route) {
        self.root = route.clone();
        self.entries = vec![StackEntry::new(route)];
    }
}

// =============================================================================
// Navigation State
// =============================================================================

/// Animation type for navigation transitions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NavigationAnimation {
    /// Push animation (slide in from right)
    #[default]
    Push,
    /// Pop animation (slide out to right)
    Pop,
    /// Fade animation
    Fade,
    /// None (instant)
    None,
}

/// Pending navigation action
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingNavigation {
    /// Target route
    pub route: Route,
    /// Animation type
    pub animation: NavigationAnimation,
    /// Target tab (if switching)
    pub target_tab: Option<NavigationTab>,
}

/// Complete navigation state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavigationState {
    /// Current active tab
    pub active_tab: NavigationTab,
    /// Stacks for each tab
    pub tab_stacks: HashMap<NavigationTab, NavigationStack>,
    /// Modal stack (overlays on top of tab content)
    pub modal_stack: Vec<StackEntry>,
    /// Pending navigation (for animations)
    #[serde(skip)]
    pub pending: Option<PendingNavigation>,
    /// Is navigation in progress
    #[serde(skip)]
    pub is_navigating: bool,
}

impl Default for NavigationState {
    fn default() -> Self {
        let mut tab_stacks = HashMap::new();
        for tab in NavigationTab::all() {
            tab_stacks.insert(tab, NavigationStack::new(tab.root_route()));
        }

        Self {
            active_tab: NavigationTab::Home,
            tab_stacks,
            modal_stack: Vec::new(),
            pending: None,
            is_navigating: false,
        }
    }
}

impl NavigationState {
    /// Create a new navigation state
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the current stack for the active tab
    pub fn current_stack(&self) -> &NavigationStack {
        self.tab_stacks
            .get(&self.active_tab)
            .expect("All tabs should have stacks")
    }

    /// Get mutable current stack
    pub fn current_stack_mut(&mut self) -> &mut NavigationStack {
        self.tab_stacks
            .get_mut(&self.active_tab)
            .expect("All tabs should have stacks")
    }

    /// Get the current route (considering modals)
    pub fn current_route(&self) -> &Route {
        if let Some(modal) = self.modal_stack.last() {
            &modal.route
        } else {
            self.current_stack().current()
        }
    }

    /// Navigate to a route
    pub fn navigate(&mut self, route: Route) {
        self.pending = Some(PendingNavigation {
            route: route.clone(),
            animation: NavigationAnimation::Push,
            target_tab: None,
        });
        self.current_stack_mut().push(route);
    }

    /// Navigate to a route with animation
    pub fn navigate_with_animation(&mut self, route: Route, animation: NavigationAnimation) {
        self.pending = Some(PendingNavigation {
            route: route.clone(),
            animation,
            target_tab: None,
        });
        self.current_stack_mut().push(route);
    }

    /// Go back
    pub fn go_back(&mut self) -> bool {
        // First try to dismiss a modal
        if !self.modal_stack.is_empty() {
            self.modal_stack.pop();
            return true;
        }

        // Then try to pop from current stack
        if self.current_stack_mut().pop() {
            self.pending = Some(PendingNavigation {
                route: self.current_route().clone(),
                animation: NavigationAnimation::Pop,
                target_tab: None,
            });
            true
        } else {
            false
        }
    }

    /// Switch to a tab
    pub fn switch_tab(&mut self, tab: NavigationTab) {
        if self.active_tab != tab {
            self.pending = Some(PendingNavigation {
                route: self
                    .tab_stacks
                    .get(&tab)
                    .map(|s| s.current().clone())
                    .unwrap_or_else(|| tab.root_route()),
                animation: NavigationAnimation::None,
                target_tab: Some(tab),
            });
            self.active_tab = tab;
        }
    }

    /// Reset to tab root
    pub fn reset_to_tab(&mut self, tab: NavigationTab) {
        if let Some(stack) = self.tab_stacks.get_mut(&tab) {
            stack.pop_to_root();
        }
        self.active_tab = tab;
    }

    /// Present a modal
    pub fn present_modal(&mut self, route: Route) {
        self.modal_stack.push(StackEntry::new(route));
    }

    /// Dismiss the top modal
    pub fn dismiss_modal(&mut self) -> bool {
        if !self.modal_stack.is_empty() {
            self.modal_stack.pop();
            true
        } else {
            false
        }
    }

    /// Dismiss all modals
    pub fn dismiss_all_modals(&mut self) {
        self.modal_stack.clear();
    }

    /// Check if any modals are presented
    pub fn has_modals(&self) -> bool {
        !self.modal_stack.is_empty()
    }

    /// Complete the pending navigation
    pub fn complete_navigation(&mut self) {
        self.pending = None;
        self.is_navigating = false;
    }

    /// Check if we can go back
    pub fn can_go_back(&self) -> bool {
        !self.modal_stack.is_empty() || self.current_stack().can_go_back()
    }

    /// Reset entire navigation state
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

// =============================================================================
// Router
// =============================================================================

/// Route pattern for matching
struct RoutePattern {
    /// Pattern segments
    segments: Vec<PatternSegment>,
    /// Route builder
    builder: fn(RouteParams) -> Option<Route>,
}

/// Segment type in a pattern
#[derive(Debug, Clone)]
enum PatternSegment {
    /// Literal segment
    Literal(String),
    /// Parameter segment
    Param(String),
}

/// URL Router for parsing paths to routes
pub struct Router {
    /// Route patterns
    patterns: Vec<RoutePattern>,
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    /// Create a new router with all routes
    pub fn new() -> Self {
        let mut router = Self {
            patterns: Vec::new(),
        };

        // Register all routes
        router.add_route("/", |_| Some(Route::Home));
        router.add_route("/search", |params| {
            Some(Route::Search {
                q: params.get("q").cloned(),
                tab: params.get("tab").and_then(|t| match t.as_str() {
                    "user" => Some(SearchTab::User),
                    "profile" => Some(SearchTab::Profile),
                    "feed" => Some(SearchTab::Feed),
                    _ => None,
                }),
            })
        });
        router.add_route("/feeds", |_| Some(Route::Feeds));
        router.add_route("/notifications", |_| Some(Route::Notifications));
        router.add_route("/messages", |params| {
            Some(Route::Messages {
                push_to_conversation: params.get("pushToConversation").cloned(),
            })
        });

        // Profile routes
        router.add_route("/profile/:name", |params| {
            Some(Route::Profile {
                name: params.get("name")?.clone(),
                hide_back_button: None,
            })
        });
        router.add_route("/profile/:name/followers", |params| {
            Some(Route::ProfileFollowers {
                name: params.get("name")?.clone(),
            })
        });
        router.add_route("/profile/:name/follows", |params| {
            Some(Route::ProfileFollows {
                name: params.get("name")?.clone(),
            })
        });
        router.add_route("/profile/:name/known-followers", |params| {
            Some(Route::ProfileKnownFollowers {
                name: params.get("name")?.clone(),
            })
        });
        router.add_route("/profile/:name/search", |params| {
            Some(Route::ProfileSearch {
                name: params.get("name")?.clone(),
                q: params.get("q").cloned(),
            })
        });
        router.add_route("/profile/:name/lists/:rkey", |params| {
            Some(Route::ProfileList {
                name: params.get("name")?.clone(),
                rkey: params.get("rkey")?.clone(),
            })
        });
        router.add_route("/profile/:name/post/:rkey", |params| {
            Some(Route::PostThread {
                name: params.get("name")?.clone(),
                rkey: params.get("rkey")?.clone(),
            })
        });
        router.add_route("/profile/:name/post/:rkey/liked-by", |params| {
            Some(Route::PostLikedBy {
                name: params.get("name")?.clone(),
                rkey: params.get("rkey")?.clone(),
            })
        });
        router.add_route("/profile/:name/post/:rkey/reposted-by", |params| {
            Some(Route::PostRepostedBy {
                name: params.get("name")?.clone(),
                rkey: params.get("rkey")?.clone(),
            })
        });
        router.add_route("/profile/:name/post/:rkey/quotes", |params| {
            Some(Route::PostQuotes {
                name: params.get("name")?.clone(),
                rkey: params.get("rkey")?.clone(),
            })
        });
        router.add_route("/profile/:name/feed/:rkey", |params| {
            Some(Route::ProfileFeed {
                name: params.get("name")?.clone(),
                rkey: params.get("rkey")?.clone(),
            })
        });
        router.add_route("/profile/:name/feed/:rkey/liked-by", |params| {
            Some(Route::ProfileFeedLikedBy {
                name: params.get("name")?.clone(),
                rkey: params.get("rkey")?.clone(),
            })
        });

        // Hashtag/Topic
        router.add_route("/hashtag/:tag", |params| {
            Some(Route::Hashtag {
                tag: params.get("tag")?.clone(),
                author: params.get("author").cloned(),
            })
        });
        router.add_route("/topic/:topic", |params| {
            Some(Route::Topic {
                topic: params.get("topic")?.clone(),
            })
        });

        // Messages
        router.add_route("/messages/:conversation", |params| {
            Some(Route::MessagesConversation {
                conversation: params.get("conversation")?.clone(),
                embed: params.get("embed").cloned(),
            })
        });
        router.add_route("/messages/settings", |_| Some(Route::MessagesSettings));
        router.add_route("/messages/inbox", |_| Some(Route::MessagesInbox));

        // Lists and moderation
        router.add_route("/lists", |_| Some(Route::Lists));
        router.add_route("/moderation", |_| Some(Route::Moderation));
        router.add_route("/moderation/modlists", |_| Some(Route::ModerationModlists));
        router.add_route("/moderation/muted-accounts", |_| {
            Some(Route::ModerationMutedAccounts)
        });
        router.add_route("/moderation/blocked-accounts", |_| {
            Some(Route::ModerationBlockedAccounts)
        });
        router.add_route("/moderation/interaction-settings", |_| {
            Some(Route::ModerationInteractionSettings)
        });

        // Settings
        router.add_route("/settings", |_| Some(Route::Settings));
        router.add_route("/settings/language", |_| Some(Route::LanguageSettings));
        router.add_route("/settings/app-passwords", |_| Some(Route::AppPasswords));
        router.add_route("/settings/saved-feeds", |_| Some(Route::SavedFeeds));
        router.add_route("/settings/following-feed", |_| {
            Some(Route::PreferencesFollowingFeed)
        });
        router.add_route("/settings/threads", |_| Some(Route::PreferencesThreads));
        router.add_route("/settings/external-embeds", |_| {
            Some(Route::PreferencesExternalEmbeds)
        });
        router.add_route("/settings/accessibility", |_| {
            Some(Route::AccessibilitySettings)
        });
        router.add_route("/settings/appearance", |_| Some(Route::AppearanceSettings));
        router.add_route("/settings/account", |_| Some(Route::AccountSettings));
        router.add_route("/settings/privacy-and-security", |_| {
            Some(Route::PrivacyAndSecuritySettings)
        });
        router.add_route("/settings/content-and-media", |_| {
            Some(Route::ContentAndMediaSettings)
        });
        router.add_route("/settings/notifications", |_| {
            Some(Route::NotificationSettings)
        });
        router.add_route("/settings/about", |_| Some(Route::AboutSettings));

        // Starter packs
        router.add_route("/starter-pack/:name/:rkey", |params| {
            Some(Route::StarterPack {
                name: params.get("name")?.clone(),
                rkey: params.get("rkey")?.clone(),
            })
        });
        router.add_route("/starter-pack/create", |_| Some(Route::StarterPackWizard));
        router.add_route("/starter-pack/edit/:rkey", |params| {
            Some(Route::StarterPackEdit {
                rkey: params.get("rkey")?.clone(),
            })
        });

        // Bookmarks
        router.add_route("/saved", |_| Some(Route::Bookmarks));

        // Support
        router.add_route("/support", |_| Some(Route::Support));
        router.add_route("/support/privacy", |_| Some(Route::PrivacyPolicy));
        router.add_route("/support/tos", |_| Some(Route::TermsOfService));
        router.add_route("/support/community-guidelines", |_| {
            Some(Route::CommunityGuidelines)
        });
        router.add_route("/support/copyright", |_| Some(Route::CopyrightPolicy));

        // Composer
        router.add_route("/compose", |params| {
            Some(Route::Composer {
                reply_to: params.get("reply_to").cloned(),
                quote: params.get("quote").cloned(),
                text: params.get("text").cloned(),
            })
        });

        // Auth
        router.add_route("/login", |_| Some(Route::Login));
        router.add_route("/create-account", |_| Some(Route::CreateAccount));

        router
    }

    /// Add a route pattern
    fn add_route(&mut self, pattern: &str, builder: fn(RouteParams) -> Option<Route>) {
        let segments = pattern
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| {
                if let Some(param) = s.strip_prefix(':') {
                    PatternSegment::Param(param.to_string())
                } else {
                    PatternSegment::Literal(s.to_string())
                }
            })
            .collect();

        self.patterns.push(RoutePattern { segments, builder });
    }

    /// Match a path to a route
    pub fn match_path(&self, path: &str) -> Route {
        // Parse the path
        let (pathname, query) = if let Some(idx) = path.find('?') {
            (&path[..idx], Some(&path[idx + 1..]))
        } else {
            (path, None)
        };

        let path_segments: Vec<&str> = pathname.split('/').filter(|s| !s.is_empty()).collect();

        // Try each pattern
        for pattern in &self.patterns {
            if let Some(params) = self.match_pattern(&pattern.segments, &path_segments, query) {
                if let Some(route) = (pattern.builder)(params) {
                    return route;
                }
            }
        }

        Route::NotFound
    }

    /// Match a pattern against path segments
    fn match_pattern(
        &self,
        pattern: &[PatternSegment],
        path: &[&str],
        query: Option<&str>,
    ) -> Option<RouteParams> {
        if pattern.len() != path.len() {
            // Special case: root path
            if pattern.is_empty() && path.is_empty() {
                let mut params = RouteParams::new();
                self.parse_query(query, &mut params);
                return Some(params);
            }
            return None;
        }

        let mut params = RouteParams::new();

        for (segment, actual) in pattern.iter().zip(path.iter()) {
            match segment {
                PatternSegment::Literal(expected) => {
                    if expected != *actual {
                        return None;
                    }
                }
                PatternSegment::Param(name) => {
                    params.insert(
                        name.clone(),
                        urlencoding::decode(actual).ok()?.into_owned(),
                    );
                }
            }
        }

        // Parse query parameters
        self.parse_query(query, &mut params);

        Some(params)
    }

    /// Parse query string into params
    fn parse_query(&self, query: Option<&str>, params: &mut RouteParams) {
        if let Some(query) = query {
            for pair in query.split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    if let Ok(decoded) = urlencoding::decode(value) {
                        params.insert(key.to_string(), decoded.into_owned());
                    }
                }
            }
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_to_path() {
        assert_eq!(Route::Home.to_path(), "/");
        assert_eq!(Route::Notifications.to_path(), "/notifications");
        assert_eq!(
            Route::Profile {
                name: "alice.bsky.social".to_string(),
                hide_back_button: None
            }
            .to_path(),
            "/profile/alice.bsky.social"
        );
    }

    #[test]
    fn test_route_requires_auth() {
        assert!(!Route::Home.requires_auth());
        assert!(Route::Notifications.requires_auth());
        assert!(Route::Settings.requires_auth());
        assert!(!Route::Support.requires_auth());
    }

    #[test]
    fn test_router_match_home() {
        let router = Router::new();
        assert_eq!(router.match_path("/"), Route::Home);
    }

    #[test]
    fn test_router_match_profile() {
        let router = Router::new();
        assert_eq!(
            router.match_path("/profile/alice.bsky.social"),
            Route::Profile {
                name: "alice.bsky.social".to_string(),
                hide_back_button: None
            }
        );
    }

    #[test]
    fn test_router_match_post() {
        let router = Router::new();
        assert_eq!(
            router.match_path("/profile/alice.bsky.social/post/3k2yihx"),
            Route::PostThread {
                name: "alice.bsky.social".to_string(),
                rkey: "3k2yihx".to_string()
            }
        );
    }

    #[test]
    fn test_router_match_search_with_query() {
        let router = Router::new();
        let route = router.match_path("/search?q=hello&tab=user");
        assert_eq!(
            route,
            Route::Search {
                q: Some("hello".to_string()),
                tab: Some(SearchTab::User)
            }
        );
    }

    #[test]
    fn test_router_not_found() {
        let router = Router::new();
        assert_eq!(router.match_path("/nonexistent/path"), Route::NotFound);
    }

    #[test]
    fn test_navigation_tab_root_routes() {
        assert_eq!(NavigationTab::Home.root_route(), Route::Home);
        assert_eq!(NavigationTab::Notifications.root_route(), Route::Notifications);
    }

    #[test]
    fn test_navigation_stack_push_pop() {
        let mut stack = NavigationStack::new(Route::Home);
        assert_eq!(stack.depth(), 1);
        assert!(!stack.can_go_back());

        stack.push(Route::Notifications);
        assert_eq!(stack.depth(), 2);
        assert!(stack.can_go_back());
        assert_eq!(*stack.current(), Route::Notifications);

        assert!(stack.pop());
        assert_eq!(stack.depth(), 1);
        assert_eq!(*stack.current(), Route::Home);

        // Can't pop past root
        assert!(!stack.pop());
    }

    #[test]
    fn test_navigation_state_default() {
        let state = NavigationState::new();
        assert_eq!(state.active_tab, NavigationTab::Home);
        assert_eq!(*state.current_route(), Route::Home);
        assert!(!state.can_go_back());
    }

    #[test]
    fn test_navigation_state_navigate() {
        let mut state = NavigationState::new();
        state.navigate(Route::Notifications);
        assert_eq!(*state.current_route(), Route::Notifications);
        assert!(state.can_go_back());
    }

    #[test]
    fn test_navigation_state_switch_tab() {
        let mut state = NavigationState::new();
        state.switch_tab(NavigationTab::Search);
        assert_eq!(state.active_tab, NavigationTab::Search);
    }

    #[test]
    fn test_navigation_state_modal() {
        let mut state = NavigationState::new();
        assert!(!state.has_modals());

        state.present_modal(Route::Composer {
            reply_to: None,
            quote: None,
            text: None,
        });
        assert!(state.has_modals());

        // Current route should be the modal
        assert!(matches!(state.current_route(), Route::Composer { .. }));

        // Go back should dismiss modal
        assert!(state.go_back());
        assert!(!state.has_modals());
    }

    #[test]
    fn test_route_serialization() {
        let route = Route::PostThread {
            name: "alice".to_string(),
            rkey: "123".to_string(),
        };
        let json = serde_json::to_string(&route).unwrap();
        let parsed: Route = serde_json::from_str(&json).unwrap();
        assert_eq!(route, parsed);
    }

    #[test]
    fn test_navigation_state_serialization() {
        let state = NavigationState::new();
        let json = serde_json::to_string(&state).unwrap();
        let parsed: NavigationState = serde_json::from_str(&json).unwrap();
        assert_eq!(state.active_tab, parsed.active_tab);
    }

    #[test]
    fn test_route_title() {
        assert_eq!(Route::Home.title(), "Home");
        assert_eq!(Route::Settings.title(), "Settings");
        assert_eq!(
            Route::Profile {
                name: "test".to_string(),
                hide_back_button: None
            }
            .title(),
            "Profile"
        );
    }

    #[test]
    fn test_url_encoding_in_routes() {
        let route = Route::Hashtag {
            tag: "hello world".to_string(),
            author: None,
        };
        let path = route.to_path();
        assert_eq!(path, "/hashtag/hello%20world");
    }
}

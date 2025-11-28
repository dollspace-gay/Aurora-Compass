//! Rich text editor
//!
//! Provides a framework-agnostic rich text editor for post composition.
//! Handles text input, formatting, mention/hashtag/link detection, and
//! integrates with the rich text parsing system.

use crate::posts::{Facet, RichText};
use serde::{Deserialize, Serialize};
use std::ops::Range;

/// Maximum length for a post (AT Protocol limit)
pub const MAX_POST_LENGTH: usize = 300;

/// Rich text editor state
///
/// Manages the content and cursor position for a rich text editor.
/// Provides methods for text manipulation, formatting detection,
/// and mention/hashtag/link insertion.
///
/// # Example
///
/// ```
/// use app_core::editor::RichTextEditor;
///
/// let mut editor = RichTextEditor::new();
/// editor.insert_text("Hello world!");
/// assert_eq!(editor.text(), "Hello world!");
/// assert_eq!(editor.char_count(), 12);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RichTextEditor {
    /// The raw text content
    text: String,
    /// Current cursor position (byte offset)
    cursor: usize,
    /// Selection range (if any)
    selection: Option<Range<usize>>,
    /// Cached grapheme count (for character limit)
    grapheme_count: usize,
}

impl Default for RichTextEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl RichTextEditor {
    /// Creates a new empty editor
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            selection: None,
            grapheme_count: 0,
        }
    }

    /// Creates an editor with initial text
    pub fn with_text(text: impl Into<String>) -> Self {
        let text = text.into();
        let grapheme_count =
            unicode_segmentation::UnicodeSegmentation::graphemes(text.as_str(), true).count();
        Self {
            text: text.clone(),
            cursor: text.len(),
            selection: None,
            grapheme_count,
        }
    }

    /// Returns the current text content
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the character count (grapheme count)
    pub fn char_count(&self) -> usize {
        self.grapheme_count
    }

    /// Returns whether the text exceeds the maximum post length
    pub fn is_too_long(&self) -> bool {
        self.grapheme_count > MAX_POST_LENGTH
    }

    /// Returns the number of characters remaining
    pub fn chars_remaining(&self) -> isize {
        MAX_POST_LENGTH as isize - self.grapheme_count as isize
    }

    /// Returns the current cursor position
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Sets the cursor position
    pub fn set_cursor(&mut self, position: usize) {
        self.cursor = position.min(self.text.len());
    }

    /// Returns the current selection range
    pub fn selection(&self) -> Option<Range<usize>> {
        self.selection.clone()
    }

    /// Sets the selection range
    pub fn set_selection(&mut self, range: Option<Range<usize>>) {
        self.selection = range.map(|r| {
            let start = r.start.min(self.text.len());
            let end = r.end.min(self.text.len());
            start..end
        });
    }

    /// Inserts text at the current cursor position
    pub fn insert_text(&mut self, text: &str) {
        // If there's a selection, delete it first
        if let Some(selection) = self.selection.take() {
            self.delete_range(selection);
        }

        self.text.insert_str(self.cursor, text);
        self.cursor += text.len();
        self.update_grapheme_count();
    }

    /// Deletes the character before the cursor (backspace)
    pub fn delete_backward(&mut self) {
        if let Some(selection) = self.selection.take() {
            self.delete_range(selection);
            return;
        }

        if self.cursor > 0 {
            // Find the previous character boundary
            let mut prev_char_boundary = self.cursor;
            while prev_char_boundary > 0 {
                prev_char_boundary -= 1;
                if self.text.is_char_boundary(prev_char_boundary) {
                    break;
                }
            }

            self.text.drain(prev_char_boundary..self.cursor);
            self.cursor = prev_char_boundary;
            self.update_grapheme_count();
        }
    }

    /// Deletes the character after the cursor (delete)
    pub fn delete_forward(&mut self) {
        if let Some(selection) = self.selection.take() {
            self.delete_range(selection);
            return;
        }

        if self.cursor < self.text.len() {
            // Find the next character boundary
            let mut next_char_boundary = self.cursor + 1;
            while next_char_boundary < self.text.len()
                && !self.text.is_char_boundary(next_char_boundary)
            {
                next_char_boundary += 1;
            }

            self.text.drain(self.cursor..next_char_boundary);
            self.update_grapheme_count();
        }
    }

    /// Deletes a range of text
    pub fn delete_range(&mut self, range: Range<usize>) {
        let start = range.start.min(self.text.len());
        let end = range.end.min(self.text.len());

        if start < end {
            self.text.drain(start..end);
            self.cursor = start;
            self.update_grapheme_count();
        }
    }

    /// Clears all text
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
        self.selection = None;
        self.grapheme_count = 0;
    }

    /// Inserts a mention at the current cursor position
    ///
    /// # Arguments
    ///
    /// * `handle` - The user handle (without @)
    /// * `did` - The user's DID
    ///
    /// # Example
    ///
    /// ```
    /// use app_core::editor::RichTextEditor;
    ///
    /// let mut editor = RichTextEditor::new();
    /// editor.insert_mention("alice.bsky.social", "did:plc:alice123");
    /// assert_eq!(editor.text(), "@alice.bsky.social ");
    /// ```
    pub fn insert_mention(&mut self, handle: &str, _did: &str) {
        let mention_text = format!("@{} ", handle);
        self.insert_text(&mention_text);

        // Store the DID in metadata (this would be handled by the UI layer)
        // For now, the mention is just text that will be detected by detect_facets
    }

    /// Inserts a hashtag at the current cursor position
    ///
    /// # Example
    ///
    /// ```
    /// use app_core::editor::RichTextEditor;
    ///
    /// let mut editor = RichTextEditor::new();
    /// editor.insert_hashtag("rust");
    /// assert_eq!(editor.text(), "#rust ");
    /// ```
    pub fn insert_hashtag(&mut self, tag: &str) {
        let hashtag_text = format!("#{} ", tag);
        self.insert_text(&hashtag_text);
    }

    /// Inserts a link at the current cursor position
    ///
    /// # Example
    ///
    /// ```
    /// use app_core::editor::RichTextEditor;
    ///
    /// let mut editor = RichTextEditor::new();
    /// editor.insert_link("https://example.com");
    /// assert_eq!(editor.text(), "https://example.com ");
    /// ```
    pub fn insert_link(&mut self, url: &str) {
        let link_text = format!("{} ", url);
        self.insert_text(&link_text);
    }

    /// Detects facets (mentions, hashtags, links) in the current text
    pub fn detect_facets(&self) -> Vec<Facet> {
        let mut rt = RichText::new(&self.text);
        rt.detect_facets();
        rt.facets().map(|f| f.to_vec()).unwrap_or_default()
    }

    /// Converts the editor content to a RichText object
    ///
    /// # Example
    ///
    /// ```
    /// use app_core::editor::RichTextEditor;
    ///
    /// let mut editor = RichTextEditor::with_text("Hello @alice.bsky.social!");
    /// let rich_text = editor.to_rich_text();
    /// assert_eq!(rich_text.text(), "Hello @alice.bsky.social!");
    /// ```
    pub fn to_rich_text(&self) -> RichText {
        let mut rt = RichText::new(&self.text);
        rt.detect_facets();
        rt
    }

    /// Updates the grapheme count after text changes
    fn update_grapheme_count(&mut self) {
        self.grapheme_count =
            unicode_segmentation::UnicodeSegmentation::graphemes(self.text.as_str(), true).count();
    }
}

/// Autocomplete suggestion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutocompleteSuggestion {
    /// The type of suggestion
    pub suggestion_type: SuggestionType,
    /// The trigger position in the text
    pub trigger_position: usize,
    /// The query string after the trigger
    pub query: String,
}

/// Type of autocomplete suggestion
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SuggestionType {
    /// Mention suggestion (triggered by @)
    Mention,
    /// Hashtag suggestion (triggered by #)
    Hashtag,
}

impl RichTextEditor {
    /// Detects if the cursor is at a position where autocomplete should trigger
    ///
    /// Returns a suggestion if the cursor is after @ or # with a query string
    pub fn detect_autocomplete(&self) -> Option<AutocompleteSuggestion> {
        if self.cursor == 0 {
            return None;
        }

        // Find the start of the current word
        let text_before_cursor = &self.text[..self.cursor];

        // Check for @ mention
        if let Some(at_pos) = text_before_cursor.rfind('@') {
            let after_at = &text_before_cursor[at_pos + 1..];
            // Only trigger if there's no whitespace after @
            if !after_at.contains(char::is_whitespace) {
                return Some(AutocompleteSuggestion {
                    suggestion_type: SuggestionType::Mention,
                    trigger_position: at_pos,
                    query: after_at.to_string(),
                });
            }
        }

        // Check for # hashtag
        if let Some(hash_pos) = text_before_cursor.rfind('#') {
            let after_hash = &text_before_cursor[hash_pos + 1..];
            // Only trigger if there's no whitespace after #
            if !after_hash.contains(char::is_whitespace) {
                return Some(AutocompleteSuggestion {
                    suggestion_type: SuggestionType::Hashtag,
                    trigger_position: hash_pos,
                    query: after_hash.to_string(),
                });
            }
        }

        None
    }

    /// Applies an autocomplete suggestion
    ///
    /// Replaces the query with the selected completion
    pub fn apply_autocomplete(&mut self, suggestion: &AutocompleteSuggestion, completion: &str) {
        // Delete from trigger position to cursor
        let range = suggestion.trigger_position..self.cursor;
        self.delete_range(range);

        // Insert the completion
        match suggestion.suggestion_type {
            SuggestionType::Mention => {
                let mention_text = format!("@{} ", completion);
                self.insert_text(&mention_text);
            }
            SuggestionType::Hashtag => {
                let hashtag_text = format!("#{} ", completion);
                self.insert_text(&hashtag_text);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_editor() {
        let editor = RichTextEditor::new();
        assert_eq!(editor.text(), "");
        assert_eq!(editor.char_count(), 0);
        assert_eq!(editor.cursor(), 0);
        assert!(!editor.is_too_long());
    }

    #[test]
    fn test_with_text() {
        let editor = RichTextEditor::with_text("Hello world");
        assert_eq!(editor.text(), "Hello world");
        assert_eq!(editor.char_count(), 11);
        assert_eq!(editor.cursor(), 11);
    }

    #[test]
    fn test_insert_text() {
        let mut editor = RichTextEditor::new();
        editor.insert_text("Hello");
        assert_eq!(editor.text(), "Hello");
        assert_eq!(editor.char_count(), 5);
        assert_eq!(editor.cursor(), 5);

        editor.insert_text(" world");
        assert_eq!(editor.text(), "Hello world");
        assert_eq!(editor.char_count(), 11);
    }

    #[test]
    fn test_delete_backward() {
        let mut editor = RichTextEditor::with_text("Hello");
        editor.delete_backward();
        assert_eq!(editor.text(), "Hell");
        assert_eq!(editor.char_count(), 4);
        assert_eq!(editor.cursor(), 4);
    }

    #[test]
    fn test_delete_forward() {
        let mut editor = RichTextEditor::with_text("Hello");
        editor.set_cursor(0);
        editor.delete_forward();
        assert_eq!(editor.text(), "ello");
        assert_eq!(editor.char_count(), 4);
    }

    #[test]
    fn test_clear() {
        let mut editor = RichTextEditor::with_text("Hello world");
        editor.clear();
        assert_eq!(editor.text(), "");
        assert_eq!(editor.char_count(), 0);
        assert_eq!(editor.cursor(), 0);
    }

    #[test]
    fn test_char_count_unicode() {
        let mut editor = RichTextEditor::new();
        editor.insert_text("Hello 👋 World 🌍");
        // "Hello 👋 World 🌍" = 5 + 1 + 1 + 1 + 5 + 1 + 1 = 15 graphemes
        assert_eq!(editor.char_count(), 15);
    }

    #[test]
    fn test_chars_remaining() {
        let mut editor = RichTextEditor::new();
        editor.insert_text("Hello");
        assert_eq!(editor.chars_remaining(), 295);
    }

    #[test]
    fn test_is_too_long() {
        let mut editor = RichTextEditor::new();
        let long_text = "a".repeat(301);
        editor.insert_text(&long_text);
        assert!(editor.is_too_long());
    }

    #[test]
    fn test_insert_mention() {
        let mut editor = RichTextEditor::new();
        editor.insert_mention("alice.bsky.social", "did:plc:alice123");
        assert_eq!(editor.text(), "@alice.bsky.social ");
    }

    #[test]
    fn test_insert_hashtag() {
        let mut editor = RichTextEditor::new();
        editor.insert_hashtag("rust");
        assert_eq!(editor.text(), "#rust ");
    }

    #[test]
    fn test_insert_link() {
        let mut editor = RichTextEditor::new();
        editor.insert_link("https://example.com");
        assert_eq!(editor.text(), "https://example.com ");
    }

    #[test]
    fn test_detect_facets() {
        let mut editor = RichTextEditor::new();
        editor.insert_text("Check out https://example.com and @alice.bsky.social #rust");
        let facets = editor.detect_facets();
        assert!(!facets.is_empty());
    }

    #[test]
    fn test_to_rich_text() {
        let editor = RichTextEditor::with_text("Hello @alice.bsky.social!");
        let rich_text = editor.to_rich_text();
        assert_eq!(rich_text.text(), "Hello @alice.bsky.social!");
    }

    #[test]
    fn test_selection() {
        let mut editor = RichTextEditor::with_text("Hello world");
        editor.set_selection(Some(0..5));
        assert_eq!(editor.selection(), Some(0..5));

        // Insert with selection should replace
        editor.insert_text("Hi");
        assert_eq!(editor.text(), "Hi world");
        assert_eq!(editor.selection(), None);
    }

    #[test]
    fn test_delete_selection() {
        let mut editor = RichTextEditor::with_text("Hello world");
        editor.set_selection(Some(0..5));
        editor.delete_backward();
        assert_eq!(editor.text(), " world");
    }

    #[test]
    fn test_detect_autocomplete_mention() {
        let editor = RichTextEditor::with_text("Hello @ali");
        let suggestion = editor.detect_autocomplete();
        assert!(suggestion.is_some());
        let suggestion = suggestion.unwrap();
        assert_eq!(suggestion.suggestion_type, SuggestionType::Mention);
        assert_eq!(suggestion.query, "ali");
    }

    #[test]
    fn test_detect_autocomplete_hashtag() {
        let editor = RichTextEditor::with_text("Hello #rus");
        let suggestion = editor.detect_autocomplete();
        assert!(suggestion.is_some());
        let suggestion = suggestion.unwrap();
        assert_eq!(suggestion.suggestion_type, SuggestionType::Hashtag);
        assert_eq!(suggestion.query, "rus");
    }

    #[test]
    fn test_detect_autocomplete_no_trigger() {
        let editor = RichTextEditor::with_text("Hello world");
        let suggestion = editor.detect_autocomplete();
        assert!(suggestion.is_none());
    }

    #[test]
    fn test_detect_autocomplete_with_whitespace() {
        let editor = RichTextEditor::with_text("Hello @ alice");
        let suggestion = editor.detect_autocomplete();
        // Should not trigger because there's whitespace after @
        assert!(suggestion.is_none());
    }

    #[test]
    fn test_apply_autocomplete_mention() {
        let mut editor = RichTextEditor::with_text("Hello @ali");
        let suggestion = editor.detect_autocomplete().unwrap();
        editor.apply_autocomplete(&suggestion, "alice.bsky.social");
        assert_eq!(editor.text(), "Hello @alice.bsky.social ");
    }

    #[test]
    fn test_apply_autocomplete_hashtag() {
        let mut editor = RichTextEditor::with_text("Hello #rus");
        let suggestion = editor.detect_autocomplete().unwrap();
        editor.apply_autocomplete(&suggestion, "rust");
        assert_eq!(editor.text(), "Hello #rust ");
    }

    #[test]
    fn test_cursor_positioning() {
        let mut editor = RichTextEditor::with_text("Hello");
        editor.set_cursor(2);
        assert_eq!(editor.cursor(), 2);

        editor.insert_text("XX");
        assert_eq!(editor.text(), "HeXXllo");
        assert_eq!(editor.cursor(), 4);
    }

    #[test]
    fn test_emoji_handling() {
        let mut editor = RichTextEditor::new();
        editor.insert_text("Hello 👋");
        assert_eq!(editor.char_count(), 7);

        editor.delete_backward();
        assert_eq!(editor.text(), "Hello ");
        assert_eq!(editor.char_count(), 6);
    }
}

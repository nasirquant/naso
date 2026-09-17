//! Document Store - In-memory document management with incremental updates

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use dashmap::DashMap;
use tower_lsp::lsp_types::*;

/// Represents a document in the LSP document store
#[derive(Debug, Clone)]
pub struct Document {
    pub uri: Url,
    pub version: i32,
    pub content: String,
    /// Line start offsets for fast position conversion
    line_starts: Vec<usize>,
}

impl Document {
    pub fn new(uri: Url, content: String, version: i32) -> Self {
        let line_starts = Self::compute_line_starts(&content);
        Self {
            uri,
            version,
            content,
            line_starts,
        }
    }

    fn compute_line_starts(content: &str) -> Vec<usize> {
        let mut starts = vec![0];
        for (i, ch) in content.char_indices() {
            if ch == '\n' {
                starts.push(i + 1);
            }
        }
        starts
    }

    /// Convert a Position to a byte offset
    pub fn position_to_offset(&self, position: Position) -> usize {
        let line = position.line as usize;
        let char_offset = position.character as usize;

        if line >= self.line_starts.len() {
            return self.content.len();
        }

        let line_start = self.line_starts[line];
        let line_end = if line + 1 < self.line_starts.len() {
            self.line_starts[line + 1]
        } else {
            self.content.len()
        };

        // Clamp to line length
        let max_char = line_end.saturating_sub(line_start);
        (line_start + char_offset.min(max_char)).min(self.content.len())
    }

    /// Convert a byte offset to a Position
    pub fn offset_to_position(&self, offset: usize) -> Position {
        let offset = offset.min(self.content.len());

        // Binary search for the line
        let line = self.line_starts.partition_point(|&start| start <= offset);
        let line = line.saturating_sub(1);

        let line_start = self.line_starts.get(line).copied().unwrap_or(0);
        let character = offset.saturating_sub(line_start);

        Position::new(line as u32, character as u32)
    }

    /// Get a range as a string slice
    pub fn get_range(&self, range: Range) -> Option<&str> {
        let start = self.position_to_offset(range.start);
        let end = self.position_to_offset(range.end);
        self.content.get(start..end)
    }

    /// Update content with a full replacement
    pub fn update_full(&mut self, content: String, version: i32) {
        self.content = content;
        self.version = version;
        self.line_starts = Self::compute_line_starts(&self.content);
    }

    /// Update content incrementally
    pub fn update_incremental(&mut self, range: Range, new_text: String, version: i32) {
        let start = self.position_to_offset(range.start);
        let end = self.position_to_offset(range.end);

        // Replace the range
        let mut new_content =
            String::with_capacity(self.content.len() - (end - start) + new_text.len());
        new_content.push_str(&self.content[..start]);
        new_content.push_str(&new_text);
        new_content.push_str(&self.content[end..]);

        self.content = new_content;
        self.version = version;
        self.line_starts = Self::compute_line_starts(&self.content);
    }
}

/// Thread-safe document store for LSP
pub struct DocumentStore {
    documents: DashMap<Url, Document>,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self {
            documents: DashMap::new(),
        }
    }

    pub fn open(&self, uri: Url, content: String, version: i32) {
        let doc = Document::new(uri.clone(), content, version);
        self.documents.insert(uri, doc);
    }

    pub fn update_full(&self, uri: &Url, content: String, version: i32) {
        if let Some(mut doc) = self.documents.get_mut(uri) {
            doc.update_full(content, version);
        }
    }

    pub fn update_incremental(&self, uri: &Url, range: Range, new_text: String, version: i32) {
        if let Some(mut doc) = self.documents.get_mut(uri) {
            doc.update_incremental(range, new_text, version);
        }
    }

    pub fn close(&self, uri: &Url) {
        self.documents.remove(uri);
    }

    pub fn get(&self, uri: &Url) -> Option<Arc<Document>> {
        self.documents.get(uri).map(|doc| Arc::new(doc.clone()))
    }

    pub fn get_mut(&self, uri: &Url) -> Option<dashmap::mapref::one::RefMut<'_, Url, Document>> {
        self.documents.get_mut(uri)
    }
}

impl Default for DocumentStore {
    fn default() -> Self {
        Self::new()
    }
}

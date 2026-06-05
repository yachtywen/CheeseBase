use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const INDEX_VERSION: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupportedFileType {
    Markdown,
    Text,
    Rust,
    Toml,
    Pdf,
}

impl SupportedFileType {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Option<Self> {
        let ext = path.as_ref().extension()?.to_string_lossy().to_lowercase();
        match ext.as_str() {
            "md" | "markdown" => Some(Self::Markdown),
            "txt" => Some(Self::Text),
            "rs" => Some(Self::Rust),
            "toml" => Some(Self::Toml),
            "pdf" => Some(Self::Pdf),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: usize,
    pub path: PathBuf,
    pub title: String,
    pub file_type: SupportedFileType,
    pub modified_secs: u64,
    pub size_bytes: u64,
    pub token_count: usize,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Posting {
    pub doc_id: usize,
    pub frequency: usize,
    pub occurrences: Vec<IndexedOccurrence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedOccurrence {
    pub token_position: usize,
    pub char_start: usize,
    pub char_end: usize,
    pub page: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetadata {
    pub version: u32,
    pub root: PathBuf,
    pub document_count: usize,
    pub term_count: usize,
    pub total_tokens: usize,
    pub created_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvertedIndex {
    pub metadata: IndexMetadata,
    pub documents: Vec<Document>,
    pub postings: HashMap<String, Vec<Posting>>,
}

impl InvertedIndex {
    pub fn document(&self, doc_id: usize) -> Option<&Document> {
        self.documents.iter().find(|doc| doc.id == doc_id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenOccurrence {
    pub token: String,
    pub position: usize,
    pub char_start: usize,
    pub char_end: usize,
    pub page: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ParsedDocument {
    pub path: PathBuf,
    pub title: String,
    pub file_type: SupportedFileType,
    pub modified_secs: u64,
    pub size_bytes: u64,
    pub content: String,
    pub tokens: Vec<TokenOccurrence>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub doc_id: usize,
    pub title: String,
    pub path: PathBuf,
    pub score: f64,
    pub matched_terms: Vec<String>,
    pub snippet: String,
    pub matches: Vec<SearchMatch>,
}

#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub snippet: String,
    pub matched_terms: Vec<String>,
    pub token_position: usize,
    pub char_start: usize,
    pub char_end: usize,
    pub page: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchState {
    Empty,
    HasQuery,
    NoResults,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    Any,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchStrategy {
    Bm25,
    Hybrid,
}

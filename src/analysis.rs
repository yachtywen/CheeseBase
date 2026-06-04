use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::{AppError, AppResult};
use crate::model::{Document, InvertedIndex};
use crate::search::truncate_to;

#[derive(Debug, Clone)]
pub struct IndexSummary {
    pub root: PathBuf,
    pub document_count: usize,
    pub term_count: usize,
    pub total_tokens: usize,
    pub average_tokens_per_document: f64,
    pub largest_documents: Vec<DocumentStat>,
    pub top_terms: Vec<TermStat>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TermStat {
    pub term: String,
    pub total_frequency: usize,
    pub document_frequency: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocumentStat {
    pub doc_id: usize,
    pub title: String,
    pub path: PathBuf,
    pub token_count: usize,
    pub size_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct DocumentInspection {
    pub document: DocumentStat,
    pub top_terms: Vec<TermStat>,
    pub preview: String,
}

pub fn summarize_index(index: &InvertedIndex, limit: usize) -> IndexSummary {
    let document_count = index.metadata.document_count;
    let average_tokens_per_document = if document_count == 0 {
        0.0
    } else {
        index.metadata.total_tokens as f64 / document_count as f64
    };

    IndexSummary {
        root: index.metadata.root.clone(),
        document_count,
        term_count: index.metadata.term_count,
        total_tokens: index.metadata.total_tokens,
        average_tokens_per_document,
        largest_documents: largest_documents(index, limit),
        top_terms: top_terms(index, limit),
    }
}

pub fn top_terms(index: &InvertedIndex, limit: usize) -> Vec<TermStat> {
    let mut stats = index
        .postings
        .iter()
        .map(|(term, postings)| TermStat {
            term: term.clone(),
            total_frequency: postings.iter().map(|posting| posting.frequency).sum(),
            document_frequency: postings.len(),
        })
        .collect::<Vec<_>>();

    stats.sort_by(compare_terms);
    truncate_to(&mut stats, limit);
    stats
}

pub fn inspect_document(
    index: &InvertedIndex,
    doc_id: usize,
    term_limit: usize,
) -> AppResult<DocumentInspection> {
    let document = index
        .document(doc_id)
        .ok_or_else(|| AppError::MissingPath(PathBuf::from(format!("document id {doc_id}"))))?;

    Ok(DocumentInspection {
        document: document_stat(document),
        top_terms: document_top_terms(index, doc_id, term_limit),
        preview: preview_content(&document.content, 300),
    })
}

pub fn format_summary(summary: &IndexSummary) -> String {
    let mut output = String::new();
    output.push_str(&format!("Root: {}\n", summary.root.display()));
    output.push_str(&format!("Documents: {}\n", summary.document_count));
    output.push_str(&format!("Terms: {}\n", summary.term_count));
    output.push_str(&format!("Total tokens: {}\n", summary.total_tokens));
    output.push_str(&format!(
        "Average tokens/document: {:.2}\n",
        summary.average_tokens_per_document
    ));

    output.push_str("\nLargest documents:\n");
    if summary.largest_documents.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for document in &summary.largest_documents {
            output.push_str(&format!(
                "  #{:<3} {:<28} {:>5} tokens  {}\n",
                document.doc_id,
                truncate_display(&document.title, 28),
                document.token_count,
                document.path.display()
            ));
        }
    }

    output.push_str("\nTop terms:\n");
    output.push_str(&format_terms(&summary.top_terms));
    output
}

pub fn format_terms(terms: &[TermStat]) -> String {
    if terms.is_empty() {
        return "  (none)\n".to_string();
    }

    let mut output = String::new();
    for (rank, term) in terms.iter().enumerate() {
        output.push_str(&format!(
            "  {:>2}. {:<18} freq={:<4} docs={}\n",
            rank + 1,
            truncate_display(&term.term, 18),
            term.total_frequency,
            term.document_frequency
        ));
    }
    output
}

pub fn format_inspection(inspection: &DocumentInspection) -> String {
    let mut output = String::new();
    output.push_str(&format!("Document #{}\n", inspection.document.doc_id));
    output.push_str(&format!("Title: {}\n", inspection.document.title));
    output.push_str(&format!("Path: {}\n", inspection.document.path.display()));
    output.push_str(&format!("Tokens: {}\n", inspection.document.token_count));
    output.push_str(&format!("Size: {} bytes\n", inspection.document.size_bytes));
    output.push_str("\nTop terms in document:\n");
    output.push_str(&format_terms(&inspection.top_terms));
    output.push_str("\nPreview:\n");
    output.push_str(&inspection.preview);
    output.push('\n');
    output
}

fn largest_documents(index: &InvertedIndex, limit: usize) -> Vec<DocumentStat> {
    let mut documents = index
        .documents
        .iter()
        .map(document_stat)
        .collect::<Vec<_>>();

    documents.sort_by(|left, right| {
        right
            .token_count
            .cmp(&left.token_count)
            .then_with(|| left.title.cmp(&right.title))
    });
    truncate_to(&mut documents, limit);
    documents
}

fn document_stat(document: &Document) -> DocumentStat {
    DocumentStat {
        doc_id: document.id,
        title: document.title.clone(),
        path: document.path.clone(),
        token_count: document.token_count,
        size_bytes: document.size_bytes,
    }
}

fn document_top_terms(index: &InvertedIndex, doc_id: usize, limit: usize) -> Vec<TermStat> {
    let mut term_map: HashMap<String, TermStat> = HashMap::new();

    for (term, postings) in &index.postings {
        if let Some(posting) = postings.iter().find(|posting| posting.doc_id == doc_id) {
            term_map.insert(
                term.clone(),
                TermStat {
                    term: term.clone(),
                    total_frequency: posting.frequency,
                    document_frequency: 1,
                },
            );
        }
    }

    let mut terms = term_map.into_values().collect::<Vec<_>>();
    terms.sort_by(compare_terms);
    truncate_to(&mut terms, limit);
    terms
}

fn compare_terms(left: &TermStat, right: &TermStat) -> Ordering {
    right
        .total_frequency
        .cmp(&left.total_frequency)
        .then_with(|| right.document_frequency.cmp(&left.document_frequency))
        .then_with(|| left.term.cmp(&right.term))
}

fn preview_content(content: &str, max_chars: usize) -> String {
    let trimmed = content.trim().replace('\n', " ");
    let chars = trimmed.chars().collect::<Vec<_>>();
    if chars.len() <= max_chars {
        return trimmed;
    }

    let mut preview = chars[..max_chars].iter().collect::<String>();
    preview.push_str("...");
    preview
}

fn truncate_display(text: &str, max_chars: usize) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    if chars.len() <= max_chars {
        return text.to_string();
    }

    let keep = max_chars.saturating_sub(3);
    let mut truncated = chars[..keep].iter().collect::<String>();
    truncated.push_str("...");
    truncated
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::index::IndexBuilder;
    use crate::parser::SimpleTokenizer;

    use super::*;

    #[test]
    fn summary_contains_top_terms_and_largest_documents() {
        let temp = tempdir().expect("tempdir");
        fs::write(temp.path().join("a.md"), "# A\nRust Rust search").expect("write a");
        fs::write(temp.path().join("b.md"), "# B\nlocal knowledge base").expect("write b");
        let index = IndexBuilder::new(SimpleTokenizer::default())
            .build(temp.path())
            .expect("build");

        let summary = summarize_index(&index, 3);

        assert_eq!(summary.document_count, 2);
        assert!(!summary.top_terms.is_empty());
        assert!(!summary.largest_documents.is_empty());
    }

    #[test]
    fn document_inspection_returns_document_terms() {
        let temp = tempdir().expect("tempdir");
        fs::write(temp.path().join("a.md"), "# A\nRust Rust ownership").expect("write a");
        let index = IndexBuilder::new(SimpleTokenizer::default())
            .build(temp.path())
            .expect("build");

        let inspection = inspect_document(&index, 0, 5).expect("inspect");

        assert_eq!(inspection.document.title, "A");
        assert!(inspection.top_terms.iter().any(|term| term.term == "rust"));
    }
}

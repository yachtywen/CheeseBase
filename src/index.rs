use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rayon::prelude::*;

use crate::error::AppResult;
use crate::model::{
    Document, INDEX_VERSION, IndexMetadata, IndexedOccurrence, InvertedIndex, ParsedDocument,
    Posting,
};
use crate::parser::{Tokenizer, parse_file};
use crate::scanner::scan_directory;

#[derive(Debug, Clone)]
pub struct IndexBuilder<T>
where
    T: Tokenizer,
{
    tokenizer: T,
}

impl<T> IndexBuilder<T>
where
    T: Tokenizer,
{
    pub fn new(tokenizer: T) -> Self {
        Self { tokenizer }
    }

    pub fn build(&self, root: impl AsRef<Path>) -> AppResult<InvertedIndex> {
        let root = root.as_ref();
        let files = scan_directory(root)?;
        let tokenizer = self.tokenizer.clone();

        let mut parsed = files
            .par_iter()
            .map(|candidate| parse_file(&candidate.path, candidate.file_type, &tokenizer))
            .collect::<AppResult<Vec<_>>>()?;

        parsed.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(build_from_parsed(root, parsed))
    }
}

fn build_from_parsed(root: &Path, parsed_docs: Vec<ParsedDocument>) -> InvertedIndex {
    let mut documents = Vec::with_capacity(parsed_docs.len());
    let mut postings: HashMap<String, Vec<Posting>> = HashMap::new();
    let mut total_tokens = 0usize;

    for (doc_id, parsed) in parsed_docs.into_iter().enumerate() {
        total_tokens += parsed.tokens.len();

        let mut per_doc: HashMap<String, Vec<IndexedOccurrence>> = HashMap::new();
        for occurrence in parsed.tokens {
            per_doc
                .entry(occurrence.token)
                .or_default()
                .push(IndexedOccurrence {
                    token_position: occurrence.position,
                    char_start: occurrence.char_start,
                    char_end: occurrence.char_end,
                    page: occurrence.page,
                });
        }

        for (token, occurrences) in per_doc {
            postings.entry(token).or_default().push(Posting {
                doc_id,
                frequency: occurrences.len(),
                occurrences,
            });
        }

        documents.push(Document {
            id: doc_id,
            path: parsed.path,
            title: parsed.title,
            file_type: parsed.file_type,
            modified_secs: parsed.modified_secs,
            size_bytes: parsed.size_bytes,
            token_count: total_tokens_for_content(&parsed.content, doc_id, &postings),
            content: parsed.content,
        });
    }

    let metadata = IndexMetadata {
        version: INDEX_VERSION,
        root: root.to_path_buf(),
        document_count: documents.len(),
        term_count: postings.len(),
        total_tokens,
        created_secs: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default(),
    };

    InvertedIndex {
        metadata,
        documents,
        postings,
    }
}

fn total_tokens_for_content(
    _content: &str,
    doc_id: usize,
    postings: &HashMap<String, Vec<Posting>>,
) -> usize {
    postings
        .values()
        .flat_map(|items| items.iter())
        .filter(|posting| posting.doc_id == doc_id)
        .map(|posting| posting.frequency)
        .sum()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::parser::SimpleTokenizer;

    use super::*;

    #[test]
    fn index_records_frequency_and_occurrences() {
        let temp = tempdir().expect("tempdir");
        fs::write(temp.path().join("one.md"), "# One\nRust Rust ownership").expect("write one");

        let builder = IndexBuilder::new(SimpleTokenizer::default());
        let index = builder.build(temp.path()).expect("build");
        let rust_posting = index
            .postings
            .get("rust")
            .and_then(|items| items.first())
            .expect("rust posting");

        assert_eq!(index.documents.len(), 1);
        assert_eq!(rust_posting.frequency, 2);
        assert_eq!(rust_posting.doc_id, 0);
        assert!(rust_posting.occurrences.len() >= 2);
        assert!(
            rust_posting.occurrences[0].token_position < rust_posting.occurrences[1].token_position
        );
    }

    #[test]
    fn index_builds_multiple_documents() {
        let temp = tempdir().expect("tempdir");
        fs::write(temp.path().join("one.md"), "# One\nRust ownership").expect("write one");
        fs::write(temp.path().join("two.txt"), "local search index").expect("write two");

        let builder = IndexBuilder::new(SimpleTokenizer::default());
        let index = builder.build(temp.path()).expect("build");

        assert_eq!(index.metadata.document_count, 2);
        assert!(index.postings.contains_key("rust"));
        assert!(index.postings.contains_key("search"));
    }
}

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::error::{AppError, AppResult};
use crate::model::{Document, InvertedIndex, SearchMatch, SearchMode, SearchResult};
use crate::parser::Tokenizer;

const BM25_K1: f64 = 1.5;
const BM25_B: f64 = 0.75;

pub struct SearchEngine<'a, T>
where
    T: Tokenizer,
{
    index: &'a InvertedIndex,
    tokenizer: T,
}

impl<'a, T> SearchEngine<'a, T>
where
    T: Tokenizer,
{
    pub fn new(index: &'a InvertedIndex, tokenizer: T) -> Self {
        Self { index, tokenizer }
    }

    pub fn search(&self, query: &str, limit: usize) -> AppResult<Vec<SearchResult>> {
        self.search_with_options(
            query,
            SearchOptions {
                limit,
                mode: SearchMode::Any,
            },
        )
    }

    pub fn search_with_options(
        &self,
        query: &str,
        options: SearchOptions,
    ) -> AppResult<Vec<SearchResult>> {
        if self.index.documents.is_empty() {
            return Err(AppError::EmptyIndex);
        }

        let terms = self.query_terms(query);
        if terms.is_empty() || options.limit == 0 {
            return Ok(Vec::new());
        }

        let mut scores: HashMap<usize, ScoreParts> = HashMap::new();
        let avg_doc_len = self.average_document_length();
        let total_documents = self.index.documents.len() as f64;

        for term in &terms {
            if let Some(postings) = self.index.postings.get(term) {
                let idf = bm25_idf(total_documents, postings.len() as f64);

                for posting in postings {
                    let Some(document) = self.index.document(posting.doc_id) else {
                        continue;
                    };

                    let score = scores.entry(posting.doc_id).or_default();
                    score.bm25 += bm25_term_score(
                        posting.frequency as f64,
                        document.token_count as f64,
                        avg_doc_len,
                        idf,
                    );
                    score.covered_terms.insert(term.clone());
                    score
                        .occurrences
                        .extend(
                            posting
                                .occurrences
                                .iter()
                                .map(|occurrence| MatchedOccurrence {
                                    term: term.clone(),
                                    token_position: occurrence.token_position,
                                    char_start: occurrence.char_start,
                                    char_end: occurrence.char_end,
                                    page: occurrence.page,
                                }),
                        );
                }
            }
        }

        let mut results = scores
            .into_iter()
            .filter(|(_, parts)| {
                options.mode == SearchMode::Any || parts.covered_terms.len() == terms.len()
            })
            .filter_map(|(doc_id, parts)| {
                let document = self.index.document(doc_id)?;
                Some(self.build_result(document, parts, terms.len()))
            })
            .collect::<Vec<_>>();

        results.sort_by(compare_results);
        truncate_to(&mut results, options.limit);
        Ok(results)
    }

    fn average_document_length(&self) -> f64 {
        if self.index.metadata.document_count == 0 {
            return 1.0;
        }

        (self.index.metadata.total_tokens as f64 / self.index.metadata.document_count as f64)
            .max(1.0)
    }

    fn query_terms(&self, query: &str) -> Vec<String> {
        let mut seen = HashSet::new();
        self.tokenizer
            .tokenize(query)
            .into_iter()
            .map(|item| item.token)
            .filter(|token| seen.insert(token.clone()))
            .collect()
    }

    fn build_result(
        &self,
        document: &Document,
        parts: ScoreParts,
        query_term_count: usize,
    ) -> SearchResult {
        let coverage_bonus = parts.covered_terms.len() as f64 / query_term_count.max(1) as f64;
        let score = parts.bm25 + coverage_bonus * 0.01;
        let mut matched_terms = parts.covered_terms.into_iter().collect::<Vec<_>>();
        matched_terms.sort();
        let matches = build_search_matches(document, &matched_terms, &parts.occurrences);
        let snippet = matches
            .first()
            .map(|item| item.snippet.clone())
            .unwrap_or_else(|| make_snippet(&document.content, &matched_terms, 120));

        SearchResult {
            doc_id: document.id,
            title: document.title.clone(),
            path: document.path.clone(),
            score,
            matched_terms,
            snippet,
            matches,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SearchOptions {
    pub limit: usize,
    pub mode: SearchMode,
}

#[derive(Debug, Default)]
struct ScoreParts {
    bm25: f64,
    covered_terms: HashSet<String>,
    occurrences: Vec<MatchedOccurrence>,
}

#[derive(Debug, Clone)]
struct MatchedOccurrence {
    term: String,
    token_position: usize,
    char_start: usize,
    char_end: usize,
    page: Option<u32>,
}

pub fn make_snippet(content: &str, terms: &[String], max_chars: usize) -> String {
    let content_lower = content.to_lowercase();
    let needle = terms
        .iter()
        .find(|term| content_lower.contains(term.as_str()))
        .cloned();

    let chars = content.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return String::new();
    }

    let start = needle
        .and_then(|term| find_char_index_case_insensitive(content, &term))
        .map(|idx| idx.saturating_sub(max_chars / 3))
        .unwrap_or(0);
    let end = (start + max_chars).min(chars.len());
    let mut snippet = chars[start..end].iter().collect::<String>();
    snippet = snippet.replace('\n', " ");

    for term in terms {
        snippet = highlight_term(&snippet, term);
    }

    let prefix = if start > 0 { "..." } else { "" };
    let suffix = if end < chars.len() { "..." } else { "" };
    format!("{prefix}{}{suffix}", snippet.trim())
}

fn build_search_matches(
    document: &Document,
    matched_terms: &[String],
    occurrences: &[MatchedOccurrence],
) -> Vec<SearchMatch> {
    let mut matches = Vec::new();
    let mut seen_ranges = HashSet::new();

    for term in matched_terms {
        let term_occurrences = occurrences
            .iter()
            .filter(|occurrence| occurrence.term == *term)
            .collect::<Vec<_>>();

        for (range_index, (start, end)) in find_all_term_ranges(&document.content, term)
            .into_iter()
            .enumerate()
        {
            if !seen_ranges.insert((start, end, term.clone())) {
                continue;
            }

            let indexed = term_occurrences
                .get(range_index)
                .or_else(|| term_occurrences.first());

            matches.push(SearchMatch {
                snippet: make_snippet_around_range(
                    &document.content,
                    start,
                    end,
                    matched_terms,
                    140,
                ),
                matched_terms: vec![term.clone()],
                token_position: indexed
                    .map(|item| item.token_position)
                    .unwrap_or(range_index),
                char_start: start,
                char_end: end,
                page: indexed.and_then(|item| item.page),
            });
        }
    }

    matches.sort_by(|left, right| {
        left.page
            .cmp(&right.page)
            .then_with(|| left.char_start.cmp(&right.char_start))
            .then_with(|| left.token_position.cmp(&right.token_position))
    });

    if matches.is_empty() && !matched_terms.is_empty() {
        let fallback = make_snippet(&document.content, matched_terms, 140);
        let first_occurrence = occurrences.iter().min_by_key(|item| item.token_position);
        if !fallback.is_empty() || first_occurrence.is_some() {
            matches.push(SearchMatch {
                snippet: if fallback.is_empty() {
                    format!("Matched in title or file name: {}", document.title)
                } else {
                    fallback
                },
                matched_terms: matched_terms.to_vec(),
                token_position: first_occurrence
                    .map(|item| item.token_position)
                    .unwrap_or_default(),
                char_start: first_occurrence
                    .map(|item| item.char_start)
                    .unwrap_or_default(),
                char_end: first_occurrence
                    .map(|item| item.char_end)
                    .unwrap_or_default(),
                page: first_occurrence.and_then(|item| item.page),
            });
        }
    }

    matches
}

fn find_all_term_ranges(content: &str, term: &str) -> Vec<(usize, usize)> {
    if content.is_empty() || term.is_empty() {
        return Vec::new();
    }

    let lower_content = content.to_lowercase();
    let lower_term = term.to_lowercase();
    let mut ranges = Vec::new();
    let mut cursor = 0usize;

    while let Some(relative) = lower_content[cursor..].find(&lower_term) {
        let start = cursor + relative;
        let end = start + lower_term.len();
        if content.is_char_boundary(start) && content.is_char_boundary(end) {
            ranges.push((start, end));
        }
        cursor = end.max(cursor + 1);
    }

    ranges
}

fn make_snippet_around_range(
    content: &str,
    start_byte: usize,
    end_byte: usize,
    terms: &[String],
    max_chars: usize,
) -> String {
    let chars = content.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return String::new();
    }

    let start_char = content[..start_byte].chars().count();
    let end_char = content[..end_byte].chars().count();
    let window_start = start_char.saturating_sub(max_chars / 3);
    let window_end = (end_char + (max_chars * 2 / 3)).min(chars.len());

    let mut snippet = chars[window_start..window_end].iter().collect::<String>();
    snippet = snippet.replace('\n', " ");
    for term in terms {
        snippet = highlight_term(&snippet, term);
    }

    let prefix = if window_start > 0 { "..." } else { "" };
    let suffix = if window_end < chars.len() { "..." } else { "" };
    format!("{prefix}{}{suffix}", snippet.trim())
}

pub fn bm25_idf(total_documents: f64, document_frequency: f64) -> f64 {
    (1.0 + (total_documents - document_frequency + 0.5) / (document_frequency + 0.5)).ln()
}

pub fn bm25_term_score(tf: f64, doc_len: f64, avg_doc_len: f64, idf: f64) -> f64 {
    if tf <= 0.0 {
        return 0.0;
    }

    let normalized_len = doc_len.max(1.0) / avg_doc_len.max(1.0);
    let denominator = tf + BM25_K1 * (1.0 - BM25_B + BM25_B * normalized_len);
    idf * (tf * (BM25_K1 + 1.0)) / denominator
}

pub fn truncate_to<T>(items: &mut Vec<T>, limit: usize) {
    if items.len() > limit {
        items.truncate(limit);
    }
}

fn compare_results(left: &SearchResult, right: &SearchResult) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.title.cmp(&right.title))
}

fn find_char_index_case_insensitive(content: &str, term: &str) -> Option<usize> {
    let lower = content.to_lowercase();
    let byte_index = lower.find(term)?;
    Some(content[..byte_index].chars().count())
}

pub fn highlight_term(snippet: &str, term: &str) -> String {
    if term.is_empty() {
        return snippet.to_string();
    }

    let lower_snippet = snippet.to_lowercase();
    let lower_term = term.to_lowercase();
    let mut result = String::new();
    let mut cursor = 0usize;

    while let Some(relative) = lower_snippet[cursor..].find(&lower_term) {
        let start = cursor + relative;
        let end = start + lower_term.len();
        result.push_str(&snippet[cursor..start]);
        result.push('[');
        result.push_str(&snippet[start..end]);
        result.push(']');
        cursor = end;
    }
    result.push_str(&snippet[cursor..]);
    result
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::index::IndexBuilder;
    use crate::parser::SimpleTokenizer;

    use super::*;

    #[test]
    fn search_returns_ranked_results() {
        let temp = tempdir().expect("tempdir");
        fs::write(
            temp.path().join("a.md"),
            "# A\nRust ownership ownership borrowing",
        )
        .expect("write a");
        fs::write(temp.path().join("b.md"), "# B\nRust only").expect("write b");

        let builder = IndexBuilder::new(SimpleTokenizer::default());
        let index = builder.build(temp.path()).expect("build");
        let engine = SearchEngine::new(&index, SimpleTokenizer::default());

        let results = engine.search("ownership", 10).expect("search");

        assert_eq!(results[0].title, "A");
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn search_result_keeps_multiple_matches_in_one_document() {
        let temp = tempdir().expect("tempdir");
        fs::write(
            temp.path().join("multi.md"),
            "# Multi\nOwnership starts here.\nA later paragraph mentions ownership again.",
        )
        .expect("write multi");

        let builder = IndexBuilder::new(SimpleTokenizer::default());
        let index = builder.build(temp.path()).expect("build");
        let engine = SearchEngine::new(&index, SimpleTokenizer::default());

        let results = engine.search("ownership", 10).expect("search");

        assert_eq!(results[0].title, "Multi");
        assert!(results[0].matches.len() >= 2);
        assert!(
            results[0].matches[0].char_start < results[0].matches[1].char_start,
            "matches should preserve distinct in-document hit locations"
        );
    }

    #[test]
    fn search_result_keeps_more_than_eight_matches_in_one_document() {
        let temp = tempdir().expect("tempdir");
        let repeated = (0..12)
            .map(|idx| format!("Paragraph {idx} mentions ownership."))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(temp.path().join("many.md"), format!("# Many\n{repeated}")).expect("write many");

        let builder = IndexBuilder::new(SimpleTokenizer::default());
        let index = builder.build(temp.path()).expect("build");
        let engine = SearchEngine::new(&index, SimpleTokenizer::default());

        let results = engine.search("ownership", 10).expect("search");

        assert_eq!(results[0].matches.len(), 12);
    }

    #[test]
    fn snippet_contains_highlighted_ascii_term() {
        let snippet = make_snippet("Rust ownership rules are useful", &["ownership".into()], 80);
        assert!(snippet.contains("[ownership]"));
    }

    #[test]
    fn snippet_contains_highlighted_chinese_term() {
        let snippet = make_snippet("支持所有权模型", &["所有权".into()], 80);
        assert!(snippet.contains("[所有权]"));
    }

    #[test]
    fn bm25_prefers_higher_term_frequency() {
        let low = bm25_term_score(1.0, 100.0, 100.0, 1.0);
        let high = bm25_term_score(3.0, 100.0, 100.0, 1.0);

        assert!(high > low);
    }

    #[test]
    fn bm25_penalizes_longer_documents() {
        let short = bm25_term_score(2.0, 20.0, 100.0, 1.0);
        let long = bm25_term_score(2.0, 300.0, 100.0, 1.0);

        assert!(short > long);
    }

    #[test]
    fn truncate_helper_is_generic() {
        let mut values = vec![1, 2, 3, 4];
        truncate_to(&mut values, 2);
        assert_eq!(values, vec![1, 2]);
    }
}

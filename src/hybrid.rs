use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::model::{InvertedIndex, SearchMatch, SearchMode, SearchResult, SearchStrategy};
use crate::parser::Tokenizer;
use crate::search::{SearchEngine, SearchOptions};
use crate::vector::{VectorSearchHit, vector_search};

const BM25_WEIGHT: f64 = 0.45;
const VECTOR_WEIGHT: f64 = 0.55;
const VECTOR_CANDIDATE_MULTIPLIER: usize = 4;

pub fn search_with_strategy<T>(
    index: &InvertedIndex,
    tokenizer: T,
    query: &str,
    options: SearchOptions,
    strategy: SearchStrategy,
    config: Option<&AppConfig>,
) -> AppResult<Vec<SearchResult>>
where
    T: Tokenizer,
{
    match strategy {
        SearchStrategy::Bm25 => {
            SearchEngine::new(index, tokenizer).search_with_options(query, options)
        }
        SearchStrategy::Hybrid => hybrid_search(
            index,
            tokenizer,
            query,
            options,
            config.ok_or_else(|| AppError::Config("Hybrid 检索缺少配置".to_string()))?,
        ),
    }
}

pub fn hybrid_search<T>(
    index: &InvertedIndex,
    tokenizer: T,
    query: &str,
    options: SearchOptions,
    config: &AppConfig,
) -> AppResult<Vec<SearchResult>>
where
    T: Tokenizer,
{
    let bm25_results = SearchEngine::new(index, tokenizer.clone()).search_with_options(
        query,
        SearchOptions {
            limit: options.limit * VECTOR_CANDIDATE_MULTIPLIER,
            mode: options.mode,
        },
    )?;
    let vector_hits = vector_search(query, options.limit * VECTOR_CANDIDATE_MULTIPLIER, config)?;
    Ok(merge_results(
        index,
        tokenizer,
        query,
        bm25_results,
        vector_hits,
        options,
        config.hybrid_score_threshold,
    ))
}

pub fn merge_results<T>(
    index: &InvertedIndex,
    tokenizer: T,
    query: &str,
    bm25_results: Vec<SearchResult>,
    vector_hits: Vec<VectorSearchHit>,
    options: SearchOptions,
    min_score: f64,
) -> Vec<SearchResult>
where
    T: Tokenizer,
{
    let max_bm25 = bm25_results
        .iter()
        .map(|result| result.score)
        .fold(0.0_f64, f64::max)
        .max(1.0);

    let query_terms = tokenizer
        .tokenize(query)
        .into_iter()
        .map(|item| item.token)
        .collect::<Vec<_>>();
    let mut merged: HashMap<usize, HybridParts> = HashMap::new();

    for result in bm25_results {
        let entry = merged.entry(result.doc_id).or_default();
        entry.bm25_score = entry.bm25_score.max(result.score);
        entry.result = Some(result);
    }

    for hit in vector_hits {
        let entry = merged.entry(hit.doc_id).or_default();
        entry.vector_score = entry.vector_score.max(hit.score);
        entry
            .vector_matches
            .push(hit.to_search_match(&tokenizer, query));
    }

    let mut results = merged
        .into_iter()
        .filter_map(|(doc_id, parts)| {
            let document = index.document(doc_id)?;
            let mut result = parts.result.unwrap_or_else(|| SearchResult {
                doc_id,
                title: document.title.clone(),
                path: document.path.clone(),
                score: 0.0,
                matched_terms: unique_terms(&query_terms),
                snippet: parts
                    .vector_matches
                    .first()
                    .map(|item| item.snippet.clone())
                    .unwrap_or_default(),
                matches: Vec::new(),
            });

            for item in parts.vector_matches {
                if !result
                    .matches
                    .iter()
                    .any(|existing| same_match(existing, &item))
                {
                    result.matches.push(item);
                }
            }
            if result.snippet.is_empty() {
                result.snippet = result
                    .matches
                    .first()
                    .map(|item| item.snippet.clone())
                    .unwrap_or_default();
            }
            for term in &query_terms {
                if !result.matched_terms.contains(term) {
                    result.matched_terms.push(term.clone());
                }
            }
            result.matched_terms.sort();
            result.matched_terms.dedup();

            let bm25_norm = parts.bm25_score / max_bm25;
            let vector_norm = parts.vector_score.clamp(0.0, 1.0);
            result.score = BM25_WEIGHT * bm25_norm + VECTOR_WEIGHT * vector_norm;
            Some((result, parts.bm25_score, parts.vector_score))
        })
        .collect::<Vec<_>>();

    results.retain(|(result, _, _)| result.score >= min_score);

    if options.mode == SearchMode::All {
        let required = unique_terms(&query_terms);
        results.retain(|(result, _, _)| {
            required
                .iter()
                .all(|term| result.matched_terms.iter().any(|matched| matched == term))
        });
    }

    results.sort_by(compare_hybrid);
    results.truncate(options.limit);
    results.into_iter().map(|(result, _, _)| result).collect()
}

#[derive(Debug, Default)]
struct HybridParts {
    bm25_score: f64,
    vector_score: f64,
    result: Option<SearchResult>,
    vector_matches: Vec<SearchMatch>,
}

fn compare_hybrid(left: &(SearchResult, f64, f64), right: &(SearchResult, f64, f64)) -> Ordering {
    right
        .0
        .score
        .partial_cmp(&left.0.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| right.2.partial_cmp(&left.2).unwrap_or(Ordering::Equal))
        .then_with(|| right.1.partial_cmp(&left.1).unwrap_or(Ordering::Equal))
        .then_with(|| left.0.doc_id.cmp(&right.0.doc_id))
}

fn same_match(left: &SearchMatch, right: &SearchMatch) -> bool {
    left.char_start == right.char_start
        && left.char_end == right.char_end
        && left.page == right.page
}

fn unique_terms(terms: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    terms
        .iter()
        .filter(|term| seen.insert((*term).clone()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::index::IndexBuilder;
    use crate::parser::SimpleTokenizer;
    use crate::vector::VectorSearchHit;

    use super::*;

    #[test]
    fn hybrid_keeps_vector_only_documents() {
        let temp = tempdir().expect("tempdir");
        fs::write(temp.path().join("note.md"), "# Note\nsemantic database").expect("write");
        let index = IndexBuilder::new(SimpleTokenizer::default())
            .build(temp.path())
            .expect("build");
        let document = index.document(0).expect("document");
        let hits = vec![VectorSearchHit {
            doc_id: 0,
            score: 0.9,
            path: document.path.clone(),
            title: document.title.clone(),
            text: "semantic database".to_string(),
            start_char: 0,
            end_char: 17,
            page: None,
        }];

        let results = merge_results(
            &index,
            SimpleTokenizer::default(),
            "meaning",
            Vec::new(),
            hits,
            SearchOptions {
                limit: 10,
                mode: SearchMode::Any,
            },
            0.45,
        );

        assert_eq!(results.len(), 1);
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn hybrid_score_prefers_strong_vector_score() {
        let temp = tempdir().expect("tempdir");
        fs::write(temp.path().join("a.md"), "# A\nrust").expect("write a");
        fs::write(temp.path().join("b.md"), "# B\nrust").expect("write b");
        let index = IndexBuilder::new(SimpleTokenizer::default())
            .build(temp.path())
            .expect("build");
        let bm25 = SearchEngine::new(&index, SimpleTokenizer::default())
            .search("rust", 10)
            .expect("search");
        let hit = VectorSearchHit {
            doc_id: 1,
            score: 1.0,
            path: index.document(1).expect("doc").path.clone(),
            title: "B".to_string(),
            text: "rust".to_string(),
            start_char: 0,
            end_char: 4,
            page: None,
        };

        let results = merge_results(
            &index,
            SimpleTokenizer::default(),
            "rust",
            bm25,
            vec![hit],
            SearchOptions {
                limit: 10,
                mode: SearchMode::Any,
            },
            0.45,
        );

        assert_eq!(results[0].doc_id, 1);
    }

    #[test]
    fn hybrid_filters_results_below_threshold() {
        let temp = tempdir().expect("tempdir");
        fs::write(temp.path().join("note.md"), "# Note\nsemantic database").expect("write");
        let index = IndexBuilder::new(SimpleTokenizer::default())
            .build(temp.path())
            .expect("build");
        let document = index.document(0).expect("document");
        let hits = vec![VectorSearchHit {
            doc_id: 0,
            score: 0.50,
            path: document.path.clone(),
            title: document.title.clone(),
            text: "semantic database".to_string(),
            start_char: 0,
            end_char: 17,
            page: None,
        }];

        let results = merge_results(
            &index,
            SimpleTokenizer::default(),
            "meaning",
            Vec::new(),
            hits,
            SearchOptions {
                limit: 10,
                mode: SearchMode::Any,
            },
            0.45,
        );

        assert!(results.is_empty());
    }
}

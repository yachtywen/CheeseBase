use std::path::PathBuf;

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::config::AppConfig;
use crate::embedding::DashScopeEmbeddingClient;
use crate::error::{AppError, AppResult};
use crate::model::{InvertedIndex, SearchMatch};
use crate::parser::Tokenizer;

const CHUNK_SIZE: usize = 700;
const CHUNK_OVERLAP: usize = 100;
const UPSERT_BATCH_SIZE: usize = 64;

#[derive(Debug, Clone)]
pub struct VectorIndexStats {
    pub collection: String,
    pub chunk_count: usize,
}

#[derive(Debug, Clone)]
pub struct DocumentChunk {
    pub doc_id: usize,
    pub chunk_id: usize,
    pub path: PathBuf,
    pub title: String,
    pub file_type: String,
    pub text: String,
    pub embedding_text: String,
    pub start_char: usize,
    pub end_char: usize,
    pub page: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct VectorSearchHit {
    pub doc_id: usize,
    pub score: f64,
    pub path: PathBuf,
    pub title: String,
    pub text: String,
    pub start_char: usize,
    pub end_char: usize,
    pub page: Option<u32>,
}

impl VectorSearchHit {
    pub fn to_search_match<T>(&self, tokenizer: &T, query: &str) -> SearchMatch
    where
        T: Tokenizer,
    {
        let matched_terms = tokenizer
            .tokenize(query)
            .into_iter()
            .map(|item| item.token)
            .collect::<Vec<_>>();
        SearchMatch {
            snippet: self.text.replace('\n', " "),
            matched_terms,
            token_position: 0,
            char_start: self.start_char,
            char_end: self.end_char,
            page: self.page,
        }
    }
}

#[derive(Debug, Clone)]
pub struct QdrantClient {
    url: String,
    collection: String,
    http: Client,
}

impl QdrantClient {
    pub fn new(url: String, collection: String) -> Self {
        Self {
            url,
            collection,
            http: Client::new(),
        }
    }

    pub fn recreate_collection(&self, dimensions: usize) -> AppResult<()> {
        let delete_response = self
            .http
            .delete(format!("{}/collections/{}", self.url, self.collection))
            .send()?;
        if !delete_response.status().is_success()
            && delete_response.status() != reqwest::StatusCode::NOT_FOUND
        {
            let status = delete_response.status();
            let body = delete_response.text().unwrap_or_default();
            return Err(AppError::Qdrant(format!(
                "delete collection failed: {status} {body}"
            )));
        }

        let response = self
            .http
            .put(format!("{}/collections/{}", self.url, self.collection))
            .json(&json!({
                "vectors": {
                    "size": dimensions,
                    "distance": "Cosine"
                }
            }))
            .send()?;
        ensure_qdrant_success(response, "create collection")
    }

    pub fn upsert_chunks(&self, chunks: &[DocumentChunk], vectors: &[Vec<f32>]) -> AppResult<()> {
        if chunks.len() != vectors.len() {
            return Err(AppError::Qdrant(format!(
                "chunk/vector count mismatch: {} chunks, {} vectors",
                chunks.len(),
                vectors.len()
            )));
        }

        for (chunk_batch, vector_batch) in chunks
            .chunks(UPSERT_BATCH_SIZE)
            .zip(vectors.chunks(UPSERT_BATCH_SIZE))
        {
            let points = chunk_batch
                .iter()
                .zip(vector_batch.iter())
                .map(|(chunk, vector)| QdrantPoint {
                    id: point_id(chunk.doc_id, chunk.chunk_id),
                    vector,
                    payload: json!({
                        "doc_id": chunk.doc_id,
                        "chunk_id": chunk.chunk_id,
                        "path": chunk.path.display().to_string(),
                        "title": chunk.title,
                        "file_type": chunk.file_type,
                        "text": chunk.text,
                        "start_char": chunk.start_char,
                        "end_char": chunk.end_char,
                        "page": chunk.page,
                    }),
                })
                .collect::<Vec<_>>();

            let response = self
                .http
                .put(format!(
                    "{}/collections/{}/points?wait=true",
                    self.url, self.collection
                ))
                .json(&json!({ "points": points }))
                .send()?;
            ensure_qdrant_success(response, "upsert points")?;
        }

        Ok(())
    }

    pub fn search(&self, vector: &[f32], limit: usize) -> AppResult<Vec<VectorSearchHit>> {
        let response = self
            .http
            .post(format!(
                "{}/collections/{}/points/search",
                self.url, self.collection
            ))
            .json(&json!({
                "vector": vector,
                "limit": limit,
                "with_payload": true
            }))
            .send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(AppError::Qdrant(format!(
                "search points failed: {status} {body}"
            )));
        }

        let response: QdrantSearchResponse = response.json()?;
        response
            .result
            .into_iter()
            .map(VectorSearchHit::try_from)
            .collect()
    }
}

pub fn build_vector_index(
    index: &InvertedIndex,
    config: &AppConfig,
) -> AppResult<VectorIndexStats> {
    let chunks = build_chunks(index);
    let texts = chunks
        .iter()
        .map(|chunk| chunk.embedding_text.clone())
        .collect::<Vec<_>>();
    let embedding = DashScopeEmbeddingClient::new(config.embedding.clone());
    let vectors = embedding.embed_batch(&texts)?;
    let qdrant = QdrantClient::new(config.qdrant.url.clone(), config.qdrant.collection.clone());
    qdrant.recreate_collection(config.embedding.dimensions)?;
    qdrant.upsert_chunks(&chunks, &vectors)?;
    Ok(VectorIndexStats {
        collection: config.qdrant.collection.clone(),
        chunk_count: chunks.len(),
    })
}

pub fn vector_search(
    query: &str,
    limit: usize,
    config: &AppConfig,
) -> AppResult<Vec<VectorSearchHit>> {
    let embedding = DashScopeEmbeddingClient::new(config.embedding.clone());
    let query_vector = embedding.embed_query(query)?;
    let qdrant = QdrantClient::new(config.qdrant.url.clone(), config.qdrant.collection.clone());
    qdrant.search(&query_vector, limit)
}

pub fn build_chunks(index: &InvertedIndex) -> Vec<DocumentChunk> {
    let mut chunks = Vec::new();
    for document in &index.documents {
        for (chunk_id, (text, start_char, end_char)) in split_text_into_chunks(&document.content)
            .into_iter()
            .enumerate()
        {
            let page = page_for_range(index, document.id, start_char, end_char);
            chunks.push(DocumentChunk {
                doc_id: document.id,
                chunk_id,
                path: document.path.clone(),
                title: document.title.clone(),
                file_type: format!("{:?}", document.file_type),
                embedding_text: format!(
                    "{}\n{}\n{}",
                    document.title,
                    document.path.display(),
                    text
                ),
                text,
                start_char,
                end_char,
                page,
            });
        }
    }
    chunks
}

pub fn split_text_into_chunks(text: &str) -> Vec<(String, usize, usize)> {
    let chars = text.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut start = 0usize;
    while start < chars.len() {
        let end = (start + CHUNK_SIZE).min(chars.len());
        let chunk = chars[start..end].iter().collect::<String>();
        chunks.push((chunk, start, end));
        if end == chars.len() {
            break;
        }
        start = end.saturating_sub(CHUNK_OVERLAP).max(start + 1);
    }
    chunks
}

fn page_for_range(
    index: &InvertedIndex,
    doc_id: usize,
    start_char: usize,
    end_char: usize,
) -> Option<u32> {
    index
        .postings
        .values()
        .flat_map(|items| items.iter())
        .filter(|posting| posting.doc_id == doc_id)
        .flat_map(|posting| posting.occurrences.iter())
        .find(|occurrence| {
            occurrence.page.is_some()
                && occurrence.char_start >= start_char
                && occurrence.char_start <= end_char
        })
        .and_then(|occurrence| occurrence.page)
}

fn point_id(doc_id: usize, chunk_id: usize) -> u64 {
    (doc_id as u64) * 1_000_000 + chunk_id as u64
}

fn ensure_qdrant_success(response: reqwest::blocking::Response, action: &str) -> AppResult<()> {
    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        Err(AppError::Qdrant(format!(
            "{action} failed: {status} {body}"
        )))
    }
}

#[derive(Debug, Serialize)]
struct QdrantPoint<'a> {
    id: u64,
    vector: &'a [f32],
    payload: Value,
}

#[derive(Debug, Deserialize)]
struct QdrantSearchResponse {
    result: Vec<QdrantScoredPoint>,
}

#[derive(Debug, Deserialize)]
struct QdrantScoredPoint {
    score: f64,
    payload: Value,
}

impl TryFrom<QdrantScoredPoint> for VectorSearchHit {
    type Error = AppError;

    fn try_from(value: QdrantScoredPoint) -> Result<Self, Self::Error> {
        let payload = value.payload;
        Ok(Self {
            doc_id: payload_usize(&payload, "doc_id")?,
            score: value.score,
            path: PathBuf::from(payload_string(&payload, "path")?),
            title: payload_string(&payload, "title")?,
            text: payload_string(&payload, "text")?,
            start_char: payload_usize(&payload, "start_char")?,
            end_char: payload_usize(&payload, "end_char")?,
            page: payload
                .get("page")
                .and_then(|value| value.as_u64())
                .map(|value| value as u32),
        })
    }
}

fn payload_string(payload: &Value, key: &str) -> AppResult<String> {
    payload
        .get(key)
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .ok_or_else(|| AppError::Qdrant(format!("missing payload string field: {key}")))
}

fn payload_usize(payload: &Value, key: &str) -> AppResult<usize> {
    payload
        .get(key)
        .and_then(|value| value.as_u64())
        .map(|value| value as usize)
        .ok_or_else(|| AppError::Qdrant(format!("missing payload numeric field: {key}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunker_keeps_short_text_as_one_chunk() {
        let chunks = split_text_into_chunks("hello 世界");

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].0, "hello 世界");
    }

    #[test]
    fn chunker_handles_chinese_boundaries() {
        let text = "知识库".repeat(300);
        let chunks = split_text_into_chunks(&text);

        assert!(chunks.len() > 1);
        for (chunk, _, _) in chunks {
            assert!(chunk.is_char_boundary(chunk.len()));
        }
    }

    #[test]
    fn point_ids_are_stable() {
        assert_eq!(point_id(7, 3), 7_000_003);
    }
}

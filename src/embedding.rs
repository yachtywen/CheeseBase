use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::config::EmbeddingConfig;
use crate::error::{AppError, AppResult};

const EMBEDDING_BATCH_SIZE: usize = 10;

#[derive(Debug, Clone)]
pub struct DashScopeEmbeddingClient {
    config: EmbeddingConfig,
    http: Client,
}

impl DashScopeEmbeddingClient {
    pub fn new(config: EmbeddingConfig) -> Self {
        Self {
            config,
            http: Client::new(),
        }
    }

    pub fn embed_query(&self, text: &str) -> AppResult<Vec<f32>> {
        let mut vectors = self.embed_batch(&[text.to_string()])?;
        vectors
            .pop()
            .ok_or_else(|| AppError::Embedding("embedding response is empty".to_string()))
    }

    pub fn embed_batch(&self, texts: &[String]) -> AppResult<Vec<Vec<f32>>> {
        let mut vectors = Vec::with_capacity(texts.len());
        for batch in texts.chunks(EMBEDDING_BATCH_SIZE) {
            let response = self.embed_batch_once(batch)?;
            vectors.extend(response);
        }
        Ok(vectors)
    }

    fn embed_batch_once(&self, texts: &[String]) -> AppResult<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let request = EmbeddingRequest {
            model: self.config.model_name.clone(),
            input: texts.to_vec(),
            dimensions: self.config.dimensions,
            encoding_format: "float".to_string(),
        };

        let response = self
            .http
            .post(format!("{}/embeddings", self.config.base_url))
            .bearer_auth(&self.config.api_key)
            .json(&request)
            .send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(AppError::Embedding(format!(
                "DashScope embedding request failed: {status} {body}"
            )));
        }

        let response: EmbeddingResponse = response.json()?;
        let mut data = response.data;
        data.sort_by_key(|item| item.index);

        let vectors = data
            .into_iter()
            .map(|item| {
                if item.embedding.len() != self.config.dimensions {
                    Err(AppError::Embedding(format!(
                        "embedding dimension mismatch: expected {}, got {}",
                        self.config.dimensions,
                        item.embedding.len()
                    )))
                } else {
                    Ok(item.embedding)
                }
            })
            .collect::<AppResult<Vec<_>>>()?;

        if vectors.len() != texts.len() {
            return Err(AppError::Embedding(format!(
                "embedding response count mismatch: expected {}, got {}",
                texts.len(),
                vectors.len()
            )));
        }

        Ok(vectors)
    }
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
    dimensions: usize,
    encoding_format: String,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    index: usize,
    embedding: Vec<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_request_contains_required_fields() {
        let request = EmbeddingRequest {
            model: "text-embedding-v3".to_string(),
            input: vec!["hello".to_string()],
            dimensions: 1024,
            encoding_format: "float".to_string(),
        };
        let value = serde_json::to_value(request).expect("json");

        assert_eq!(value["model"], "text-embedding-v3");
        assert_eq!(value["dimensions"], 1024);
        assert_eq!(value["encoding_format"], "float");
        assert_eq!(value["input"][0], "hello");
    }
}

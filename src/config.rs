use std::env;

use crate::error::{AppError, AppResult};

pub const DEFAULT_EMBED_MODEL_TYPE: &str = "dashscope";
pub const DEFAULT_EMBED_MODEL_NAME: &str = "text-embedding-v3";
pub const DEFAULT_EMBED_BASE_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";
pub const DEFAULT_EMBED_DIMENSIONS: usize = 1024;
pub const DEFAULT_QDRANT_URL: &str = "http://localhost:6333";
pub const DEFAULT_QDRANT_COLLECTION: &str = "cheesebase_chunks";
pub const DEFAULT_HYBRID_SCORE_THRESHOLD: f64 = 0.45;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub embedding: EmbeddingConfig,
    pub qdrant: QdrantConfig,
    pub hybrid_score_threshold: f64,
}

#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub model_type: String,
    pub model_name: String,
    pub api_key: String,
    pub base_url: String,
    pub dimensions: usize,
}

#[derive(Debug, Clone)]
pub struct QdrantConfig {
    pub url: String,
    pub collection: String,
}

impl AppConfig {
    pub fn from_env() -> AppResult<Self> {
        let _ = dotenvy::dotenv();
        Ok(Self {
            embedding: EmbeddingConfig::from_env()?,
            qdrant: QdrantConfig::from_env(),
            hybrid_score_threshold: env_f64_or_default(
                "HYBRID_SCORE_THRESHOLD",
                DEFAULT_HYBRID_SCORE_THRESHOLD,
            )?,
        })
    }
}

impl EmbeddingConfig {
    pub fn from_env() -> AppResult<Self> {
        let model_type = env_or_default("EMBED_MODEL_TYPE", DEFAULT_EMBED_MODEL_TYPE);
        if model_type != "dashscope" {
            return Err(AppError::Config(format!(
                "当前版本仅支持 dashscope embedding，不支持 {model_type}"
            )));
        }

        let api_key = env::var("EMBED_API_KEY")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::Config("缺少 EMBED_API_KEY 环境变量".to_string()))?;

        Ok(Self {
            model_type,
            model_name: env_or_default("EMBED_MODEL_NAME", DEFAULT_EMBED_MODEL_NAME),
            api_key,
            base_url: env_or_default("EMBED_BASE_URL", DEFAULT_EMBED_BASE_URL)
                .trim_end_matches('/')
                .to_string(),
            dimensions: env_usize_or_default("EMBED_DIMENSIONS", DEFAULT_EMBED_DIMENSIONS)?,
        })
    }
}

impl QdrantConfig {
    pub fn from_env() -> Self {
        Self {
            url: env_or_default("QDRANT_URL", DEFAULT_QDRANT_URL)
                .trim_end_matches('/')
                .to_string(),
            collection: env_or_default("QDRANT_COLLECTION", DEFAULT_QDRANT_COLLECTION),
        }
    }
}

fn env_or_default(key: &str, default: &str) -> String {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn env_usize_or_default(key: &str, default: usize) -> AppResult<usize> {
    match env::var(key).ok().map(|value| value.trim().to_string()) {
        Some(value) if !value.is_empty() => value
            .parse::<usize>()
            .map_err(|_| AppError::Config(format!("{key} 必须是正整数"))),
        _ => Ok(default),
    }
}

fn env_f64_or_default(key: &str, default: f64) -> AppResult<f64> {
    match env::var(key).ok().map(|value| value.trim().to_string()) {
        Some(value) if !value.is_empty() => {
            let parsed = value
                .parse::<f64>()
                .map_err(|_| AppError::Config(format!("{key} 必须是 0 到 1 之间的小数")))?;
            if (0.0..=1.0).contains(&parsed) {
                Ok(parsed)
            } else {
                Err(AppError::Config(format!("{key} 必须是 0 到 1 之间的小数")))
            }
        }
        _ => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qdrant_config_uses_defaults() {
        let config = QdrantConfig::from_env();

        assert!(!config.url.is_empty());
        assert!(!config.collection.is_empty());
    }

    #[test]
    fn default_constants_match_plan() {
        assert_eq!(DEFAULT_EMBED_MODEL_NAME, "text-embedding-v3");
        assert_eq!(DEFAULT_EMBED_DIMENSIONS, 1024);
        assert_eq!(DEFAULT_QDRANT_COLLECTION, "cheesebase_chunks");
        assert_eq!(DEFAULT_HYBRID_SCORE_THRESHOLD, 0.45);
    }
}

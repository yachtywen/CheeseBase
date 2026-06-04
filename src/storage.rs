use std::fs;
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::{AppError, AppResult};
use crate::model::{INDEX_VERSION, InvertedIndex};

pub fn save_index(path: impl AsRef<Path>, index: &InvertedIndex) -> AppResult<()> {
    write_json(path, index)
}

pub fn load_index(path: impl AsRef<Path>) -> AppResult<InvertedIndex> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(AppError::MissingPath(path.to_path_buf()));
    }

    let index: InvertedIndex = read_json(path)?;
    if index.metadata.version != INDEX_VERSION {
        return Err(AppError::IncompatibleIndex {
            expected: INDEX_VERSION,
            found: index.metadata.version,
        });
    }
    Ok(index)
}

pub fn write_json<T>(path: impl AsRef<Path>, value: &T) -> AppResult<()>
where
    T: Serialize,
{
    let text = serde_json::to_string_pretty(value)?;
    fs::write(path, text)?;
    Ok(())
}

pub fn read_json<T>(path: impl AsRef<Path>) -> AppResult<T>
where
    T: DeserializeOwned,
{
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::index::IndexBuilder;
    use crate::parser::SimpleTokenizer;

    use super::*;

    #[test]
    fn saved_index_can_be_loaded() {
        let temp = tempdir().expect("tempdir");
        fs::write(temp.path().join("note.md"), "# Note\nRust search").expect("write note");
        let builder = IndexBuilder::new(SimpleTokenizer::default());
        let index = builder.build(temp.path()).expect("build");
        let index_path = temp.path().join("index.json");

        save_index(&index_path, &index).expect("save");
        let loaded = load_index(&index_path).expect("load");

        assert_eq!(loaded.metadata.document_count, 1);
        assert!(loaded.postings.contains_key("rust"));
    }

    #[test]
    fn damaged_json_returns_error() {
        let temp = tempdir().expect("tempdir");
        let index_path = temp.path().join("broken.json");
        fs::write(&index_path, "{broken").expect("write broken");

        let err = load_index(&index_path).expect_err("should fail");
        assert!(matches!(err, AppError::Json(_)));
    }
}

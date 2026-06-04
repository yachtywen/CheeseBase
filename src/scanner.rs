use std::path::{Path, PathBuf};

use walkdir::{DirEntry, WalkDir};

use crate::error::{AppError, AppResult};
use crate::model::SupportedFileType;

#[derive(Debug, Clone)]
pub struct FileCandidate {
    pub path: PathBuf,
    pub file_type: SupportedFileType,
}

pub fn scan_directory(root: impl AsRef<Path>) -> AppResult<Vec<FileCandidate>> {
    let root = root.as_ref();
    if !root.exists() {
        return Err(AppError::MissingPath(root.to_path_buf()));
    }
    if !root.is_dir() {
        return Err(AppError::NotDirectory(root.to_path_buf()));
    }

    let mut candidates = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_entry(should_enter) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        if let Some(file_type) = SupportedFileType::from_path(entry.path()) {
            candidates.push(FileCandidate {
                path: entry.path().to_path_buf(),
                file_type,
            });
        }
    }

    candidates.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(candidates)
}

fn should_enter(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return true;
    }

    if !entry.file_type().is_dir() {
        return true;
    }

    let name = entry.file_name().to_string_lossy();
    if name == ".git" || name == "target" {
        return false;
    }

    !name.starts_with('.')
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn supported_extensions_are_detected() {
        assert_eq!(
            SupportedFileType::from_path("note.md"),
            Some(SupportedFileType::Markdown)
        );
        assert_eq!(
            SupportedFileType::from_path("main.rs"),
            Some(SupportedFileType::Rust)
        );
        assert_eq!(
            SupportedFileType::from_path("paper.pdf"),
            Some(SupportedFileType::Pdf)
        );
        assert_eq!(SupportedFileType::from_path("image.png"), None);
    }

    #[test]
    fn scan_skips_hidden_and_target_directories() {
        let temp = tempdir().expect("tempdir");
        fs::write(temp.path().join("visible.md"), "# visible").expect("write visible");
        fs::create_dir(temp.path().join(".git")).expect("mkdir git");
        fs::write(temp.path().join(".git").join("hidden.md"), "# hidden").expect("write hidden");
        fs::create_dir(temp.path().join("target")).expect("mkdir target");
        fs::write(temp.path().join("target").join("build.rs"), "fn main() {}")
            .expect("write target");

        let files = scan_directory(temp.path()).expect("scan");

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("visible.md"));
    }
}

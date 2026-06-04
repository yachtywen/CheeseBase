use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use jieba_rs::Jieba;
use lopdf::Document as PdfDocument;

use crate::error::AppResult;
use crate::model::{ParsedDocument, SupportedFileType, TokenOccurrence};

pub trait Tokenizer: Clone + Send + Sync + 'static {
    fn tokenize(&self, text: &str) -> Vec<TokenOccurrence>;
}

#[derive(Clone)]
pub struct SimpleTokenizer {
    min_ascii_len: usize,
    jieba: Arc<Jieba>,
}

impl SimpleTokenizer {
    pub fn new(min_ascii_len: usize) -> Self {
        Self {
            min_ascii_len,
            jieba: Arc::new(Jieba::new()),
        }
    }
}

impl Default for SimpleTokenizer {
    fn default() -> Self {
        Self::new(2)
    }
}

impl std::fmt::Debug for SimpleTokenizer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SimpleTokenizer")
            .field("min_ascii_len", &self.min_ascii_len)
            .field("engine", &"jieba-rs")
            .finish()
    }
}

impl Tokenizer for SimpleTokenizer {
    fn tokenize(&self, text: &str) -> Vec<TokenOccurrence> {
        self.tokenize_internal(text, None)
    }
}

impl SimpleTokenizer {
    pub fn tokenize_with_page(&self, text: &str, page: Option<u32>) -> Vec<TokenOccurrence> {
        self.tokenize_internal(text, page)
    }

    fn tokenize_internal(&self, text: &str, page: Option<u32>) -> Vec<TokenOccurrence> {
        let min_ascii_len = self.min_ascii_len.max(1);
        let mut occurrences = Vec::new();
        let mut ascii_buf = String::new();
        let mut ascii_start = 0usize;
        let mut chinese_run = String::new();
        let mut chinese_start = 0usize;
        let mut position = 0usize;

        for (byte_index, ch) in text.char_indices() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.flush_chinese(
                    &mut chinese_run,
                    chinese_start,
                    page,
                    &mut position,
                    &mut occurrences,
                );
                if ascii_buf.is_empty() {
                    ascii_start = byte_index;
                }
                ascii_buf.push(ch.to_ascii_lowercase());
            } else if is_cjk(ch) {
                flush_ascii(
                    &mut ascii_buf,
                    ascii_start,
                    page,
                    min_ascii_len,
                    &mut position,
                    &mut occurrences,
                );
                if chinese_run.is_empty() {
                    chinese_start = byte_index;
                }
                chinese_run.push(ch);
            } else {
                flush_ascii(
                    &mut ascii_buf,
                    ascii_start,
                    page,
                    min_ascii_len,
                    &mut position,
                    &mut occurrences,
                );
                self.flush_chinese(
                    &mut chinese_run,
                    chinese_start,
                    page,
                    &mut position,
                    &mut occurrences,
                );
            }
        }

        flush_ascii(
            &mut ascii_buf,
            ascii_start,
            page,
            min_ascii_len,
            &mut position,
            &mut occurrences,
        );
        self.flush_chinese(
            &mut chinese_run,
            chinese_start,
            page,
            &mut position,
            &mut occurrences,
        );
        occurrences
    }

    fn flush_chinese(
        &self,
        chinese_run: &mut String,
        run_start: usize,
        page: Option<u32>,
        position: &mut usize,
        occurrences: &mut Vec<TokenOccurrence>,
    ) {
        if chinese_run.is_empty() {
            return;
        }

        for word in self.jieba.cut(chinese_run, false) {
            let token = word.trim();
            if token.is_empty() || token.chars().all(char::is_whitespace) {
                continue;
            }
            if token.chars().count() == 1 && chinese_run.chars().count() > 1 {
                continue;
            }

            occurrences.push(TokenOccurrence {
                token: token.to_string(),
                position: *position,
                char_start: run_start,
                char_end: run_start + chinese_run.len(),
                page,
            });
            *position += 1;
        }

        chinese_run.clear();
    }
}

pub fn parse_file<T>(
    path: impl AsRef<Path>,
    file_type: SupportedFileType,
    tokenizer: &T,
) -> AppResult<ParsedDocument>
where
    T: Tokenizer,
{
    let path = path.as_ref();
    let metadata = fs::metadata(path)?;
    let modified_secs = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let content = read_document_content(path, file_type)?;
    let title = extract_title(&content, path, file_type);
    let search_text = searchable_text(path, &title, &content);
    let tokens = match file_type {
        SupportedFileType::Pdf => parse_pdf_tokens_with_metadata(path, &title, tokenizer)
            .unwrap_or_else(|_| tokenizer.tokenize(&search_text)),
        _ => tokenizer.tokenize(&search_text),
    };

    Ok(ParsedDocument {
        path: path.to_path_buf(),
        title,
        file_type,
        modified_secs,
        size_bytes: metadata.len(),
        content,
        tokens,
    })
}

fn read_document_content(path: &Path, file_type: SupportedFileType) -> AppResult<String> {
    match file_type {
        SupportedFileType::Pdf => pdf_extract::extract_text(path)
            .map(|text| clean_extracted_pdf_text(&text))
            .map_err(|err| crate::error::AppError::Pdf(err.to_string())),
        SupportedFileType::Markdown
        | SupportedFileType::Text
        | SupportedFileType::Rust
        | SupportedFileType::Toml => Ok(fs::read_to_string(path)?),
    }
}

fn parse_pdf_tokens<T>(path: &Path, tokenizer: &T) -> AppResult<Vec<TokenOccurrence>>
where
    T: Tokenizer,
{
    let document =
        PdfDocument::load(path).map_err(|err| crate::error::AppError::Pdf(err.to_string()))?;
    let mut tokens = Vec::new();
    let mut position_offset = 0usize;

    for page_number in document.get_pages().keys().copied() {
        let text = document
            .extract_text(&[page_number])
            .map(|text| clean_extracted_pdf_text(&text))
            .map_err(|err| crate::error::AppError::Pdf(err.to_string()))?;
        let mut page_tokens = tokenizer.tokenize(&text);
        for token in &mut page_tokens {
            token.position += position_offset;
            token.page = Some(page_number);
        }
        position_offset += page_tokens.len();
        tokens.extend(page_tokens);
    }

    Ok(tokens)
}

fn parse_pdf_tokens_with_metadata<T>(
    path: &Path,
    title: &str,
    tokenizer: &T,
) -> AppResult<Vec<TokenOccurrence>>
where
    T: Tokenizer,
{
    let mut tokens = tokenizer.tokenize(&searchable_metadata_text(path, title));
    let mut page_tokens = parse_pdf_tokens(path, tokenizer)?;
    let position_offset = tokens.len();
    for token in &mut page_tokens {
        token.position += position_offset;
    }
    tokens.extend(page_tokens);
    Ok(tokens)
}

fn searchable_text(path: &Path, title: &str, content: &str) -> String {
    format!("{}\n{content}", searchable_metadata_text(path, title))
}

fn searchable_metadata_text(path: &Path, title: &str) -> String {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    let file_stem = path
        .file_stem()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();

    format!("{title}\n{file_name}\n{file_stem}")
}

fn clean_extracted_pdf_text(text: &str) -> String {
    text.chars()
        .map(|ch| {
            if ch.is_control() && ch != '\n' && ch != '\t' {
                ' '
            } else {
                ch
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn extract_title(content: &str, path: &Path, file_type: SupportedFileType) -> String {
    if file_type == SupportedFileType::Markdown {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("# ") {
                let title = rest.trim();
                if !title.is_empty() {
                    return title.to_string();
                }
            }
        }
    }

    path.file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}

fn flush_ascii(
    ascii_buf: &mut String,
    byte_start: usize,
    page: Option<u32>,
    min_len: usize,
    position: &mut usize,
    occurrences: &mut Vec<TokenOccurrence>,
) {
    if ascii_buf.len() >= min_len && !is_ascii_stop_word(ascii_buf) {
        push_ascii_token(ascii_buf, byte_start, page, position, occurrences);

        for part in ascii_subtokens(ascii_buf) {
            if part.len() >= min_len
                && part.as_str() != ascii_buf.as_str()
                && !is_ascii_stop_word(&part)
            {
                occurrences.push(TokenOccurrence {
                    token: part,
                    position: *position,
                    char_start: byte_start,
                    char_end: byte_start + ascii_buf.len(),
                    page,
                });
                *position += 1;
            }
        }
    }
    ascii_buf.clear();
}

fn ascii_subtokens(token: &str) -> Vec<String> {
    let mut parts = token
        .split('_')
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    let mut alpha = String::new();
    let mut numeric = String::new();
    for ch in token.chars() {
        if ch.is_ascii_alphabetic() {
            alpha.push(ch);
        } else {
            if alpha.len() >= 2 {
                parts.push(alpha.clone());
            }
            alpha.clear();
        }

        if ch.is_ascii_digit() {
            numeric.push(ch);
        } else {
            if numeric.len() >= 2 {
                parts.push(numeric.clone());
            }
            numeric.clear();
        }
    }
    if alpha.len() >= 2 {
        parts.push(alpha);
    }
    if numeric.len() >= 2 {
        parts.push(numeric);
    }

    parts.sort();
    parts.dedup();
    parts
}

fn push_ascii_token(
    ascii_buf: &str,
    byte_start: usize,
    page: Option<u32>,
    position: &mut usize,
    occurrences: &mut Vec<TokenOccurrence>,
) {
    occurrences.push(TokenOccurrence {
        token: ascii_buf.to_string(),
        position: *position,
        char_start: byte_start,
        char_end: byte_start + ascii_buf.len(),
        page,
    });
    *position += 1;
}

fn is_ascii_stop_word(token: &str) -> bool {
    matches!(
        token,
        "a" | "an"
            | "the"
            | "and"
            | "or"
            | "of"
            | "to"
            | "in"
            | "is"
            | "are"
            | "be"
            | "with"
            | "for"
            | "on"
            | "by"
            | "from"
            | "into"
            | "as"
            | "this"
            | "that"
            | "it"
            | "fn"
            | "let"
            | "mut"
            | "use"
            | "pub"
            | "impl"
            | "self"
    )
}

#[allow(dead_code)]
fn is_cjk_run(text: &str) -> bool {
    text.chars().all(is_cjk)
}

#[cfg(test)]
fn token_texts(tokenizer: &impl Tokenizer, text: &str) -> Vec<String> {
    tokenizer
        .tokenize(text)
        .into_iter()
        .map(|item| item.token)
        .collect()
}

#[cfg(test)]
fn assert_contains_token(tokens: &[String], expected: &str) {
    assert!(
        tokens.contains(&expected.to_string()),
        "expected token `{expected}` in {tokens:?}"
    );
}

#[cfg(test)]
fn assert_not_contains_token(tokens: &[String], unexpected: &str) {
    assert!(
        !tokens.contains(&unexpected.to_string()),
        "unexpected token `{unexpected}` in {tokens:?}"
    );
}

fn is_cjk(ch: char) -> bool {
    matches!(ch as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn tokenizer_handles_english_code_and_chinese() {
        let tokenizer = SimpleTokenizer::default();
        let tokens = token_texts(&tokenizer, "Rust ownership, borrow_check and 所有权模型");

        assert_contains_token(&tokens, "rust");
        assert_contains_token(&tokens, "ownership");
        assert_contains_token(&tokens, "borrow_check");
        assert_contains_token(&tokens, "borrow");
        assert_contains_token(&tokens, "check");
        assert_contains_token(&tokens, "所有权");
        assert_contains_token(&tokens, "模型");
        assert_not_contains_token(&tokens, "有权");
        assert_not_contains_token(&tokens, "and");
    }

    #[test]
    fn tokenizer_splits_letter_number_identifiers() {
        let tokenizer = SimpleTokenizer::default();
        let tokens = token_texts(&tokenizer, "fli-icml03 paper");

        assert_contains_token(&tokens, "icml03");
        assert_contains_token(&tokens, "icml");
        assert_contains_token(&tokens, "03");
    }

    #[test]
    fn tokenizer_uses_natural_chinese_words() {
        let tokenizer = SimpleTokenizer::default();
        let tokens = token_texts(&tokenizer, "本地知识库搜索系统支持所有权模型");

        assert_contains_token(&tokens, "本地");
        assert_contains_token(&tokens, "知识库");
        assert_contains_token(&tokens, "搜索");
        assert_contains_token(&tokens, "所有权");
        assert_not_contains_token(&tokens, "地知");
        assert_not_contains_token(&tokens, "库搜");
    }

    #[test]
    fn markdown_title_is_extracted() {
        let path = PathBuf::from("note.md");
        let title = extract_title("# Rust Notes\n\nbody", &path, SupportedFileType::Markdown);
        assert_eq!(title, "Rust Notes");
    }

    #[test]
    fn non_markdown_uses_file_stem() {
        let path = PathBuf::from("src/main.rs");
        let title = extract_title("fn main() {}", &path, SupportedFileType::Rust);
        assert_eq!(title, "main");
    }

    #[test]
    fn searchable_text_includes_file_name_and_title() {
        let path = PathBuf::from("knowledge_base/论文资料/icml03.pdf");
        let text = searchable_text(&path, "1776943102908-fli-icml03", "paper body");

        assert!(text.contains("icml03"));
        assert!(text.contains("paper body"));
    }

    #[test]
    fn pdf_text_cleaner_removes_control_characters() {
        let cleaned = clean_extracted_pdf_text("hello\u{0}\u{2}   world\npdf");

        assert_eq!(cleaned, "hello world pdf");
    }
}

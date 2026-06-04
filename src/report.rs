use std::fs;
use std::path::Path;

use crate::analysis::{summarize_index, top_terms};
use crate::error::AppResult;
use crate::model::{Document, InvertedIndex};

pub fn write_markdown_report(
    path: impl AsRef<Path>,
    index: &InvertedIndex,
    term_limit: usize,
) -> AppResult<()> {
    let report = build_markdown_report(index, term_limit);
    fs::write(path, report)?;
    Ok(())
}

pub fn build_markdown_report(index: &InvertedIndex, term_limit: usize) -> String {
    let summary = summarize_index(index, term_limit);
    let terms = top_terms(index, term_limit);

    let mut output = String::new();
    output.push_str("# RustNoteSearch Index Report\n\n");
    output.push_str("## Summary\n\n");
    output.push_str(&format!("- Root: `{}`\n", summary.root.display()));
    output.push_str(&format!("- Documents: {}\n", summary.document_count));
    output.push_str(&format!("- Terms: {}\n", summary.term_count));
    output.push_str(&format!("- Total tokens: {}\n", summary.total_tokens));
    output.push_str(&format!(
        "- Average tokens/document: {:.2}\n",
        summary.average_tokens_per_document
    ));
    output.push_str(&format!("- Index version: {}\n", index.metadata.version));
    output.push_str(&format!(
        "- Created timestamp: {}\n",
        index.metadata.created_secs
    ));

    output.push_str("\n## Top Terms\n\n");
    output.push_str("| Rank | Term | Frequency | Documents |\n");
    output.push_str("| --- | --- | ---: | ---: |\n");
    for (rank, term) in terms.iter().enumerate() {
        output.push_str(&format!(
            "| {} | `{}` | {} | {} |\n",
            rank + 1,
            escape_markdown_cell(&term.term),
            term.total_frequency,
            term.document_frequency
        ));
    }

    output.push_str("\n## Documents\n\n");
    output.push_str("| ID | Title | Tokens | Size | Path |\n");
    output.push_str("| ---: | --- | ---: | ---: | --- |\n");
    for document in sorted_documents(&index.documents) {
        output.push_str(&format!(
            "| {} | {} | {} | {} | `{}` |\n",
            document.id,
            escape_markdown_cell(&document.title),
            document.token_count,
            document.size_bytes,
            document.path.display()
        ));
    }

    output.push_str("\n## Notes\n\n");
    output.push_str("- This report is generated from the local JSON inverted index.\n");
    output.push_str("- It can be used as supporting material for the experiment report.\n");
    output.push_str("- Search quality depends on the tokenizer and the indexed file set.\n");
    output
}

fn sorted_documents(documents: &[Document]) -> Vec<&Document> {
    let mut sorted = documents.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then_with(|| left.path.cmp(&right.path))
    });
    sorted
}

fn escape_markdown_cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::index::IndexBuilder;
    use crate::parser::SimpleTokenizer;

    use super::*;

    #[test]
    fn report_contains_summary_terms_and_documents() {
        let temp = tempdir().expect("tempdir");
        fs::write(temp.path().join("note.md"), "# Note\nRust Rust report").expect("write note");
        let index = IndexBuilder::new(SimpleTokenizer::default())
            .build(temp.path())
            .expect("build");

        let report = build_markdown_report(&index, 5);

        assert!(report.contains("## Summary"));
        assert!(report.contains("## Top Terms"));
        assert!(report.contains("## Documents"));
        assert!(report.contains("Note"));
    }
}

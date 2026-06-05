use clap::Parser;

use rust_note_search::analysis;
use rust_note_search::cli::{Cli, Command};
use rust_note_search::config::AppConfig;
use rust_note_search::error::AppResult;
use rust_note_search::hybrid::search_with_strategy;
use rust_note_search::index::IndexBuilder;
use rust_note_search::parser::SimpleTokenizer;
use rust_note_search::report;
use rust_note_search::search::SearchOptions;
use rust_note_search::{storage, ui, vector};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> AppResult<()> {
    let cli = Cli::parse();
    let tokenizer = SimpleTokenizer::default();

    match cli.command {
        Command::Index { path, output } => {
            let builder = IndexBuilder::new(tokenizer);
            let index = builder.build(&path)?;
            storage::save_index(&output, &index)?;
            println!(
                "Indexed {} documents, {} terms, {} tokens.",
                index.metadata.document_count,
                index.metadata.term_count,
                index.metadata.total_tokens
            );
            println!("Saved index to {}", output.display());
        }
        Command::Search {
            query,
            index,
            limit,
            mode,
            strategy,
        } => {
            let index = storage::load_index(&index)?;
            let strategy = strategy.into();
            let config = if matches!(strategy, rust_note_search::model::SearchStrategy::Hybrid) {
                Some(AppConfig::from_env()?)
            } else {
                None
            };
            let results = search_with_strategy(
                &index,
                tokenizer,
                &query,
                SearchOptions {
                    limit,
                    mode: mode.into(),
                },
                strategy,
                config.as_ref(),
            )?;
            if results.is_empty() {
                println!("No results for \"{query}\".");
            } else {
                for (rank, result) in results.iter().enumerate() {
                    let file_name = result
                        .path
                        .file_name()
                        .map(|name| name.to_string_lossy())
                        .unwrap_or_else(|| result.path.to_string_lossy());
                    let pages = result_pages(result);
                    println!(
                        "{}. {:.2} | {} | {}",
                        rank + 1,
                        result.score,
                        file_name,
                        pages.unwrap_or_else(|| "pages: -".to_string())
                    );
                }
            }
        }
        Command::VectorIndex { index } => {
            let index = storage::load_index(&index)?;
            let config = AppConfig::from_env()?;
            let stats = vector::build_vector_index(&index, &config)?;
            println!(
                "Indexed {} vector chunks into Qdrant collection {}.",
                stats.chunk_count, stats.collection
            );
        }
        Command::Stats { index } => {
            let index = storage::load_index(&index)?;
            let summary = analysis::summarize_index(&index, 8);
            print!("{}", analysis::format_summary(&summary));
            println!("Index version: {}", index.metadata.version);
            println!("Created timestamp: {}", index.metadata.created_secs);
        }
        Command::Terms { index, limit } => {
            let index = storage::load_index(&index)?;
            let terms = analysis::top_terms(&index, limit);
            print!("{}", analysis::format_terms(&terms));
        }
        Command::Inspect {
            doc_id,
            index,
            limit,
        } => {
            let index = storage::load_index(&index)?;
            let inspection = analysis::inspect_document(&index, doc_id, limit)?;
            print!("{}", analysis::format_inspection(&inspection));
        }
        Command::Report {
            index,
            output,
            limit,
        } => {
            let index = storage::load_index(&index)?;
            report::write_markdown_report(&output, &index, limit)?;
            println!("Wrote report to {}", output.display());
        }
        Command::Tui { index } => {
            let index_path = index;
            let index = storage::load_index(&index_path)?;
            ui::run_tui(index, index_path, tokenizer)?;
        }
    }

    Ok(())
}

fn result_pages(result: &rust_note_search::model::SearchResult) -> Option<String> {
    let mut pages = result
        .matches
        .iter()
        .filter_map(|item| item.page)
        .collect::<Vec<_>>();
    pages.sort_unstable();
    pages.dedup();

    if pages.is_empty() {
        None
    } else {
        Some(format!(
            "pages: {}",
            pages
                .iter()
                .map(|page| page.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ))
    }
}

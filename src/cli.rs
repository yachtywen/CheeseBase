use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::model::SearchMode;

#[derive(Debug, Parser)]
#[command(name = "rust-note-search")]
#[command(about = "A local knowledge-base search tool written in Rust")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Build an index from a local directory.
    Index {
        /// Directory to scan.
        path: PathBuf,

        /// Output JSON index file.
        #[arg(short, long, default_value = "index.json")]
        output: PathBuf,
    },

    /// Search an existing index.
    Search {
        /// Query text.
        query: String,

        /// JSON index file.
        #[arg(short, long, default_value = "index.json")]
        index: PathBuf,

        /// Maximum number of results.
        #[arg(short = 'n', long, default_value_t = 10)]
        limit: usize,

        /// Match any query term or require all query terms.
        #[arg(short, long, value_enum, default_value_t = CliSearchMode::Any)]
        mode: CliSearchMode,
    },

    /// Show index statistics.
    Stats {
        /// JSON index file.
        #[arg(short, long, default_value = "index.json")]
        index: PathBuf,
    },

    /// Show the most frequent terms in the index.
    Terms {
        /// JSON index file.
        #[arg(short, long, default_value = "index.json")]
        index: PathBuf,

        /// Number of terms to print.
        #[arg(short = 'n', long, default_value_t = 20)]
        limit: usize,
    },

    /// Inspect one indexed document by its document id.
    Inspect {
        /// Document id printed by search results.
        doc_id: usize,

        /// JSON index file.
        #[arg(short, long, default_value = "index.json")]
        index: PathBuf,

        /// Number of document-local terms to print.
        #[arg(short = 'n', long, default_value_t = 12)]
        limit: usize,
    },

    /// Export a Markdown report from the index.
    Report {
        /// JSON index file.
        #[arg(short, long, default_value = "index.json")]
        index: PathBuf,

        /// Output Markdown report file.
        #[arg(short, long, default_value = "index-report.md")]
        output: PathBuf,

        /// Number of top terms to include.
        #[arg(short = 'n', long, default_value_t = 20)]
        limit: usize,
    },

    /// Open the terminal search interface.
    Tui {
        /// JSON index file.
        #[arg(short, long, default_value = "index.json")]
        index: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliSearchMode {
    Any,
    All,
}

impl From<CliSearchMode> for SearchMode {
    fn from(value: CliSearchMode) -> Self {
        match value {
            CliSearchMode::Any => SearchMode::Any,
            CliSearchMode::All => SearchMode::All,
        }
    }
}

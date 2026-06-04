# RustNoteSearch Project Notes

RustNoteSearch is a local knowledge-base search system.

The project supports Markdown, text, Rust source code, TOML, and text-based PDF
files. It recursively scans the knowledge base, builds an inverted index, saves
the index as JSON, and searches with BM25 ranking.

The TUI provides a cover page, slash commands, indexed file browsing, top-term
analysis, statistics, and interactive search.

Useful commands:

- `/help`
- `/files`
- `/terms`
- `/stats`
- `/update`
- `/select`

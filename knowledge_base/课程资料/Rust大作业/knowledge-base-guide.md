# Knowledge Base Guide

The `knowledge_base` directory is the official local knowledge base.

Users can manually add, delete, rename, and move files or folders. The program
does not enforce a fixed directory layout. After changing files, rebuild the
index with one of the following methods:

- CLI: `cargo run -- index knowledge_base`
- TUI: type `/update`

The `/files`, `/terms`, and `/stats` pages are based on the current index. If
the file system changes but the index is not updated, those pages still show the
old indexed state.

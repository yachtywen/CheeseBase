# Rust Error Handling

Rust encourages explicit error handling through `Result<T, E>`.

In RustNoteSearch, file scanning, PDF extraction, JSON loading, TUI rendering,
and index updating all return `AppResult<T>`. This design avoids hidden panic
paths and makes failures visible to the caller.

Common patterns:

- Use `?` to propagate errors.
- Use custom error enums for application-level failures.
- Keep old data available when a refresh operation fails.

The `/update` command should rebuild the index safely. If rebuilding fails, the
TUI keeps the previous index in memory and displays an error message.

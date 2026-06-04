# Rust Ownership Notes

Rust ownership is a memory management model without a garbage collector.

The main rules are:

- Each value has one owner.
- A value can be borrowed immutably many times.
- A value can be borrowed mutably only when no other borrow exists.
- When the owner goes out of scope, the value is dropped.

Ownership and borrowing make Rust suitable for reliable local search tools. The index builder reads file content, tokenizes it, and then moves owned strings into the inverted index.

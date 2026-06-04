# Rust Ownership Notes

Rust ownership is a memory management model without a garbage collector.

The main rules are:

- Each value has one owner.
- A value can have only one owner at a time.
- When the owner goes out of scope, the value is dropped.

Ownership and borrowing make Rust suitable for reliable local search tools.
The index builder reads file content, tokenizes it, and stores owned strings
inside the inverted index. Borrowed references are used when searching so that
large document content does not need to be copied repeatedly.

Useful keywords: ownership, borrowing, lifetime, Result, trait, rayon.

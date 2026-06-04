# MySQL Index Notes

MySQL indexes improve query performance by reducing the amount of data scanned.

Common index types:

- Primary key index
- Unique index
- Composite index
- Covering index

Index design should consider selectivity, query predicates, sorting, and update
cost. A composite index follows the leftmost prefix rule. For slow SQL analysis,
`EXPLAIN` can show access type, possible keys, selected key, rows, and extra
execution information.

Useful keywords: index, B+ tree, composite index, covering index, explain.

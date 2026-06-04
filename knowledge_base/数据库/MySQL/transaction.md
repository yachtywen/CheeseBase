# MySQL Transaction Notes

A transaction groups multiple SQL operations into one logical unit.

ACID properties:

- Atomicity: all operations succeed or fail together.
- Consistency: data moves from one valid state to another.
- Isolation: concurrent transactions should not corrupt each other.
- Durability: committed changes survive crashes.

Isolation levels include read uncommitted, read committed, repeatable read, and
serializable. MySQL InnoDB uses MVCC to support concurrent reads and writes.

Useful keywords: transaction, ACID, isolation, MVCC, lock, rollback, commit.

中文摘要：事务用于保证一组数据库操作要么全部成功，要么全部失败。
在 MySQL 中，事务、隔离级别、锁和 MVCC 是理解并发控制的核心概念。

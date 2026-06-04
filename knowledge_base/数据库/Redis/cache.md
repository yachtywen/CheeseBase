# Redis Cache Notes

Redis is often used as an in-memory cache.

Common cache problems:

- Cache penetration: requests query data that does not exist.
- Cache breakdown: a hot key expires and many requests hit the database.
- Cache avalanche: many keys expire at the same time.

Mitigations include null-value caching, Bloom filters, mutex locks, random TTL,
and multi-level cache design.

Useful keywords: Redis, cache, TTL, hot key, Bloom filter, avalanche.

# Storage Backend Spike

Architecture spike comparing SQLite+sqlite-vec vs LanceDB (for vector search)
and Tantivy (for BM25 full-text search) for Vera's storage needs.

## What We Test

- **Write throughput**: Insert 10K chunks with 768-dim vectors
- **Vector query latency**: KNN search (p50/p95/p99) over 100 queries
- **BM25 query latency**: Full-text search with realistic code queries
- **Storage size on disk**: Total index artifacts

## How to Run

```bash
cd spikes/storage
cargo run --release
```

Results are saved to `results/` as JSON files.

## Backends Tested

1. **SQLite + sqlite-vec**: Metadata in SQLite, vectors via sqlite-vec virtual table
2. **LanceDB**: Arrow-native columnar storage with built-in vector search
3. **Tantivy**: Lucene-like BM25 full-text search engine (for BM25 component)

## Key Insight

Both SQLite and LanceDB need a separate BM25 engine (Tantivy) for full-text search.
The decision is primarily about vector storage + metadata, not BM25.

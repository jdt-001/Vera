# ADR-002: Storage Backend

**Status:** Accepted
**Date:** 2026-03-20

## Question

What storage backend should Vera use for its code index? Vera needs to store:
1. **Chunk metadata** (file path, symbol name/type, language, line ranges)
2. **Embedding vectors** (768-dim float32 for semantic search)
3. **Full-text index** (BM25 for lexical/keyword search)

The choice affects write throughput during indexing, query latency during retrieval, storage footprint, binary size, compile time, and long-term maintainability.

## Options Considered

### Option A: SQLite + sqlite-vec + Tantivy

- **Metadata:** SQLite tables via `rusqlite` (mature, battle-tested, single-file DB)
- **Vectors:** `sqlite-vec` extension for brute-force KNN vector search within SQLite
- **BM25:** Tantivy as a separate index for full-text search
- **Pros:** Lightweight dependencies, fast compile (~40s incremental), simple API, SQLite is the most deployed DB in the world, trivial backup/migration (single .db file for metadata+vectors), no async required for metadata/vector queries
- **Cons:** Slower vector writes and queries vs LanceDB; sqlite-vec is relatively new (v0.1.x); no built-in ANN index (brute-force only, acceptable for <100K chunks)

### Option B: LanceDB + Tantivy

- **Metadata:** Stored as Arrow columns in LanceDB tables
- **Vectors:** LanceDB native vector search with optional IVF-PQ ANN index
- **BM25:** Tantivy as a separate index (LanceDB has full-text search but Tantivy is more mature for BM25)
- **Pros:** Fastest vector writes and queries, Arrow-native columnar format, built-in ANN indexing for future scaling, active development
- **Cons:** Massive dependency tree (537 crates including Arrow, DataFusion, lance-*), 2.5 minute full build, complex API requiring 3+ trait imports, ~15MB+ added to binary size, all operations require async runtime

### Tantivy (shared by both options)

Both options use Tantivy for BM25 full-text search. Tantivy is mature (v0.22+, used by Quickwit), has excellent performance, and is the clear Rust BM25 choice. This is not a contested decision.

## Evaluation Method

Built spike implementations for all three backends in `spikes/storage/`, measuring identical workloads:

- **Write throughput:** Insert 10,000 chunks with 768-dim vectors and metadata
- **Vector query latency:** 100 KNN queries (top-10) using brute-force scan, with warmup
- **BM25 query latency:** 100 text queries using realistic code search terms, with warmup
- **Storage size:** Total on-disk footprint after indexing

**Environment:** AMD Ryzen 5 7600X3D (12 threads), 30GB RAM, Arch Linux, Rust 1.94.0

**Configuration:**
- SQLite: WAL mode, batch inserts of 500 chunks, `sqlite-vec` float[768] virtual table
- LanceDB: Local mode, batch inserts of 1,000 chunks, flat scan (no ANN index)
- Tantivy: 50MB writer heap, default tokenizer, 5 indexed text fields

Both vector backends use brute-force search (no ANN index) for fair comparison. At Vera's expected scale (<100K chunks per repo), brute-force is sufficient and ANN adds complexity without meaningful benefit.

## Evidence

### Write Throughput (10K chunks with 768-dim vectors)

| Backend | Total Time (ms) | Throughput (chunks/sec) | Speedup vs SQLite |
|---------|----------------:|------------------------:|------------------:|
| SQLite + sqlite-vec | 1,316.6 | 7,596 | 1.0× |
| LanceDB | 41.4 | 241,440 | **31.8×** |
| Tantivy (text only) | 36.3 | 275,104 | 36.3× |

LanceDB's columnar batch-write model is dramatically faster for bulk inserts. SQLite's row-by-row insert with sqlite-vec virtual table is the bottleneck. However, even SQLite's 7,596 chunks/sec translates to indexing 10K chunks in ~1.3 seconds — well within Vera's 60-second budget for 100K LOC repos.

### Vector Query Latency (100 KNN queries, top-10, brute-force)

| Backend | p50 (ms) | p95 (ms) | p99 (ms) | Mean (ms) |
|---------|----------:|----------:|----------:|-----------:|
| SQLite + sqlite-vec | 9.731 | 10.013 | 10.215 | 9.747 |
| LanceDB | 1.945 | 2.809 | 3.050 | 2.078 |

LanceDB is **5× faster** at vector queries. Both are well within Vera's 500ms p95 target. SQLite-vec's ~10ms per query leaves ample room for the hybrid pipeline (BM25 + vector + fusion + reranking).

### BM25 Query Latency (100 queries, top-10, Tantivy)

| Metric | Value |
|--------|------:|
| p50 | 0.067 ms |
| p95 | 0.133 ms |
| p99 | 0.138 ms |
| Mean | 0.064 ms |

Tantivy BM25 is extremely fast — sub-millisecond for all queries. This is shared between both architecture options.

### Storage Size

| Backend | Size (KB) | Size (MB) | Relative |
|---------|----------:|----------:|---------:|
| SQLite + sqlite-vec | 34,028 | 33.2 | 1.03× |
| LanceDB | 32,928 | 32.2 | 1.0× |
| Tantivy (text only) | 1,504 | 1.5 | — |

Storage is nearly identical for both vector backends. The vector data (10K × 768 × 4 bytes = 30.7 MB) dominates. Combined with Tantivy, total index size would be ~34–36 MB for either option.

### API Ergonomics & Developer Experience

| Aspect | SQLite + sqlite-vec | LanceDB |
|--------|:---:|:---:|
| Dependency count | ~60 crates | 537 crates |
| Full build time | ~40s | ~150s |
| Incremental build | ~3s | ~8s |
| Async required | No | Yes (tokio runtime) |
| Trait imports needed | 0 | 3+ (`QueryBase`, `ExecutableQuery`, `IntoQueryVector`) |
| Data model | SQL rows | Arrow RecordBatches |
| Error messages | Clear, well-documented | Complex, generic trait bounds |
| Crate maturity | rusqlite: 10+ years, sqlite-vec: v0.1 | lancedb: v0.27, 2 years |
| Documentation | Excellent (rusqlite) | Adequate but sparse for Rust |
| Single-file DB | Yes (.db file) | No (directory of lance files) |

### Correctness Verification

All three backends passed self-query correctness checks:
- SQLite+sqlite-vec: querying chunk 0's vector returns chunk 0 ✓
- LanceDB: querying chunk 0's vector returns chunk 0 ✓
- Tantivy: searching chunk 0's symbol name returns chunk 0 ✓

## Decision

**Option A: SQLite + sqlite-vec + Tantivy**

Despite LanceDB's superior raw performance (5× faster vector queries, 32× faster writes), we choose SQLite + sqlite-vec + Tantivy for the following reasons:

1. **Performance is sufficient.** SQLite-vec's 10ms p50 vector query latency and 7,596 chunks/sec write throughput are well within Vera's requirements (500ms p95 query budget, 60s index budget for 100K LOC). The entire hybrid pipeline (BM25 + vector + fusion) will run in <50ms even with SQLite-vec.

2. **Dramatically simpler dependency tree.** ~60 crates vs 537. This directly impacts compile times (40s vs 150s), binary size, and supply chain risk. For a CLI tool distributed as a single binary, minimizing dependencies is a first-class concern.

3. **Simpler programming model.** Synchronous SQLite API vs async-required LanceDB. No trait import gymnastics. SQL-based queries are more readable and debuggable than Arrow builder patterns. SQLite's single-file model simplifies backup, migration, and debugging.

4. **SQLite is the most battle-tested database.** Combined with rusqlite (10+ years of production use), this is the lowest-risk foundation. sqlite-vec is newer (v0.1), but the vector search surface area is small and well-tested.

5. **Tantivy for BM25 is uncontested.** Both options need it, so the choice is really just about vector storage + metadata, where SQLite's simplicity advantage outweighs LanceDB's speed advantage at Vera's scale.

6. **ANN indexing is not needed.** At Vera's expected scale (<100K chunks per repo), brute-force KNN is fast enough. If future scaling requires it, sqlite-vec can be replaced without affecting the Tantivy BM25 or SQLite metadata layers.

## Consequences

**Gains:**
- Fast compile times and small binary size from minimal dependencies
- Simple, synchronous API for metadata and vector operations
- Single .db file for index portability and debugging
- Mature, well-documented foundation (rusqlite + SQLite)
- Clean separation: SQLite for metadata+vectors, Tantivy for BM25
- No async runtime required for storage operations

**Trade-offs accepted:**
- ~5× slower vector queries than LanceDB (10ms vs 2ms) — acceptable within 500ms budget
- ~32× slower bulk writes than LanceDB (1.3s vs 41ms for 10K chunks) — acceptable within 60s budget
- sqlite-vec is v0.1 with limited community — mitigated by small API surface and easy replaceability
- No built-in ANN index — acceptable at Vera's scale, can be added later if needed

**Mitigations:**
- SQLite WAL mode and batch transactions optimize write throughput
- Vector query caching can reduce repeated query costs
- If sqlite-vec proves problematic, the vector search layer can be swapped independently (e.g., to hnswlib or custom SIMD brute-force) without affecting metadata or BM25

## Follow-up

1. Benchmark incremental update performance (insert/delete individual chunks) — deferred to M3
2. Monitor sqlite-vec development and version stability
3. If repo sizes exceed 100K chunks, evaluate ANN options (hnswlib via FFI, or custom SIMD)
4. Consider SQLite FTS5 as a potential alternative/complement to Tantivy for simpler single-DB architecture
5. Profile end-to-end hybrid pipeline latency once all components are integrated

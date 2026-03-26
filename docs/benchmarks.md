# Vera Benchmarks

This page tracks two benchmark snapshots:

- the current local release benchmark used to tune retrieval quality
- the older public API benchmark kept for historical comparison

## Current Local Release Benchmark

This is the benchmark used to measure the `v0.7.0` retrieval pipeline.

- 21 tasks
- 4 repos: `ripgrep`, `flask`, `fastify`, `turborepo`
- local Jina embedding + reranker stack
- CUDA ONNX backend
- same pinned corpora and the same local-binary harness for every version below

### Accuracy Improvements From `v0.4.0` To `v0.7.0`

| Version | Recall@1 | Recall@5 | Recall@10 | MRR@10 | nDCG@10 |
|--------|----------|----------|-----------|--------|---------|
| `v0.4.0` | 0.2421 | 0.5040 | 0.5159 | 0.5016 | 0.4570 |
| `v0.5.0` | 0.3135 | 0.5635 | 0.6349 | 0.5452 | 0.5293 |
| `v0.7.0` | **0.7183** | **0.7778** | **0.8254** | **0.9095** | **0.8361** |

From `v0.4.0` to `v0.7.0`, Vera improved by:

- `+0.4762` Recall@1
- `+0.2738` Recall@5
- `+0.3095` Recall@10
- `+0.4079` MRR@10
- `+0.3791` nDCG@10

Committed artifacts:

- [v0.4.0 benchmark](../benchmarks/results/local-binaries/v0.4.0-jina-cuda-onnx.json)
- [v0.5.0 benchmark](../benchmarks/results/local-binaries/v0.5.0-jina-cuda-onnx.json)
- [v0.7.0 benchmark](../benchmarks/results/local-binaries/v0.7.0-jina-cuda-onnx.json)
- [`v0.7.0` accuracy improvements](./releases/v0.7.0-accuracy-improvements.md)

### Current Performance Snapshot

`v0.7.0` local Jina CUDA ONNX results:

| Measure | Result |
|---------|--------|
| Search latency p50 | `3716 ms` |
| Search latency p95 | `4185 ms` |

## Vera vs ColGREP

These ColGREP numbers are the earlier reference results recorded on the same 21-task, 4-repo suite. They remain useful as a retrieval quality reference because they show how the current Vera pipeline compares with a late-interaction code search system on the same workload.

| Metric | Vera `v0.7.0` | ColGREP (149M) | ColGREP Edge (17M) |
|--------|---------------|----------------|--------------------|
| Recall@1 | **0.7183** | 0.5710 | 0.5240 |
| Recall@5 | **0.7778** | 0.6670 | 0.5710 |
| Recall@10 | **0.8254** | 0.7140 | 0.7140 |
| MRR@10 | **0.9095** | 0.6170 | 0.5660 |
| nDCG@10 | **0.8361** | 0.5610 | 0.5240 |

Indexing time, 4 repos combined:

| Tool | Total time | Hardware |
|------|-----------|----------|
| Vera `v0.7.0` | `~70 s` | RTX 4080 |
| ColGREP (149M, CPU) | `~180 s` | Ryzen 5 7600X3D 6c/12t |
| ColGREP Edge (17M, CPU) | `~160 s` | Ryzen 5 7600X3D 6c/12t |

ColGREP's late-interaction design was a useful reference while improving Vera's own ranking and chunk selection.

## Legacy Public API Benchmark

This is the older public benchmark snapshot that still appears in older docs and release notes.

- 17 tasks
- 3 repos: `ripgrep`, `flask`, `fastify`
- mixed API and local runs

### Retrieval Quality

| Metric | ripgrep | cocoindex-code | vector-only | Vera hybrid |
|--------|---------|----------------|-------------|-------------|
| Recall@1 | 0.1548 | 0.1587 | 0.0952 | **0.4265** |
| Recall@5 | 0.2817 | 0.3730 | 0.4921 | **0.6961** |
| Recall@10 | 0.3651 | 0.5040 | 0.6627 | **0.7549** |
| MRR@10 | 0.2625 | 0.3517 | 0.2814 | **0.6009** |
| nDCG@10 | 0.2929 | 0.5206 | 0.7077 | **0.8008** |

### Local vs API Models

The local Jina models were competitive with the much larger Qwen3-Embedding-8B API model on that older 17-task benchmark:

| Metric | Jina local (ONNX) | Qwen3-8B (API) |
|--------|-------------------|----------------|
| MRR@10 | **0.68** | 0.60 |
| Recall@5 | 0.65 | **0.73** |
| Recall@10 | 0.73 | **0.75** |
| nDCG@10 | 0.72 | **0.81** |

### Performance Snapshot

From the same older benchmark set:

| Measure | Result |
|---------|--------|
| BM25-only search p95 | `3.5 ms` |
| Hybrid search p95 | `6749 ms` |
| `ripgrep` index time | `65.1 s` |
| `flask` index time | `20.2 s` |
| `fastify` index time | `41.8 s` |

## Limits And Caveats

- The current release benchmark is deterministic and fully local, which makes it better for regression gating.
- The legacy public snapshot is still useful for older comparisons, but it should not be treated as the current retrieval baseline.
- Benchmark numbers in this repository show comparative behavior, not a promise that another machine or codebase will land on the same values.

## Related Docs

- [`v0.7.0` accuracy improvements](./releases/v0.7.0-accuracy-improvements.md)
- [Indexing performance note](../benchmarks/indexing-performance.md)
- [Reproduction guide](../benchmarks/reports/reproduction-guide.md)

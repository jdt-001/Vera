# Vera Benchmark Reproduction Guide

How to reproduce the Vera final benchmark results.

## Prerequisites

### System Requirements
- Linux x86_64 (tested on CachyOS/Arch Linux)
- 8+ GB RAM (30 GB recommended)
- 10+ GB free disk space
- Internet connection for embedding/reranker APIs

### Tool Versions

| Tool              | Version                  | Install Command |
|-------------------|--------------------------|-----------------|
| Rust              | 1.94.0+                  | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Python            | 3.14+                    | System package manager |
| ripgrep           | 13.0.0+                  | `cargo install ripgrep` or system package |
| git               | 2.40+                    | System package manager |

### API Credentials

Create a `secrets.env` file at the repository root with:
```bash
EMBEDDING_MODEL_BASE_URL=<your-openai-compatible-embedding-api-url>
EMBEDDING_MODEL_ID=Qwen/Qwen3-Embedding-8B
EMBEDDING_MODEL_API_KEY=<your-api-key>
RERANKER_MODEL_BASE_URL=<your-openai-compatible-reranker-api-url>
RERANKER_MODEL_ID=Qwen/Qwen3-Reranker
RERANKER_MODEL_API_KEY=<your-api-key>
```

The embedding and reranker APIs must be OpenAI-compatible endpoints.
We used Nebius tokenfactory for benchmarking.

## Corpus Setup

### Step 1: Clone and pin test repositories
```bash
bash eval/setup-corpus.sh
```

This clones 4 repositories to `.bench/repos/` and pins them to specific SHAs:

| Repository | Language   | Commit SHA                                       |
|------------|-----------|--------------------------------------------------|
| ripgrep    | Rust       | `4519153e5e461527f4bca45b042fff45c4ec6fb9`       |
| flask      | Python     | `4cae5d8e411b1e69949d8fae669afeacbd3e5908`       |
| fastify    | TypeScript | `a22217f9420f70017a419d8e18b2a3141ab27989`       |
| turborepo  | Polyglot   | `56b79ff5c1c9366593e9e68a922d997e2698c5f4`       |

### Step 2: Verify corpus
```bash
cargo run --bin vera-eval -- verify-corpus
```

## Build Vera

```bash
cargo build --release
```

## Run Benchmarks

### Full Suite (single command)
```bash
set -a; source secrets.env; set +a
python3 benchmarks/scripts/run_final_benchmarks.py
```

This will:
1. Clean and re-index all 4 repositories (with cooldown between repos)
2. Run 21 benchmark tasks in 3 Vera modes (bm25-only, hybrid-norerank, hybrid)
3. Compare against pre-computed competitor baselines
4. Verify all performance targets
5. Produce comparison tables and save results to `benchmarks/results/final-suite/`

### Individual Components
```bash
# Just Vera benchmarks (skip indexing)
python3 benchmarks/scripts/run_vera_benchmarks.py --modes bm25-only hybrid-norerank hybrid --skip-index --runs 2

# Competitor baselines
python3 benchmarks/scripts/run_baselines.py --tool all --runs 3

# Just ripgrep baseline
python3 benchmarks/scripts/run_baselines.py --tool ripgrep --runs 3
```

## Expected Ranges

Results should fall within these ranges on comparable hardware:

| Metric                     | Expected Range    | Notes |
|----------------------------|-------------------|-------|
| Vera hybrid Recall@10      | 0.70 – 0.85       | Depends on API model version |
| Vera hybrid MRR@10         | 0.50 – 0.70       | Reranker quality varies |
| Vera BM25-only Recall@10   | 0.35 – 0.50       | Deterministic, stable |
| BM25 p95 latency           | 1 – 15 ms         | Depends on disk/cache |
| Hybrid p95 latency         | 3000 – 10000 ms   | Dominated by API round trips |
| ripgrep index time         | 50 – 120 s        | Dominated by embedding API |
| Index size ratio           | 1.0x – 2.0x       | Depends on repo structure |
| Incremental update         | 1 – 5 s           | Single file change |

## Metric Reproducibility

- **Retrieval metrics** (Recall, MRR, nDCG): Deterministic — same query yields
  same results. Two runs should match within ±2%.
- **Latency**: Varies by ~20% due to API response times and system load.
  Use BM25-only mode for stable latency measurements.
- **Index time**: Dominated by embedding API throughput. Varies 20-50%
  depending on API load.

## Troubleshooting

### Dimension mismatch errors
If you see dimension errors, clear old indexes:
```bash
rm -rf .bench/repos/*/.vera
```

### API rate limits
The script includes cooldown periods between repos. If rate limits persist,
increase `COOLDOWN_SECS` in the script or index repos individually with
pauses between them.

### Turborepo indexing fails
Turborepo is a large polyglot repo and requires more API calls. If it fails,
run the other 3 repos first, wait 60 seconds, then index turborepo:
```bash
set -a; source secrets.env; set +a
./target/release/vera index .bench/repos/turborepo
```

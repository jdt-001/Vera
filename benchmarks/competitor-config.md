# Competitor Baseline Configurations

Detailed configuration and reproduction documentation for each competitor
tool benchmarked as a baseline for Vera.

## 1. ripgrep (Lexical Baseline)

### Version
```
ripgrep 13.0.0 (rev af6b6c543b)
-SIMD -AVX (compiled)
+SIMD +AVX (runtime)
```

### Installation
ripgrep was pre-installed on the benchmark system. To install:
```bash
# Arch Linux
pacman -S ripgrep

# macOS
brew install ripgrep

# From source
cargo install ripgrep
```

### Configuration
- **Search mode:** Case-insensitive content search (`rg -i --json`)
- **Query strategy:**
  - Extract identifiers (CamelCase, snake_case, dotted names, filenames) from query
  - Extract significant keywords (excluding stop words)
  - Search each term independently, weight identifiers 3x
  - Group results by file+line-range proximity (30-line window)
  - Rank by cumulative match count
- **Max results per search:** 20
- **No indexing required** (live filesystem search)

### Reproduction
```bash
# Verify version
rg --version

# Run benchmark
python3 benchmarks/scripts/run_baselines.py --tool ripgrep --runs 3
```

### Strengths
- Extremely fast (p50 ~18ms)
- Zero index time and storage
- Excellent for exact identifier matches
- Good disambiguation (finds all variants)

### Weaknesses
- Cannot match semantic intent (only lexical)
- Poor on config file lookup (searches content, not filenames)
- No understanding of code structure

---

## 2. cocoindex-code (Semantic Competitor)

### Version
```
cocoindex-code 0.2.4
```

### Installation
```bash
# Via pipx (recommended)
pipx install cocoindex-code

# Or via uv
uv tool install --upgrade cocoindex-code --prerelease explicit --with "cocoindex>=1.0.0a24"
```

### Configuration
- **Embedding model:** sentence-transformers/all-MiniLM-L6-v2 (local, default)
  - 384-dimensional embeddings
  - No API key required
- **Chunking:** AST-based (tree-sitter) with language-specific parsing
- **Search mode:** Semantic similarity via embedded chunks
- **Index location:** `.cocoindex_code/` directory in each repo
- **Initialization:** `ccc init && ccc index` per repo

### Reproduction
```bash
# Install
pipx install cocoindex-code

# Initialize and index a repo
cd .bench/repos/ripgrep
ccc init
ccc index

# Search
ccc search "Searcher struct definition" --limit 20

# Run full benchmark
python3 benchmarks/scripts/run_baselines.py --tool cocoindex --runs 3
```

### Strengths
- AST-aware chunking preserves code structure
- Good balance of precision and recall
- Fast indexing (~5s for 4 repos)
- No API costs (local embeddings)
- Best cross-file discovery among baselines

### Weaknesses
- Higher query latency (~445ms) than ripgrep
- Default MiniLM model is general-purpose, not code-optimized
- Config/build file search is weak
- Documentation files can dilute results

---

## 3. Vector-Only Baseline (Custom Embedding + Search)

### Version
```
vector-only (embedding: Qwen/Qwen3-Embedding-8B)
Custom implementation using API embeddings + cosine similarity
```

### Configuration
- **Embedding model:** Qwen/Qwen3-Embedding-8B (remote API)
- **Embedding dimensions:** Variable (model default)
- **Chunking:** Line-based, 50 lines per chunk with 10-line overlap
- **Search method:** Brute-force cosine similarity over all chunk embeddings
- **File limit:** 500 source files per repo (prioritizing src/lib/crates/)
- **Chunk truncation:** Content truncated to 2000 chars per chunk for embedding

### Environment Variables Required
```bash
EMBEDDING_MODEL_BASE_URL=<api-base-url>
EMBEDDING_MODEL_ID=Qwen/Qwen3-Embedding-8B
EMBEDDING_MODEL_API_KEY=<api-key>
```

### Reproduction
```bash
# Load API credentials
set -a; source secrets.env; set +a

# Run benchmark (1 timing run due to API latency)
python3 benchmarks/scripts/run_baselines.py --tool vector-only --runs 1
```

### Strengths
- Highest Recall@10 (0.66) and nDCG (0.71) overall
- Best at semantic/intent queries
- Can find config files by content meaning
- Code-optimized embedding model (Qwen3)

### Weaknesses
- Very high latency (~1.2s per query due to API)
- Expensive indexing (415s for 4 repos, API costs)
- Naive line-based chunking loses code structure
- Poor disambiguation (embeddings cluster on one meaning)
- Requires API access and credentials

---

## Comparison Summary

| Aspect               | ripgrep       | cocoindex-code | vector-only   |
|---------------------|---------------|----------------|---------------|
| **Type**            | Lexical       | AST+Semantic   | Vector-only   |
| **Indexing**        | None          | Local          | Remote API    |
| **Embeddings**      | N/A           | MiniLM-L6-v2   | Qwen3-8B      |
| **Chunking**        | N/A           | AST-based      | Line-based    |
| **Recall@10**       | 0.37          | 0.50           | 0.66          |
| **MRR**             | 0.26          | 0.35           | 0.28          |
| **Query Latency**   | 18ms          | 446ms          | 1186ms        |
| **Index Time**      | 0s            | 5s             | 416s          |
| **Storage**         | 0 MB          | 143 MB         | 421 MB        |
| **API Required**    | No            | No             | Yes           |

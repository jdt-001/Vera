# Local Models

Vera's local pipeline uses one embedding model plus the curated local reranker. `vera setup` downloads the selected embedding model into `~/.vera/models/` when the source is Hugging Face, and always installs the matching ONNX Runtime library into `~/.vera/lib/`.

## Curated Embedding Options

| Option | Command | Notes |
| --- | --- | --- |
| Jina v5 nano retrieval | `vera setup` | Default. Best general-purpose local choice. Faster indexing and the strongest end-to-end benchmark coverage in Vera so far. |
| CodeRankEmbed | `vera setup --code-rank-embed` | Optional. Useful when you want a code-specific bi-encoder or you are testing without reranking. On Vera's short 6-task no-rerank check it beat the Jina preset on retrieval quality, but indexing was much slower. |

The local reranker stays the same for both options:

| Model | Role |
| --- | --- |
| [`jinaai/jina-reranker-v2-base-multilingual`](https://huggingface.co/jinaai/jina-reranker-v2-base-multilingual) | Local cross-encoder reranker |

## CodeRankEmbed Comparison

Short no-rerank check on 6 tasks across `flask` and `ripgrep` with CUDA ONNX:

| Model | Recall@1 | Recall@5 | Recall@10 | MRR | nDCG | Search p50 | Flask index | Ripgrep index |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Jina preset | 0.5556 | 0.5556 | 0.5556 | 0.8462 | 0.6442 | 761.9 ms | 5.8 s | 11.9 s |
| CodeRankEmbed preset | 0.7222 | 0.7222 | 0.7222 | 1.0000 | 0.8108 | 611.4 ms | 14.7 s | 29.1 s |

This is why CodeRankEmbed ships as an option, not the default. It showed better pure embedding retrieval on that small no-rerank slice, but Vera's full local benchmark still uses the Jina preset because the full reranked pipeline is already very strong and Jina indexes faster.

See [benchmarks.md](benchmarks.md) for the full benchmark context.

## Custom Local Embedding Models

You can point Vera at a different ONNX embedding model without changing the local reranker.

### Hugging Face Repo Or URL

```bash
vera setup --onnx-jina-cuda \
  --embedding-repo Zenabius/CodeRankEmbed-onnx \
  --embedding-pooling cls \
  --embedding-no-onnx-data \
  --embedding-query-prefix "Represent this query for searching relevant code:"
```

`--embedding-repo` also accepts a full Hugging Face URL such as `https://huggingface.co/Zenabius/CodeRankEmbed-onnx`.

### Local Directory

```bash
vera setup --onnx-jina-cuda \
  --embedding-dir /path/to/model-dir \
  --embedding-onnx-file onnx/model_quantized.onnx \
  --embedding-tokenizer-file tokenizer.json \
  --embedding-dim 768
```

Use this when you already downloaded or exported the model yourself.

## Flags

| Flag | Meaning |
| --- | --- |
| `--code-rank-embed` | Select the built-in CodeRankEmbed preset |
| `--embedding-repo <repo-or-url>` | Download a custom embedding model from Hugging Face |
| `--embedding-dir <dir>` | Use a local directory instead of downloading from Hugging Face |
| `--embedding-onnx-file <path>` | Relative path to the ONNX file inside the repo or directory |
| `--embedding-onnx-data-file <path>` | Relative path to an ONNX external data file |
| `--embedding-no-onnx-data` | Use models that do not ship an external data file |
| `--embedding-tokenizer-file <path>` | Relative path to the tokenizer file |
| `--embedding-dim <n>` | Embedding dimension stored in the index |
| `--embedding-pooling mean|cls` | Pooling method for token-level outputs |
| `--embedding-max-length <n>` | Tokenizer truncation length |
| `--embedding-query-prefix <text>` | Optional prefix prepended to local embedding queries |

## Required Files

For a custom embedding model, Vera needs:

- an ONNX model file
- a tokenizer file
- optionally an ONNX external data file

The defaults are:

| Asset | Default path |
| --- | --- |
| ONNX model | `onnx/model_quantized.onnx` |
| ONNX external data | `onnx/model_quantized.onnx_data` |
| Tokenizer | `tokenizer.json` |

If your model uses different names, pass the matching `--embedding-*` flags.

## Inference Speed

GPU is recommended; CPU works but is slow for initial indexing. After the first index, `vera update .` only re-embeds changed files, so updates are fast even on CPU.

| Backend | Hardware | Time | Notes |
|---------|----------|------|-------|
| CUDA | RTX 4080 | **~8 s** | Recommended for large repos |
| API mode | Remote GPU | ~56 s | Requires API key, no local compute |
| CPU | Ryzen 5 7600X3D (6c/12t) | ~6 min | Use GPU or API mode if this is too slow |

## API Mode

```bash
export EMBEDDING_MODEL_BASE_URL=https://your-embedding-api/v1
export EMBEDDING_MODEL_ID=your-embedding-model
export EMBEDDING_MODEL_API_KEY=your-api-key

# Optional reranker
export RERANKER_MODEL_BASE_URL=https://your-reranker-api/v1
export RERANKER_MODEL_ID=your-reranker-model
export RERANKER_MODEL_API_KEY=your-api-key

vera setup --api
```

Only model calls leave your machine. Indexing, storage, and search remain local.

## Notes

- These options only affect local embeddings. API mode is unchanged.
- Query prefixes only apply to local embedding queries, not API embeddings.
- If you switch local embedding models, re-index the repo so the stored vectors match the active model.
- If your network blocks CLI downloads, use [manual-install.md](manual-install.md).

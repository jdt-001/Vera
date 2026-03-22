# Environment

Environment variables, external dependencies, and setup notes.

**What belongs here:** Required env vars, external API keys/services, dependency quirks, platform-specific notes.
**What does NOT belong here:** Service ports/commands (use `.factory/services.yaml`).

---

## API Credentials

Loaded from `secrets.env` (gitignored, never commit):
- `EMBEDDING_MODEL_BASE_URL` — OpenAI-compatible embedding endpoint
- `EMBEDDING_MODEL_ID` — e.g., `Qwen/Qwen3-Embedding-8B`
- `EMBEDDING_MODEL_API_KEY`
- `RERANKER_MODEL_BASE_URL` — OpenAI-compatible reranker endpoint
- `RERANKER_MODEL_ID` — e.g., `Qwen/Qwen3-Reranker`
- `RERANKER_MODEL_API_KEY`

## Local Inference

- Models stored in `~/.vera/models/` (global, reused across projects)
- Downloads quantized ONNX only (`model_quantized.onnx`)
- Embedding: jina-embeddings-v5-text-nano-retrieval (239M params)
- Reranking: jina-reranker-v2-base-multilingual (278M params)
- Activated by `--local` flag or `VERA_LOCAL=1` env var
- Requires an ONNX Runtime shared library at runtime; if auto-detection fails, set `ORT_DYLIB_PATH` to the library path
- API mode does not need ONNX Runtime installed

## Build Requirements

- Rust 1.85+ (project uses edition 2024)
- C compiler for tree-sitter grammars (cc crate) and bundled SQLite
- API mode has no other external runtime dependencies

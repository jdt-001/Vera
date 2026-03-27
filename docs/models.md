# Local Models

`vera setup` downloads two quantized ONNX models into `~/.vera/models/` and the ONNX Runtime shared library into `~/.vera/lib/`. No manual install required.

## Embedding Model

[`jinaai/jina-embeddings-v5-text-nano-retrieval`](https://huggingface.co/jinaai/jina-embeddings-v5-text-nano-retrieval) (239M parameters, quantized ONNX)

The highest scoring embedding model under 500M parameters on MMTEB (65.5), beating KaLM-mini-v2.5 (494M), Gemma-300M (308M), and voyage-4-nano (480M). Scores 71.0 on MTEB English v2. Built on EuroBERT-210M with distillation from Qwen3-Embedding-4B, it uses a retrieval-specific LoRA adapter designed for asymmetric search where the query is short and the document is a code block.

## Reranker Model

[`jinaai/jina-reranker-v2-base-multilingual`](https://huggingface.co/jinaai/jina-reranker-v2-base-multilingual) (278M parameters, quantized ONNX)

A cross-encoder that scores query-document pairs jointly rather than comparing pre-computed embeddings. Its 1,024-token context window is a natural fit for Vera's tree-sitter symbol chunks (discrete functions and classes, not raw files). Fine-tuned on ToolBench (function-calling schemas) and NSText2SQL (structured queries), it scores 71.36 on CodeSearchNet MRR@10 and 77.75 on ToolBench recall@3 while being half the size and 15x faster than bge-reranker-v2-m3 (568M).

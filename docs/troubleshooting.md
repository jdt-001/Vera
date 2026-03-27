# Troubleshooting

## "No index found in current directory"

Either the repository hasn't been indexed yet, or you're running the command from the wrong directory.

```bash
vera index .
```

Make sure you're in the repository root (the directory containing `.vera/`).

## Results feel stale or outdated

Code changed after the last index. Update it:

```bash
vera update .
```

## Local ONNX inference isn't working

Run the diagnostic command first:

```bash
vera doctor
vera doctor --probe
```

Common causes:

- Models haven't been downloaded yet. run `vera setup` (or `vera setup --onnx-jina-cpu`)
- ONNX Runtime auto-download failed. check network, or set `ORT_DYLIB_PATH` to a manually installed library
- GPU backend not working. make sure the required drivers are installed (CUDA 12+ for `--onnx-jina-cuda`, ROCm for `--onnx-jina-rocm`, DirectX 12 for `--onnx-jina-directml`). CoreML (`--onnx-jina-coreml`) requires macOS on Apple Silicon. OpenVINO (`--onnx-jina-openvino`) requires Intel compute runtime on Linux x86_64. If GPU init still fails, rerun with `--onnx-jina-cpu` or fix the provider-specific dependencies.
- `vera doctor` will flag missing models or runtime, show the saved and active backend, print the installed Vera version, and check for newer releases. `vera doctor --probe` adds a deeper read-only session probe and does not download or repair missing assets.

## API mode isn't working

Check that all three environment variables are set:

- `EMBEDDING_MODEL_BASE_URL`
- `EMBEDDING_MODEL_ID`
- `EMBEDDING_MODEL_API_KEY`

If you're using a reranker, its three variables (`RERANKER_MODEL_BASE_URL`, `RERANKER_MODEL_ID`, `RERANKER_MODEL_API_KEY`) must either all be set or all be absent. Partial configuration will fail.

Re-run setup to persist a working configuration:

```bash
vera setup --api
```

## Too many irrelevant results

Try narrowing your search:

- `--lang rust`: filter by language
- `--path "src/**/*.ts"`: filter by file path
- `--type function`: filter by symbol type
- `--limit 5`: return fewer results
- Rewrite the query to be more specific about the behavior you're looking for

See the [query guide](query-guide.md) for more tips on writing effective queries.

## Need an exact text match?

Vera is a semantic search tool. For exact string or regex matching, use `rg` (ripgrep) instead:

```bash
rg "EMBEDDING_MODEL_BASE_URL"
rg "TODO\(" -n
```

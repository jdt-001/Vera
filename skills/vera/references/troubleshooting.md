# Troubleshooting

## `no index found in current directory`

Cause:

- the repository has not been indexed yet
- the command is running from the wrong directory

Fix:

```sh
vera index .
```

Or run from the repository root that contains `.vera/`.

## Results Are Stale

Cause:

- code changed after the last index

Fix:

```sh
vera update .
```

## Local Mode Fails

Check:

```sh
vera doctor
```

Common causes:

- ONNX Runtime auto-download failed (check network, or set `ORT_DYLIB_PATH`)
- local model assets have not been downloaded yet

Helpful commands:

```sh
vera setup
vera doctor
```

## API Mode Fails

Check:

- `EMBEDDING_MODEL_BASE_URL`
- `EMBEDDING_MODEL_ID`
- `EMBEDDING_MODEL_API_KEY`

Optional reranker values must either all be present or all be absent.

Persist a working shell configuration with:

```sh
vera setup --api
```

## Too Much Noise

Try one of these:

- add `--lang`
- add `--path`
- add `--type`
- reduce `--limit`
- rewrite the query to describe behavior, not just a vague topic

## Exact Match Requested

Do not force Vera for exact text search. Use `rg`.

# Install And First Run

Vera is designed to be used through the CLI, with the Vera skill installed into the user's coding agent.

## Contents

- Preferred Flow
- Local Mode
- API Mode
- Agent Install Locations
- Diagnostics

## Preferred Flow

1. Install the Vera skill:

```sh
vera agent install
```

2. Configure Vera:

```sh
vera setup --local
```

3. Index the current repository:

```sh
vera index .
```

4. Search:

```sh
vera search "authentication logic"
```

## Local Mode

Local mode is the default setup path.

```sh
vera setup --local
```

Notes:

- Vera downloads local model files into `~/.vera/models/`
- ONNX Runtime still needs to be available for local inference
- `vera doctor` will tell you if runtime setup is incomplete

You can configure and index in one step:

```sh
vera setup --local --index .
```

## API Mode

Use API mode only when the environment already has working embedding credentials or the user explicitly wants that setup.

Set these first:

```sh
export EMBEDDING_MODEL_BASE_URL=https://your-embedding-api/v1
export EMBEDDING_MODEL_ID=your-embedding-model
export EMBEDDING_MODEL_API_KEY=your-api-key
```

Optional reranker:

```sh
export RERANKER_MODEL_BASE_URL=https://your-reranker-api/v1
export RERANKER_MODEL_ID=your-reranker-model
export RERANKER_MODEL_API_KEY=your-api-key
```

Then persist them:

```sh
vera setup --api
```

## Agent Install Locations

`vera agent install` writes the canonical `vera` skill into known skill directories for:

- Claude Code
- Codex
- GitHub Copilot CLI
- Cursor
- Kiro

Useful variants:

```sh
vera agent install --client codex
vera agent install --scope project
vera agent status --scope all
vera agent remove --client claude
```

## Diagnostics

Use these when setup fails or results look wrong:

```sh
vera doctor
vera config
vera stats
```

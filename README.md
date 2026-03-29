<div align="center">

<img width="1584" height="539" alt="vera" src="https://github.com/user-attachments/assets/c866fc70-b1e6-400b-aaf7-fa68721a4955" />

# Vera

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/lemon07r/Vera/blob/master/LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)
[![GitHub release](https://img.shields.io/github/v/release/lemon07r/Vera?include_prereleases&sort=semver)](https://github.com/lemon07r/Vera/releases)
[![Languages](https://img.shields.io/badge/languages-63%2B-green.svg)](docs/supported-languages.md)

[Features](docs/features.md)
·
[Query Guide](docs/query-guide.md)
·
[Benchmarks](docs/benchmarks.md)
·
[How It Works](docs/how-it-works.md)
·
[Models](docs/models.md)
·
[Manual Install](docs/manual-install.md)
·
[Docker](docs/docker.md)
·
[Supported Languages](docs/supported-languages.md)
·
[Troubleshooting](docs/troubleshooting.md)

**V**ector **E**nhanced **R**eranking **A**gent

Vera is a code search tool built in Rust that combines BM25 keyword matching, vector similarity, and cross-encoder reranking into a single retrieval pipeline. It parses 60+ languages with tree-sitter, runs everything locally, and returns structured JSON with file paths, line ranges, symbol metadata, and relevance scores.

</div>

<h2></h2>

After trying a lot of other tools and maintaining Pampax, a fork of someone's code search tool, I ran into constant issues. The upstream project was hastily thrown together with deep-rooted bugs. Despite significantly improving Pampax over time, I kept running into new issues and nothing supported all the things I wanted (like provider-agnostic reranking), so I wanted to build something better from scratch. Every design choice in Vera (the retrieval pipeline, the model selection, the output format) comes from months of research, real benchmarking and evaluation.

## Quick Start

```bash
bunx @vera-ai/cli install   # or: npx -y @vera-ai/cli install / uvx vera-ai install
vera setup                   # downloads local models, no API needed
vera index .
vera search "authentication logic"
```

## Why Vera is Better

**Cross-encoder reranking.** Most code search tools retrieve candidates and stop. Vera adds a reranking stage that reads query and candidate as a single pair, scoring relevance jointly instead of comparing pre-computed vectors. Result: 0.60 MRR@10 vs. 0.28 with vector retrieval alone.

**Zero-dependency, single binary.** One static binary with 60+ tree-sitter grammars compiled in. No Python, no language servers, no per-language toolchains. Drop it on any machine, run `vera setup`, done. Compare: Serena requires Python, uv, and separate LSP installs per language.

**Higher accuracy, proven on real codebases.** Vera scores 0.80 nDCG@10 and 0.70 Recall@5 on a 17-task benchmark across `ripgrep`, `flask`, and `fastify`. The current 21-task suite scores even higher. See [Benchmark Snapshot](#benchmark-snapshot).

**Token-efficient output.** Returns only the relevant chunks, not entire files. On a 20-query benchmark against its own codebase, Vera's output averages 67% fewer tokens than loading the full files containing the same results. Most queries see 75-95% reduction; only queries hitting small config files break even. Default markdown codeblock format cuts an additional ~35-40% vs JSON. Ships with skill files that teach AI agents how to write effective queries and when to reach for `rg` instead.

## Features

**Model-agnostic, local-first.** Point Vera at any OpenAI-compatible embedding or reranker endpoint, remote or local. Run `vera setup` to download two curated ONNX models and run the full pipeline offline.

**Tree-sitter structural parsing.** 63 language grammars extract functions, classes, methods, and structs as discrete chunks. Results map to actual symbol boundaries, not arbitrary line ranges.

**Structured, code-aware results.** Every result includes file path, line range, source content, symbol name, and type. Agents and scripts consume this directly without parsing.

**Multi-query, intent-aware search.** Send multiple queries in one call, add an intent parameter for reranking, follow symbol chains with `--deep`, and filter by language, path, type, or corpus scope.

**Code intelligence.** Call graph traversal (`vera references`), dead code detection (`vera dead-code`), project overview with convention detection (`vera overview`), and regex search (`vera grep`).

**31 agent clients.** Skill files for Junie, Claude Code, Cursor, Windsurf, Copilot, Cline, Roo Code, and more. Interactive install, auto-sync on upgrade.

Full feature list: [docs/features.md](docs/features.md).

## Installation

```bash
bunx @vera-ai/cli install   # or: npx -y @vera-ai/cli install / uvx vera-ai install
```

This downloads the `vera` binary, writes a `vera` shim into a user bin directory, and installs the global agent skill files. If that bin directory is not already on `PATH`, Vera tells you what to add. After that, `vera` is a standalone command. You don't need `bunx`/`npx`/`uvx` again.

```bash
vera setup                          # interactive wizard: backend + agent skills + optional indexing
vera index .                        # index the current project (creates .vera/ in project root)
vera search "query"                 # search (each project gets its own index)
vera update .                       # after code changes
```

`vera setup` with no flags runs an interactive wizard that walks through backend selection, agent skill installation, and optional project indexing. You can skip the wizard with flags: `vera setup --onnx-jina-cuda` (NVIDIA), `--onnx-jina-coreml` (Apple Silicon), `--api` (remote endpoints), etc. Add `--code-rank-embed` for the optional CodeRankEmbed local embedding preset.

For focused changes after initial setup: `vera backend` manages the ONNX runtime and model backend, `vera agent install` manages skill files (detects existing installs, lets you add or remove agents in one step), and `vera agent sync` refreshes all stale skill installs to match the current binary version. Run `vera setup --help` for all options.

Use `vera doctor` if anything goes wrong. It reports the saved and active backend, installed Vera version, and checks GitHub for newer releases. Add `--probe` for a deeper read-only ONNX session check that does not download or repair missing assets. Use `vera repair` to re-fetch missing local assets or re-save API config from the current environment. Use `vera upgrade` to inspect or apply the binary update plan.

If your network blocks CLI downloads and only allows browser downloads, use the short [manual install guide](docs/manual-install.md).

<details>
<summary>MCP server (JSON-RPC over stdio)</summary>

```bash
vera mcp   # or: bunx @vera-ai/cli mcp / uvx vera-ai mcp
```

Exposes `search_code`, `index_project`, `update_project`, `get_stats`, `get_overview`, `watch_project`, `find_references`, `find_dead_code`, and `regex_search` tools.

</details>

<details>
<summary>Docker (MCP server)</summary>

```bash
docker run --rm -i -v $(pwd):/workspace ghcr.io/lemon07r/vera:cpu
```

CPU, CUDA (NVIDIA), ROCm (AMD), and OpenVINO (Intel) images available. See [docs/docker.md](docs/docker.md) for GPU flags and MCP client configuration.

</details>

<details>
<summary>Prebuilt binaries</summary>

Download from [GitHub Releases](https://github.com/lemon07r/Vera/releases) for Linux (x86_64, aarch64), macOS (x86_64, aarch64), or Windows (x86_64).

```bash
curl -sL https://github.com/lemon07r/Vera/releases/latest/download/vera-x86_64-unknown-linux-gnu.tar.gz | tar xz
cp vera-x86_64-unknown-linux-gnu/vera ~/.local/bin/
vera setup
```

</details>

<details>
<summary>Build from source</summary>

Rust 1.85+ required.

```bash
git clone https://github.com/lemon07r/Vera.git && cd Vera
cargo build --release
cp target/release/vera ~/.local/bin/
vera setup
```

</details>

### Updating

Vera checks for new releases once per day and prints a hint when a newer release is available. To inspect the current update plan:

```bash
vera upgrade
vera upgrade --apply
```

`vera upgrade` is a dry run by default. It shows the detected install method and the exact command Vera would run. `--apply` only runs when Vera can determine a single install method. After an upgrade, Vera automatically syncs any stale agent skill installs to match the new binary version.

You can still update manually:

```bash
bunx @vera-ai/cli install   # re-downloads latest binary + refreshes skill files
```

Set `VERA_NO_UPDATE_CHECK=1` to disable the automatic check.

## Model Backend

Vera itself is always local: the index lives in `.vera/`, config in `~/.vera/`. The backend choice only affects where embeddings and reranking run.

`vera setup` downloads two curated ONNX models and auto-detects your GPU. You can also specify a backend directly: `--onnx-jina-cuda` (NVIDIA), `--onnx-jina-rocm` (AMD), `--onnx-jina-directml` (Windows), `--onnx-jina-coreml` (Apple Silicon), `--onnx-jina-openvino` (Intel). Use `vera setup --api` to point at any OpenAI-compatible endpoint instead. Only model calls leave your machine.

GPU is recommended; CPU works but is slow for initial indexing. After the first index, `vera update .` only re-embeds changed files, so updates are fast even on CPU.

Full details on models, GPU backends, inference speed, custom embeddings, and API mode: [docs/models.md](docs/models.md). Feature overview: [docs/features.md](docs/features.md#model-backend).

## Usage

```bash
vera search "authentication logic"
vera search "error handling" --lang rust
vera search "routes" --path "src/**/*.ts"
vera search "handler" --type function --limit 5
vera search "config loading" --deep              # multi-hop: follows symbols from initial results
```

See the [query guide](docs/query-guide.md) for tips on writing effective queries and when to use `rg` instead.

Update the index after code changes: `vera update .`

For long sessions, start the file watcher to keep the index fresh automatically:

```bash
vera watch .
```

This watches for file changes and triggers incremental index updates (debounced at 2s). Runs until you press Ctrl-C.

### Excluding Files

Vera respects `.gitignore` by default. Create a `.veraignore` file (gitignore syntax) for more control, or use `--exclude` flags for one-off exclusions. Details: [docs/features.md](docs/features.md#flexible-exclusions).

### Output Format

Defaults to markdown codeblocks (the most token-efficient format for AI agents):

````
```src/auth/login.rs:42-68 function:authenticate
pub fn authenticate(credentials: &Credentials) -> Result<Token> { ... }
```
````

| Flag | Output |
|------|--------|
| `--json` | Compact single-line JSON |
| `--raw` | Verbose human-readable output |
| `--timing` | Per-stage pipeline durations to stderr |

### Other Commands

```bash
vera grep "fn\s+main"              # regex search over indexed files
vera grep "TODO|FIXME" -i          # case-insensitive regex
vera doctor                    # diagnose setup issues
vera doctor --probe            # deeper read-only ONNX probe
vera doctor --probe --json     # machine-readable deep diagnostics
vera repair                    # re-fetch missing assets for current backend
vera upgrade                   # inspect the binary update plan
vera upgrade --apply           # run it when the install method is known
vera watch .                   # auto-update index on file changes (Ctrl-C to stop)
vera stats                     # index statistics
vera overview                  # project summary (languages, entry points, hotspots)
vera references foo            # find all callers of symbol 'foo'
vera references foo --callees  # find what 'foo' calls
vera dead-code                 # find functions with no callers
vera config                    # show current configuration
vera agent install             # interactive: choose scope + agents
vera agent install --client all  # non-interactive: all agents, global
vera agent status              # check skill installation status
vera agent remove              # interactive: pick installs to remove
```

### Uninstalling

```bash
vera uninstall
```

Removes `~/.vera/` (binary, models, ONNX Runtime libs, config), agent skill files, and the PATH shim. Per-project indexes (`.vera/` in each project) are left in place.

If something isn't working, see [troubleshooting](docs/troubleshooting.md).

## Benchmarks

21-task benchmark across `ripgrep`, `flask`, `fastify`, and `turborepo`:

| Metric | ripgrep | cocoindex | ColGREP (149M) | Vera |
|--------|---------|-----------|----------------|------|
| Recall@5 | 0.28 | 0.37 | 0.67 | **0.78** |
| MRR@10 | 0.26 | 0.35 | 0.62 | **0.91** |
| nDCG@10 | 0.29 | 0.52 | 0.56 | **0.84** |

Full methodology, version history, and additional comparisons: [docs/benchmarks.md](docs/benchmarks.md).

## Supported Languages

63 languages and file formats with tree-sitter symbol extraction, plus text chunking for data formats. Full list: [docs/supported-languages.md](docs/supported-languages.md).

## How It Works

BM25 keyword search and vector similarity run in parallel, merge via Reciprocal Rank Fusion, then a cross-encoder reranks the top candidates. Full breakdown: [docs/how-it-works.md](docs/how-it-works.md).

## Configure Your AI Agent

`vera agent install` installs the Vera skill for your coding agents and offers to add a usage snippet to your project's `AGENTS.md` (or `CLAUDE.md`, `.cursorrules`, etc.). This gives agents persistent context about Vera so they use it automatically.

If you skipped the prompt or want to add it manually, put this in your project's agent config file:

```markdown
## Code Search

This project is indexed with Vera. Use `vera search "query"` for semantic code search
and `vera grep "pattern"` for regex search. Run `vera update .` after code changes.
For query tips and output format details, see the Vera skill in your skills directory.
```

Why this matters: agents always read `AGENTS.md` but don't reliably discover installed skills on their own. A 3-5 line block is enough to tell the agent Vera exists and when to use it. Keep it short; don't paste the full skill definition here.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for build instructions, project layout, how to add a language, and coding conventions.

# Agent Usage

Vera is a semantic code search CLI. For the full skill definition, see [`skills/vera/SKILL.md`](skills/vera/SKILL.md).

## Quick Start

```bash
npx -y @vera-ai/cli install   # install binary
vera index .                    # index the repo (add .vera/ to .gitignore)
vera search "query" --json      # search — returns ranked JSON capsules
vera update .                   # after code changes
```

## When to Use

- **Vera**: semantic search, symbol lookup, cross-file discovery, ranked context
- **rg**: exact text, regex, bulk find-and-replace

## Output

`--json` returns capsules with `file_path`, `line_start`, `line_end`, `content`, `language`, `score`, and optional `symbol_name`/`symbol_type`. Use highest-ranked results first.

## References

Query tips, troubleshooting, MCP setup, and install details are in [`skills/vera/SKILL.md`](skills/vera/SKILL.md) and its `references/` subdirectory.

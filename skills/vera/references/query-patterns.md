# Query Patterns

## Good Vera Queries

```sh
vera search "authentication middleware"
vera search "JWT token validation"
vera search "parse_config"
vera search "request rate limiting" --lang rust
vera search "routes" --path "src/**/*.ts"
vera search "handler" --type function --limit 5
```

## Weak Vera Queries

Single generic words return noise:

- `vera search "code"`
- `vera search "utils"`
- `vera search "file"`

Fix: describe what the code *does*, not what it *is*.

## When To Use `vera references` Instead

For structural queries about call relationships, use `references` or `dead-code` instead of `search`:

```sh
vera references parse_config            # who calls parse_config?
vera references parse_config --callees  # what does parse_config call?
vera dead-code                          # functions with no callers
```

These query the call graph built during indexing (direct calls only, no dynamic dispatch).

## When To Use `rg` Instead

- Exact string: `rg "EMBEDDING_MODEL_BASE_URL"`
- Regex: `rg "TODO\\(" -n`
- File name search: `rg --files | rg "docker"`
- Counting occurrences
- Bulk find-and-replace prep

## Narrowing Results

Add one filter at a time:

1. `--lang rust`: restrict to a language
2. `--path "src/auth/**"`: restrict to a path glob
3. `--type function`: restrict to symbol type
4. `--limit 3`: fewer, higher-confidence results

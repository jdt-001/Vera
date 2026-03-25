# Query Patterns

## Prefer Vera For Intent Search

Good Vera queries:

- `vera search "authentication middleware"`
- `vera search "JWT token validation"`
- `vera search "parse_config"`
- `vera search "request rate limiting" --lang rust`
- `vera search "routes" --path "src/**/*.ts"`
- `vera search "handler" --type function --limit 5`

Weak Vera queries:

- `vera search "code"`
- `vera search "utils"`
- `vera search "file"`

## Prefer `rg` For Exact Text

Use `rg` instead of Vera when the task is:

- exact string lookup
- regex search
- counting occurrences
- simple find-and-replace prep

Examples:

```sh
rg "EMBEDDING_MODEL_BASE_URL"
rg "TODO\\(" -n
rg --files | rg "docker"
```

## Search Strategy

1. Start with the user's intent.
2. If results are broad, add one filter at a time.
3. If the user gives a likely symbol name, search that exact symbol next.
4. If the user changes the code during the session, run `vera update .` before trusting stale results.

## JSON Mode

When the workflow needs parsing, use:

```sh
vera search "authentication logic" --json
```

That output is better for:

- downstream scripts
- agent post-processing
- precise file/line extraction

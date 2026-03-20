//! Shared types and utilities for the storage spike benchmarks.

use rand::Rng;
use serde::{Deserialize, Serialize};

/// A test chunk simulating what Vera's indexer would produce.
#[derive(Clone, Debug)]
pub struct TestChunk {
    pub id: u64,
    pub file_path: String,
    pub symbol_name: String,
    pub symbol_type: String,
    pub language: String,
    pub content: String,
    pub line_start: u32,
    pub line_end: u32,
    pub vector: Vec<f32>,
}

/// Benchmark results for a single storage backend.
#[derive(Debug, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub backend: String,
    pub num_chunks: usize,
    pub vector_dim: usize,

    // Write metrics
    pub write_total_ms: f64,
    pub write_throughput_per_sec: f64,

    // Vector query metrics (optional, Tantivy doesn't do vector search)
    pub vector_query_p50_ms: Option<f64>,
    pub vector_query_p95_ms: Option<f64>,
    pub vector_query_p99_ms: Option<f64>,
    pub vector_query_mean_ms: Option<f64>,

    // BM25 query metrics (optional, sqlite-vec doesn't do BM25)
    pub bm25_query_p50_ms: Option<f64>,
    pub bm25_query_p95_ms: Option<f64>,
    pub bm25_query_p99_ms: Option<f64>,
    pub bm25_query_mean_ms: Option<f64>,

    // Storage
    pub storage_size_bytes: u64,

    // Metadata
    pub notes: Vec<String>,
}

/// Generate synthetic test chunks with random vectors.
pub fn generate_test_chunks(num: usize, vector_dim: usize) -> Vec<TestChunk> {
    let mut rng = rand::rng();
    let languages = ["rust", "python", "typescript", "go", "java"];
    let symbol_types = [
        "function",
        "struct",
        "class",
        "method",
        "trait",
        "interface",
    ];

    // Some realistic code snippets for BM25 testing
    let code_templates = [
        "fn {name}(ctx: &Context, request: Request) -> Result<Response> {{\n    let data = ctx.db.query(&request.params)?;\n    Ok(Response::new(data))\n}}",
        "async def {name}(self, request: dict) -> dict:\n    \"\"\"Handle incoming request and return response.\"\"\"\n    result = await self.db.execute(request['query'])\n    return {{'status': 'ok', 'data': result}}",
        "export function {name}(config: Config): Promise<Result> {{\n    const client = createClient(config);\n    return client.fetch(config.endpoint);\n}}",
        "func {name}(ctx context.Context, req *Request) (*Response, error) {{\n    result, err := db.QueryContext(ctx, req.Query)\n    if err != nil {{\n        return nil, fmt.Errorf(\"{name}: %w\", err)\n    }}\n    return &Response{{Data: result}}, nil\n}}",
        "pub struct {name} {{\n    inner: Arc<Mutex<Inner>>,\n    config: Config,\n    metrics: MetricsCollector,\n}}",
        "class {name}:\n    def __init__(self, config: dict):\n        self.config = config\n        self._cache = {{}}\n    \n    def process(self, data: list) -> list:\n        return [self._transform(item) for item in data]",
        "impl {name} {{\n    pub fn new(config: &Config) -> Self {{\n        Self {{\n            inner: Arc::new(Mutex::new(Inner::default())),\n            config: config.clone(),\n        }}\n    }}\n\n    pub fn execute(&self, query: &str) -> Result<Vec<Row>> {{\n        let guard = self.inner.lock().unwrap();\n        guard.run_query(query)\n    }}\n}}",
        "interface {name} {{\n    id: string;\n    name: string;\n    execute(params: Record<string, unknown>): Promise<void>;\n    validate(input: unknown): boolean;\n}}",
    ];

    let name_prefixes = [
        "handle",
        "process",
        "create",
        "update",
        "delete",
        "fetch",
        "validate",
        "transform",
        "parse",
        "serialize",
        "deserialize",
        "connect",
        "disconnect",
        "authenticate",
        "authorize",
        "cache",
        "index",
        "search",
        "filter",
        "sort",
        "aggregate",
        "compute",
        "render",
        "format",
        "encode",
        "decode",
        "compress",
        "decompress",
    ];

    let name_suffixes = [
        "Request",
        "Response",
        "Handler",
        "Manager",
        "Service",
        "Client",
        "Server",
        "Worker",
        "Queue",
        "Buffer",
        "Config",
        "Options",
        "Params",
        "Result",
        "Error",
        "Context",
        "Session",
        "Connection",
        "Pipeline",
        "Cache",
    ];

    (0..num)
        .map(|i| {
            let lang_idx = i % languages.len();
            let sym_idx = i % symbol_types.len();
            let prefix = name_prefixes[i % name_prefixes.len()];
            let suffix = name_suffixes[i % name_suffixes.len()];
            let name = format!("{prefix}_{suffix}_{i}");

            let template = code_templates[i % code_templates.len()];
            let content = template.replace("{name}", &name);

            let file_num = i / 10; // ~10 chunks per "file"
            let file_path = format!(
                "src/{}/{}/mod.rs",
                languages[lang_idx],
                format!("module_{file_num}")
            );

            // Random normalized vector
            let vector: Vec<f32> = (0..vector_dim)
                .map(|_| rng.random_range(-1.0f32..1.0f32))
                .collect();
            let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
            let vector: Vec<f32> = vector.iter().map(|x| x / norm).collect();

            TestChunk {
                id: i as u64,
                file_path,
                symbol_name: name,
                symbol_type: symbol_types[sym_idx].to_string(),
                language: languages[lang_idx].to_string(),
                content,
                line_start: (i * 20) as u32,
                line_end: (i * 20 + 15) as u32,
                vector,
            }
        })
        .collect()
}

/// Compute percentile from sorted latencies.
pub fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (p / 100.0 * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Get total size of a directory recursively.
pub fn dir_size(path: &std::path::Path) -> u64 {
    if path.is_file() {
        return std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    }
    walkdir(path)
}

fn walkdir(path: &std::path::Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                total += std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            } else if p.is_dir() {
                total += walkdir(&p);
            }
        }
    }
    total
}

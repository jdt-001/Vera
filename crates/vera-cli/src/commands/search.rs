//! `vera search <query>` — Search the indexed codebase.

use anyhow::bail;

use crate::helpers::{is_local_mode, load_runtime_config, output_results};

/// Run the `vera search <query>` command.
///
/// Performs hybrid search (BM25 + vector via RRF fusion) with optional
/// cross-encoder reranking. Falls back gracefully:
/// - Embedding API unavailable → BM25-only search with warning
/// - Reranker API unavailable → unreranked hybrid results with warning
pub fn run(
    query: &str,
    limit: Option<usize>,
    filters: &vera_core::types::SearchFilters,
    json_output: bool,
    local_flag: bool,
) -> anyhow::Result<()> {
    let config = load_runtime_config()?;
    let result_limit = limit.unwrap_or(config.retrieval.default_limit);

    // Find the index directory (look in current working directory).
    let cwd = std::env::current_dir()
        .map_err(|e| anyhow::anyhow!("failed to get current directory: {e}"))?;
    let index_dir = vera_core::indexing::index_dir(&cwd);

    if !index_dir.exists() {
        bail!(
            "no index found in current directory.\n\
             Hint: run `vera index <path>` first to create an index."
        );
    }

    let is_local = is_local_mode(local_flag);

    // Use the shared search service (handles hybrid/BM25 fallback internally).
    let results = vera_core::retrieval::search_service::execute_search(
        &index_dir,
        query,
        &config,
        filters,
        result_limit,
        is_local,
    )?;

    output_results(&results, json_output);
    Ok(())
}

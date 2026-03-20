//! Shared search service used by both CLI and MCP.
//!
//! Encapsulates the common hybrid search flow: create embedding provider,
//! build reranker, compute fetch limits, execute search, apply filters.

use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use tracing::warn;

use crate::config::VeraConfig;
use crate::embedding::{CachedEmbeddingProvider, EmbeddingProviderConfig, OpenAiProvider};
use crate::retrieval::{
    ApiReranker, RerankerConfig, apply_filters, search_bm25, search_hybrid, search_hybrid_reranked,
};
use crate::types::{SearchFilters, SearchResult};

/// Execute a search against the index at `index_dir`.
///
/// Attempts hybrid search (BM25 + vector + optional reranking). Falls
/// back to BM25-only when embedding API is unavailable.
pub fn execute_search(
    index_dir: &Path,
    query: &str,
    config: &VeraConfig,
    filters: &SearchFilters,
    result_limit: usize,
) -> Result<Vec<SearchResult>> {
    let fetch_limit = compute_fetch_limit(filters, result_limit);

    // Try to create embedding provider for hybrid search.
    let provider_config = match EmbeddingProviderConfig::from_env() {
        Ok(cfg) => cfg,
        Err(_) => {
            warn!("Embedding API not configured, using BM25-only search");
            let results = search_bm25(index_dir, query, fetch_limit)?;
            return Ok(apply_filters(results, filters, result_limit));
        }
    };

    let provider_config = provider_config
        .with_timeout(Duration::from_secs(config.embedding.timeout_secs))
        .with_max_retries(config.embedding.max_retries);

    let provider = match OpenAiProvider::new(provider_config) {
        Ok(p) => p,
        Err(_) => {
            warn!("Failed to create embedding provider, using BM25-only search");
            let results = search_bm25(index_dir, query, fetch_limit)?;
            return Ok(apply_filters(results, filters, result_limit));
        }
    };

    let provider = CachedEmbeddingProvider::new(provider, 512);

    // Create optional reranker.
    let reranker = create_reranker(config);

    let stored_dim = config.embedding.max_stored_dim;
    let rrf_k = config.retrieval.rrf_k;
    let rerank_candidates = config.retrieval.rerank_candidates;

    let rt = tokio::runtime::Runtime::new()?;
    let results = if let Some(ref reranker) = reranker {
        rt.block_on(search_hybrid_reranked(
            index_dir,
            &provider,
            reranker,
            query,
            fetch_limit,
            rrf_k,
            stored_dim,
            rerank_candidates.max(fetch_limit),
        ))?
    } else {
        rt.block_on(search_hybrid(
            index_dir,
            &provider,
            query,
            fetch_limit,
            rrf_k,
            stored_dim,
        ))?
    };

    Ok(apply_filters(results, filters, result_limit))
}

/// Create the optional reranker from environment configuration.
fn create_reranker(config: &VeraConfig) -> Option<ApiReranker> {
    if !config.retrieval.reranking_enabled {
        return None;
    }

    let reranker_config = RerankerConfig::from_env().ok()?;
    let reranker_config = reranker_config
        .with_timeout(Duration::from_secs(30))
        .with_max_retries(2);
    ApiReranker::new(reranker_config).ok()
}

/// Compute how many candidates to fetch before filtering.
///
/// When filters are active, fetch more candidates to ensure we have enough
/// results after filtering.
fn compute_fetch_limit(filters: &SearchFilters, result_limit: usize) -> usize {
    if filters.is_empty() {
        result_limit
    } else {
        result_limit.saturating_mul(3).max(result_limit + 20)
    }
}

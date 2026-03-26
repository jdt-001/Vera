//! Shared search service used by both CLI and MCP.
//!
//! Encapsulates the common hybrid search flow: create embedding provider,
//! build reranker, compute fetch limits, execute search, apply filters.

use std::path::Path;

use anyhow::Result;
use tracing::warn;

use crate::config::{InferenceBackend, VeraConfig};
use crate::retrieval::hybrid::compute_vector_candidates;
use crate::retrieval::query_classifier::{QueryType, classify_query, params_for_query_type};
use crate::retrieval::{apply_filters, search_bm25, search_hybrid, search_hybrid_reranked};
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
    backend: InferenceBackend,
) -> Result<Vec<SearchResult>> {
    let fetch_limit = compute_fetch_limit(filters, result_limit);
    let rt = tokio::runtime::Runtime::new()?;

    // Try to create embedding provider for hybrid search.
    let (provider, model_name) =
        match rt.block_on(crate::embedding::create_dynamic_provider(config, backend)) {
            Ok(res) => res,
            Err(e) => {
                if backend.is_local() {
                    anyhow::bail!("{}", e);
                }
                warn!(
                    "Failed to create embedding provider ({}), using BM25-only search",
                    e
                );
                let results = search_bm25(index_dir, query, fetch_limit)?;
                return Ok(apply_filters(results, filters, result_limit));
            }
        };

    let mut stored_dim = config.embedding.max_stored_dim;

    // Check metadata mismatch
    let metadata_path = index_dir.join("metadata.db");
    if let Ok(metadata_store) = crate::storage::metadata::MetadataStore::open(&metadata_path) {
        if let (Some(s_model), Some(s_dim)) = (
            metadata_store.get_index_meta("model_name").unwrap_or(None),
            metadata_store
                .get_index_meta("embedding_dim")
                .unwrap_or(None),
        ) {
            if s_model != model_name {
                anyhow::bail!(
                    "Index was created with model '{}' ({} dimensions), but you are using model '{}'. Please re-index with matching provider.",
                    s_model,
                    s_dim,
                    model_name
                );
            }
            if let Ok(dim) = s_dim.parse::<usize>() {
                use crate::embedding::EmbeddingProvider;
                if let Some(provider_dim) = provider.expected_dim() {
                    if provider_dim != dim {
                        anyhow::bail!(
                            "Dimension mismatch: index has {} dimensions but active provider expects {}. Please re-index with matching provider.",
                            dim,
                            provider_dim
                        );
                    }
                }
                stored_dim = dim;
            }
        }
    }

    let provider = crate::embedding::CachedEmbeddingProvider::new(provider, 512);

    // Create optional reranker.
    let reranker = rt
        .block_on(crate::retrieval::create_dynamic_reranker(config, backend))
        .unwrap_or_else(|e| {
            warn!("Failed to create reranker ({})", e);
            None
        });

    // Classify query to adapt fusion parameters.
    let query_type = classify_query(query);
    let query_params = params_for_query_type(query_type);
    let rrf_k = query_params.rrf_k;
    let vector_candidates = effective_vector_candidates(fetch_limit, query_params, query);
    let rerank_candidates =
        effective_rerank_candidates(config.retrieval.rerank_candidates, fetch_limit, query);
    let skip_reranker = should_skip_reranker(query, query_type);

    let results = if let Some(ref reranker) = reranker.filter(|_| !skip_reranker) {
        rt.block_on(search_hybrid_reranked(
            index_dir,
            &provider,
            reranker,
            query,
            fetch_limit,
            rrf_k,
            stored_dim,
            rerank_candidates.max(fetch_limit),
            vector_candidates,
        ))?
    } else {
        rt.block_on(search_hybrid(
            index_dir,
            &provider,
            query,
            fetch_limit,
            rrf_k,
            stored_dim,
            vector_candidates,
        ))?
    };

    Ok(apply_filters(results, filters, result_limit))
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

fn effective_vector_candidates(
    fetch_limit: usize,
    query_params: crate::retrieval::query_classifier::QueryParams,
    query: &str,
) -> usize {
    let mut candidates =
        compute_vector_candidates(fetch_limit, query_params.vector_candidate_multiplier);

    if needs_broader_candidate_pool(query, classify_query(query)) {
        candidates = candidates.max(fetch_limit.saturating_mul(6));
    }

    candidates
}

fn effective_rerank_candidates(base: usize, fetch_limit: usize, query: &str) -> usize {
    let mut candidates = base.max(fetch_limit);

    if needs_broader_candidate_pool(query, classify_query(query)) {
        candidates = candidates.max(fetch_limit.saturating_mul(2));
    }

    candidates
}

fn should_skip_reranker(query: &str, query_type: QueryType) -> bool {
    query_type == QueryType::Identifier && is_path_weighted_query(query)
}

fn needs_broader_candidate_pool(query: &str, query_type: QueryType) -> bool {
    if matches!(query_type, QueryType::NaturalLanguage) {
        return true;
    }

    let lower = query.trim().to_ascii_lowercase();
    [
        "implementations",
        "implementation",
        "registered",
        "registration",
        "mounted",
        "mounting",
        "configured",
        "configuration",
        "across",
        "schema",
        "validation",
        "route",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn is_path_weighted_query(query: &str) -> bool {
    let lower = query.trim().to_ascii_lowercase();
    [
        "cargo.toml",
        "pyproject.toml",
        "package.json",
        "tsconfig.json",
        "dockerfile",
        "makefile",
        "cmakelists.txt",
        "nginx.conf",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
        || [".toml", ".json", ".yaml", ".yml", ".ini", ".md", ".conf"]
            .iter()
            .any(|suffix| lower.contains(suffix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::metadata::MetadataStore;
    use tempfile::tempdir;

    #[test]
    fn test_dimension_mismatch_and_inference() {
        let dir = tempdir().unwrap();
        let index_dir = dir.path();

        let metadata_path = index_dir.join("metadata.db");
        let store = MetadataStore::open(&metadata_path).unwrap();

        // 1. Test dimension mismatch (requires local model so provider_dim is Some(768))
        store
            .set_index_meta("model_name", "jina-embeddings-v5-text-nano-retrieval")
            .unwrap();
        store.set_index_meta("embedding_dim", "1024").unwrap(); // Mismatch: 1024 vs 768

        let config = VeraConfig::default();
        let filters = SearchFilters::default();

        // This will attempt to create local provider and should fail at mismatch
        {
            let res = execute_search(
                index_dir,
                "test",
                &config,
                &filters,
                10,
                crate::config::InferenceBackend::OnnxJina(
                    crate::config::OnnxExecutionProvider::Cpu,
                ),
            );
            assert!(res.is_err());
            let err_msg = res.unwrap_err().to_string();
            // With load-dynamic ort, if ONNX Runtime is not present the error will be
            // about loading the runtime. If it IS present, it will be a dimension mismatch.
            // Either way the search correctly fails.
            assert!(
                err_msg.contains(
                    "Dimension mismatch: index has 1024 dimensions but active provider expects 768"
                ) || err_msg.contains("Failed to initialize local embedding provider"),
                "{}",
                err_msg
            );
        }

        // 2. Test metadata-dimension inference path (API provider returns None for expected_dim)
        // Set up dummy environment variables for API provider to bypass missing keys error
        unsafe {
            std::env::set_var("EMBEDDING_MODEL_BASE_URL", "http://127.0.0.1:0");
            std::env::set_var("EMBEDDING_MODEL_ID", "dummy-api-model");
            std::env::set_var("EMBEDDING_MODEL_API_KEY", "dummy-key");
        }

        store
            .set_index_meta("model_name", "dummy-api-model")
            .unwrap();
        store.set_index_meta("embedding_dim", "123").unwrap();

        // Calling execute_search with is_local = false
        // It will pass the metadata check (model_name matches), skip mismatch check (expected_dim is None),
        // infer stored_dim = 123, and proceed to search.
        // Since the index is empty, it will return Ok([]) without making network calls.
        let res = execute_search(
            index_dir,
            "test",
            &config,
            &filters,
            10,
            crate::config::InferenceBackend::Api,
        );
        assert!(res.is_ok(), "Expected Ok but got {:?}", res);
    }

    #[test]
    fn skips_reranker_for_filename_queries() {
        assert!(should_skip_reranker(
            "Cargo.toml workspace configuration",
            QueryType::Identifier
        ));
        assert!(!should_skip_reranker(
            "where is the workspace configured",
            QueryType::NaturalLanguage
        ));
    }

    #[test]
    fn broader_queries_expand_candidates() {
        assert!(effective_rerank_candidates(50, 10, "Sink trait and its implementations") >= 50);
        assert!(
            effective_vector_candidates(
                10,
                params_for_query_type(QueryType::NaturalLanguage),
                "request validation and schema enforcement"
            ) >= 60
        );
    }
}

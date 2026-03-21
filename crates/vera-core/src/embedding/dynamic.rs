use crate::config::VeraConfig;
#[cfg(feature = "local")]
use crate::embedding::local_provider::LocalEmbeddingProvider;
use crate::embedding::provider::{
    EmbeddingError, EmbeddingProvider, EmbeddingProviderConfig, OpenAiProvider,
};
use std::time::Duration;

pub enum DynamicProvider {
    Api(OpenAiProvider),
    #[cfg(feature = "local")]
    Local(LocalEmbeddingProvider),
}

impl EmbeddingProvider for DynamicProvider {
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        match self {
            Self::Api(p) => p.embed_batch(texts).await,
            #[cfg(feature = "local")]
            Self::Local(p) => p.embed_batch(texts).await,
        }
    }

    fn expected_dim(&self) -> Option<usize> {
        match self {
            Self::Api(p) => p.expected_dim(),
            #[cfg(feature = "local")]
            Self::Local(p) => p.expected_dim(),
        }
    }
}

pub async fn create_dynamic_provider(
    config: &VeraConfig,
    is_local: bool,
) -> anyhow::Result<(DynamicProvider, String)> {
    if is_local {
        #[cfg(feature = "local")]
        {
            let p = LocalEmbeddingProvider::new().await.map_err(|e| {
                anyhow::anyhow!("Failed to initialize local embedding provider: {e}")
            })?;
            Ok((
                DynamicProvider::Local(p),
                "jina-embeddings-v5-text-nano-retrieval".to_string(),
            ))
        }
        #[cfg(not(feature = "local"))]
        {
            anyhow::bail!(
                "Vera was compiled without the 'local' feature. Please recompile with --features local to use local models."
            );
        }
    } else {
        let provider_config = EmbeddingProviderConfig::from_env()
            .map_err(|err| anyhow::anyhow!("embedding API not configured: {err}\nHint: set EMBEDDING_MODEL_BASE_URL, EMBEDDING_MODEL_ID, and EMBEDDING_MODEL_API_KEY environment variables."))?;
        let model_name = provider_config.model_id.clone();
        let provider_config = provider_config
            .with_timeout(Duration::from_secs(config.embedding.timeout_secs))
            .with_max_retries(config.embedding.max_retries);
        let p = OpenAiProvider::new(provider_config)
            .map_err(|err| anyhow::anyhow!("failed to initialize embedding provider: {err}"))?;
        Ok((DynamicProvider::Api(p), model_name))
    }
}

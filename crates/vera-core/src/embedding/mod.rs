//! Embedding generation via external API providers.
//!
//! This module provides:
//! - [`EmbeddingProvider`] trait for abstracting embedding API calls
//! - [`OpenAiProvider`] for OpenAI-compatible embedding endpoints
//! - Batched embedding generation with configurable batch size
//! - Credential management (read from environment, never log)
//! - Error handling (auth failures, connection errors, rate limits)

mod provider;

pub use provider::{
    EmbeddingError, EmbeddingProvider, EmbeddingProviderConfig, OpenAiProvider, embed_chunks,
};

/// Test helpers for creating mock embedding providers.
#[cfg(test)]
pub(crate) mod test_helpers {
    pub use super::provider::test_helpers::MockProvider;
}

#[cfg(test)]
mod tests;

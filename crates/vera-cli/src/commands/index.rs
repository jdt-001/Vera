//! `vera index <path>` — Index a codebase for search.

use std::path::Path;

use anyhow::{Context, bail};

use crate::helpers::{create_embedding_provider, print_human_summary};

/// Run the `vera index <path>` command.
pub fn run(path: &str, json_output: bool) -> anyhow::Result<()> {
    let repo_path = Path::new(path);

    // Validate path early — before requiring API credentials.
    if !repo_path.exists() {
        bail!(
            "path does not exist: {path}\n\
             Hint: check the path and try again."
        );
    }
    if !repo_path.is_dir() {
        bail!(
            "path is not a directory: {path}\n\
             Hint: vera index expects a directory path, not a file."
        );
    }

    // Build the tokio runtime for async embedding calls.
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| anyhow::anyhow!("failed to create async runtime: {e}"))?;

    let config = vera_core::config::VeraConfig::default();

    // Create the embedding provider from environment.
    let provider = create_embedding_provider(&config)?;

    // Run the indexing pipeline.
    let summary = rt
        .block_on(vera_core::indexing::index_repository(
            repo_path, &provider, &config,
        ))
        .context("indexing failed")?;

    // Output results.
    if json_output {
        let json = serde_json::to_string_pretty(&summary)
            .map_err(|e| anyhow::anyhow!("failed to serialize summary: {e}"))?;
        println!("{json}");
    } else {
        print_human_summary(&summary);
    }

    Ok(())
}

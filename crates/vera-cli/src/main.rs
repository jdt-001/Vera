//! Vera CLI — code indexing and retrieval for AI coding agents.
//!
//! Usage:
//!   vera index <path>    Index a codebase
//!   vera search <query>  Search the index
//!   vera update <path>   Incrementally update the index
//!   vera stats            Show index statistics

use std::path::Path;
use std::process;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "vera",
    about = "Evidence-backed code indexing & retrieval for AI coding agents",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output results as JSON (machine-readable).
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Index a codebase for search.
    Index {
        /// Path to the directory to index.
        path: String,
    },

    /// Search the indexed codebase.
    Search {
        /// The search query.
        query: String,

        /// Filter by programming language (case-insensitive).
        #[arg(long)]
        lang: Option<String>,

        /// Filter by file path glob pattern.
        #[arg(long)]
        path: Option<String>,

        /// Maximum number of results to return.
        #[arg(long, short = 'n')]
        limit: Option<usize>,
    },

    /// Incrementally update the index for changed files.
    Update {
        /// Path to the directory to update.
        path: String,
    },

    /// Show index statistics.
    Stats,
}

fn main() -> anyhow::Result<()> {
    // Initialize tracing subscriber (logs go to stderr).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("VERA_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Index { path } => {
            tracing::info!(path = %path, "indexing");
            run_index(&path, cli.json)?;
        }
        Commands::Search {
            query,
            lang,
            path,
            limit,
        } => {
            tracing::info!(query = %query, "searching");
            let _ = (lang, path, limit); // Will be used in implementation
            eprintln!("vera search: not yet implemented (query: {query})");
        }
        Commands::Update { path } => {
            tracing::info!(path = %path, "updating");
            eprintln!("vera update: not yet implemented (path: {path})");
        }
        Commands::Stats => {
            tracing::info!("showing stats");
            eprintln!("vera stats: not yet implemented");
        }
    }

    Ok(())
}

/// Run the `vera index <path>` command.
fn run_index(path: &str, json_output: bool) -> anyhow::Result<()> {
    let repo_path = Path::new(path);

    // Validate path early — before requiring API credentials.
    if !repo_path.exists() {
        eprintln!("Error: path does not exist: {path}");
        process::exit(1);
    }
    if !repo_path.is_dir() {
        eprintln!("Error: path is not a directory: {path}");
        process::exit(1);
    }

    // Build the tokio runtime for async embedding calls.
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| anyhow::anyhow!("failed to create async runtime: {e}"))?;

    let config = vera_core::config::VeraConfig::default();

    // Create the embedding provider from environment.
    let provider_config = match vera_core::embedding::EmbeddingProviderConfig::from_env() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!(
                "Error: embedding API not configured: {err}\n\
                 Set EMBEDDING_MODEL_BASE_URL, EMBEDDING_MODEL_ID, and \
                 EMBEDDING_MODEL_API_KEY environment variables."
            );
            process::exit(1);
        }
    };
    let provider_config = provider_config
        .with_timeout(std::time::Duration::from_secs(
            config.embedding.timeout_secs,
        ))
        .with_max_retries(config.embedding.max_retries);

    let provider = match vera_core::embedding::OpenAiProvider::new(provider_config) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("Error: failed to initialize embedding provider: {err}");
            process::exit(1);
        }
    };

    // Run the indexing pipeline.
    let summary = match rt.block_on(vera_core::indexing::index_repository(
        repo_path, &provider, &config,
    )) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("Error: {err:#}");
            process::exit(1);
        }
    };

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

/// Print a human-readable summary of the indexing run.
fn print_human_summary(summary: &vera_core::indexing::IndexSummary) {
    println!("Indexing complete!");
    println!();
    println!("  Files parsed:        {}", summary.files_parsed);
    println!("  Chunks created:      {}", summary.chunks_created);
    println!("  Embeddings generated: {}", summary.embeddings_generated);
    println!("  Elapsed time:        {:.2}s", summary.elapsed_secs);

    // Report skipped files if any.
    let skipped_total = summary.binary_skipped + summary.large_skipped + summary.error_skipped;
    if skipped_total > 0 {
        println!();
        println!("  Skipped files:");
        if summary.binary_skipped > 0 {
            println!("    Binary:     {}", summary.binary_skipped);
        }
        if summary.large_skipped > 0 {
            println!("    Too large:  {}", summary.large_skipped);
        }
        if summary.error_skipped > 0 {
            println!("    Read errors: {}", summary.error_skipped);
        }
    }

    // Report parse errors if any.
    if !summary.parse_errors.is_empty() {
        println!();
        println!("  Parse errors ({}):", summary.parse_errors.len());
        for err in &summary.parse_errors {
            println!("    {}: {}", err.file_path, err.error);
        }
    }

    // Special message for empty repos.
    if summary.files_parsed == 0 && summary.chunks_created == 0 {
        println!();
        println!("  No source files found to index.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_index_command() {
        let cli = Cli::parse_from(["vera", "index", "/tmp/repo"]);
        assert!(matches!(cli.command, Commands::Index { path } if path == "/tmp/repo"));
    }

    #[test]
    fn cli_parses_search_command() {
        let cli = Cli::parse_from(["vera", "search", "find auth"]);
        assert!(matches!(cli.command, Commands::Search { query, .. } if query == "find auth"));
    }

    #[test]
    fn cli_parses_search_with_filters() {
        let cli = Cli::parse_from([
            "vera",
            "search",
            "find auth",
            "--lang",
            "rust",
            "--limit",
            "5",
        ]);
        match cli.command {
            Commands::Search {
                query, lang, limit, ..
            } => {
                assert_eq!(query, "find auth");
                assert_eq!(lang, Some("rust".to_string()));
                assert_eq!(limit, Some(5));
            }
            _ => panic!("expected Search command"),
        }
    }

    #[test]
    fn cli_parses_update_command() {
        let cli = Cli::parse_from(["vera", "update", "/tmp/repo"]);
        assert!(matches!(cli.command, Commands::Update { path } if path == "/tmp/repo"));
    }

    #[test]
    fn cli_parses_stats_command() {
        let cli = Cli::parse_from(["vera", "stats"]);
        assert!(matches!(cli.command, Commands::Stats));
    }

    #[test]
    fn cli_parses_json_flag() {
        let cli = Cli::parse_from(["vera", "--json", "stats"]);
        assert!(cli.json);
    }
}

//! `vera setup` — persist a preferred Vera mode and bootstrap first-run state.

use anyhow::{Context, bail};
use serde::Serialize;

use crate::commands;
use crate::state::{self, ApiSetupInput};

#[derive(Debug, Serialize)]
struct SetupReport {
    mode: &'static str,
    config_path: String,
    credentials_path: String,
    models_prefetched: usize,
    onnx_runtime_ready: bool,
    indexed_path: Option<String>,
}

pub fn run(
    local: bool,
    api: bool,
    index_path: Option<String>,
    json_output: bool,
    yes: bool,
) -> anyhow::Result<()> {
    if local && api {
        bail!("setup mode conflict: choose either --local or --api");
    }

    let use_local = !api;
    if !yes && !confirm(use_local, index_path.as_deref())? {
        if !json_output {
            println!("Cancelled.");
        }
        return Ok(());
    }

    let mut models_prefetched = 0usize;
    let onnx_runtime_ready;

    if use_local {
        state::save_local_mode(true)?;
        state::apply_saved_env_force()?;

        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| anyhow::anyhow!("failed to create async runtime: {e}"))?;
        let prefetched = rt.block_on(vera_core::local_models::prefetch_default_local_models())?;
        models_prefetched = prefetched.len();
        onnx_runtime_ready = vera_core::local_models::ensure_ort_runtime(None).is_ok();
    } else {
        let embedding = read_required_api_env(
            "EMBEDDING_MODEL_BASE_URL",
            "EMBEDDING_MODEL_ID",
            "EMBEDDING_MODEL_API_KEY",
        )?;
        let reranker = read_optional_api_env(
            "RERANKER_MODEL_BASE_URL",
            "RERANKER_MODEL_ID",
            "RERANKER_MODEL_API_KEY",
        )?;
        state::save_api_setup(&embedding, reranker.as_ref())?;
        state::save_local_mode(false)?;
        state::apply_saved_env_force()?;
        onnx_runtime_ready = vera_core::local_models::ensure_ort_runtime(None).is_ok();
    }

    if let Some(path) = index_path.as_deref() {
        commands::index::execute(path, use_local)?;
    }

    let report = SetupReport {
        mode: if use_local { "local" } else { "api" },
        config_path: state::config_path()?.display().to_string(),
        credentials_path: state::credentials_path()?.display().to_string(),
        models_prefetched,
        onnx_runtime_ready,
        indexed_path: index_path,
    };

    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("Vera setup complete.");
        println!();
        println!("  Mode:                 {}", report.mode);
        println!("  Config:               {}", report.config_path);
        println!("  Credentials:          {}", report.credentials_path);
        if use_local {
            println!("  Prefetched model files: {}", report.models_prefetched);
        }
        println!(
            "  ONNX Runtime ready:   {}",
            if report.onnx_runtime_ready {
                "yes"
            } else {
                "no"
            }
        );
        if let Some(path) = report.indexed_path.as_deref() {
            println!("  Indexed path:         {path}");
        }
    }

    Ok(())
}

fn confirm(use_local: bool, index_path: Option<&str>) -> anyhow::Result<bool> {
    let mode = if use_local { "local" } else { "api" };
    println!("This will configure Vera for {mode} mode.");
    if let Some(path) = index_path {
        println!("It will also index: {path}");
    }
    print!("Continue? [y/N]: ");
    let mut stdout = std::io::stdout();
    std::io::Write::flush(&mut stdout).context("failed to flush confirmation prompt")?;

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .context("failed to read confirmation input")?;
    Ok(matches!(input.trim(), "y" | "Y" | "yes" | "YES"))
}

fn read_required_api_env(
    base_key: &str,
    model_key: &str,
    api_key_key: &str,
) -> anyhow::Result<ApiSetupInput> {
    Ok(ApiSetupInput {
        base_url: std::env::var(base_key)
            .with_context(|| format!("{base_key} must be set for `vera setup --api`"))?,
        model_id: std::env::var(model_key)
            .with_context(|| format!("{model_key} must be set for `vera setup --api`"))?,
        api_key: std::env::var(api_key_key)
            .with_context(|| format!("{api_key_key} must be set for `vera setup --api`"))?,
    })
}

fn read_optional_api_env(
    base_key: &str,
    model_key: &str,
    api_key_key: &str,
) -> anyhow::Result<Option<ApiSetupInput>> {
    let base = std::env::var(base_key).ok();
    let model = std::env::var(model_key).ok();
    let api_key = std::env::var(api_key_key).ok();

    match (base, model, api_key) {
        (Some(base_url), Some(model_id), Some(api_key)) => Ok(Some(ApiSetupInput {
            base_url,
            model_id,
            api_key,
        })),
        (None, None, None) => Ok(None),
        _ => bail!(
            "reranker config is incomplete. Set all of {base_key}, {model_key}, and {api_key_key}, or leave all three unset."
        ),
    }
}

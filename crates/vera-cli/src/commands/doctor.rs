//! `vera doctor` — inspect the current Vera setup for common failures.

use serde::Serialize;

use crate::state;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
enum CheckStatus {
    Ok,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize)]
struct DoctorCheck {
    name: &'static str,
    status: CheckStatus,
    detail: String,
}

#[derive(Debug, Clone, Serialize)]
struct DoctorReport {
    overall_ok: bool,
    checks: Vec<DoctorCheck>,
}

pub fn run(json_output: bool) -> anyhow::Result<()> {
    let mut checks = Vec::new();

    let config_path = state::config_path()?;
    checks.push(DoctorCheck {
        name: "config-file",
        status: if config_path.exists() {
            CheckStatus::Ok
        } else {
            CheckStatus::Warn
        },
        detail: config_path.display().to_string(),
    });

    let local_mode = vera_core::config::is_local_mode();
    checks.push(DoctorCheck {
        name: "effective-mode",
        status: CheckStatus::Ok,
        detail: if local_mode {
            "local".to_string()
        } else {
            "api".to_string()
        },
    });

    if local_mode {
        checks.push(DoctorCheck {
            name: "onnx-runtime",
            status: if vera_core::local_models::ensure_ort_runtime(None).is_ok() {
                CheckStatus::Ok
            } else {
                CheckStatus::Fail
            },
            detail: "required for local inference".to_string(),
        });

        let model_assets = vera_core::local_models::inspect_default_local_model_files()?;
        let present = model_assets.iter().filter(|asset| asset.exists).count();
        checks.push(DoctorCheck {
            name: "local-models",
            status: if present == model_assets.len() {
                CheckStatus::Ok
            } else {
                CheckStatus::Warn
            },
            detail: format!(
                "{present}/{} default local assets present",
                model_assets.len()
            ),
        });
    } else {
        checks.push(check_env_group(
            "embedding-api",
            &[
                "EMBEDDING_MODEL_BASE_URL",
                "EMBEDDING_MODEL_ID",
                "EMBEDDING_MODEL_API_KEY",
            ],
        ));
        checks.push(check_env_group(
            "reranker-api",
            &[
                "RERANKER_MODEL_BASE_URL",
                "RERANKER_MODEL_ID",
                "RERANKER_MODEL_API_KEY",
            ],
        ));
    }

    let cwd = std::env::current_dir()?;
    let index_dir = vera_core::indexing::index_dir(&cwd);
    checks.push(DoctorCheck {
        name: "current-index",
        status: if index_dir.exists() {
            CheckStatus::Ok
        } else {
            CheckStatus::Warn
        },
        detail: index_dir.display().to_string(),
    });

    let overall_ok = checks
        .iter()
        .all(|check| !matches!(check.status, CheckStatus::Fail));
    let report = DoctorReport { overall_ok, checks };

    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("Vera doctor");
        println!();
        for check in &report.checks {
            let icon = match check.status {
                CheckStatus::Ok => "ok",
                CheckStatus::Warn => "warn",
                CheckStatus::Fail => "fail",
            };
            println!("  {:<5} {:<14} {}", icon, check.name, check.detail);
        }
    }

    Ok(())
}

fn check_env_group(name: &'static str, keys: &[&'static str]) -> DoctorCheck {
    let present = keys
        .iter()
        .filter(|key| std::env::var_os(key).is_some())
        .count();

    let status = match present {
        0 => CheckStatus::Warn,
        n if n == keys.len() => CheckStatus::Ok,
        _ => CheckStatus::Fail,
    };

    DoctorCheck {
        name,
        status,
        detail: format!("{present}/{} variables present", keys.len()),
    }
}

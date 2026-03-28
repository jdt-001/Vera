//! `vera uninstall` — remove Vera binary, models, config, and agent skills.

use std::fs;
use std::path::PathBuf;

use anyhow::Result;

use super::agent;
use crate::state;

/// Candidate directories where the shim may have been placed.
fn shim_candidates(home: &std::path::Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(v) = std::env::var("VERA_USER_BIN_DIR") {
        if !v.is_empty() {
            dirs.push(PathBuf::from(v));
        }
    }
    #[cfg(windows)]
    {
        dirs.push(home.join("AppData").join("Roaming").join("npm"));
        dirs.push(
            home.join("AppData")
                .join("Local")
                .join("Programs")
                .join("Vera")
                .join("bin"),
        );
    }
    #[cfg(not(windows))]
    {
        dirs.push(home.join(".local").join("bin"));
        dirs.push(home.join(".cargo").join("bin"));
        dirs.push(home.join("bin"));
    }
    dirs
}

fn shim_name() -> &'static str {
    if cfg!(windows) { "vera.cmd" } else { "vera" }
}

pub fn run(json_output: bool) -> Result<()> {
    let home = state::user_home_dir()?;
    let vera_home = state::vera_dir()?;
    let mut removed = Vec::new();

    // 1. Remove agent skill files (all clients, all scopes).
    if let Err(e) = agent::run(
        agent::AgentCommand::Remove,
        Some(agent::AgentClient::All),
        Some(agent::AgentScope::All),
        json_output,
    ) {
        tracing::warn!("failed to remove agent skills: {e:#}");
    }
    removed.push("agent skills");

    // 2. Remove ~/.vera/ (binary cache, models, libs, config, credentials).
    if vera_home.exists() {
        fs::remove_dir_all(&vera_home)?;
        if !json_output {
            eprintln!("  Removed {}", vera_home.display());
        }
    }
    removed.push("~/.vera");

    // 3. Remove the PATH shim.
    let name = shim_name();
    for dir in shim_candidates(&home) {
        let shim = dir.join(name);
        if shim.exists() {
            // Only remove if it's a Vera shim (contains "vera" in content or is a symlink to vera).
            let is_vera_shim = fs::read_to_string(&shim)
                .map(|c| c.contains("vera"))
                .unwrap_or(false)
                || fs::read_link(&shim)
                    .map(|t| t.to_string_lossy().contains("vera"))
                    .unwrap_or(false);
            if is_vera_shim {
                fs::remove_file(&shim)?;
                if !json_output {
                    eprintln!("  Removed shim {}", shim.display());
                }
            }
        }
    }
    removed.push("PATH shim");

    if json_output {
        println!(
            "{}",
            serde_json::json!({ "uninstalled": true, "removed": removed })
        );
    } else {
        eprintln!();
        eprintln!("Vera has been uninstalled.");
        eprintln!("Per-project indexes (.vera/ in each project) were not removed.");
    }

    Ok(())
}

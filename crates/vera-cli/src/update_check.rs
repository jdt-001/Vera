//! Non-blocking update hints printed to stderr after command execution.
//!
//! Two checks:
//! 1. **Skill staleness** — compares binary version against `.version` files
//!    written by `vera agent install` into each agent client's skill directory.
//! 2. **Binary staleness** — fetches the latest release tag from GitHub (cached
//!    for 24 hours in `~/.vera/update-check.json`) and compares against the
//!    running binary version.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const GITHUB_API_TIMEOUT: Duration = Duration::from_secs(5);
const REPO: &str = "lemon07r/Vera";

/// Run all update checks and print hints to stderr. Never fails — errors are
/// silently swallowed so the user's actual command output is never disrupted.
pub fn print_nudges() {
    if std::env::var("VERA_NO_UPDATE_CHECK").is_ok() {
        return;
    }
    check_skill_staleness();
    check_binary_staleness();
}

// ---------------------------------------------------------------------------
// Skill version check
// ---------------------------------------------------------------------------

fn check_skill_staleness() {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return,
    };

    // Check global skill dirs for all known clients.
    let skill_dirs: Vec<PathBuf> = [".claude", ".codex", ".copilot", ".cursor", ".kiro"]
        .iter()
        .map(|client| home.join(client).join("skills").join("vera"))
        .collect();

    let mut any_stale = false;
    for dir in &skill_dirs {
        if let Some(installed) = read_skill_version(dir) {
            if installed != CURRENT_VERSION {
                any_stale = true;
                break;
            }
        }
    }

    if any_stale {
        eprintln!(
            "hint: installed skill files are from v{} (binary is v{}). \
             Run `vera agent install` to update.",
            skill_dirs
                .iter()
                .filter_map(|d| read_skill_version(d))
                .next()
                .unwrap_or_default(),
            CURRENT_VERSION,
        );
    }
}

fn read_skill_version(skill_dir: &Path) -> Option<String> {
    let version_file = skill_dir.join(".version");
    fs::read_to_string(version_file).ok().map(|s| s.trim().to_string())
}

// ---------------------------------------------------------------------------
// Binary version check (cached GitHub API)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct UpdateCache {
    latest_version: String,
    checked_at_secs: u64,
    #[serde(default)]
    install_method: Option<String>,
}

fn cache_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".vera").join("update-check.json"))
}

fn check_binary_staleness() {
    let cache_file = match cache_path() {
        Some(p) => p,
        None => return,
    };

    // Try loading cache first.
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if let Some(cached) = load_cache(&cache_file) {
        if now.saturating_sub(cached.checked_at_secs) < CHECK_INTERVAL.as_secs() {
            if is_newer(&cached.latest_version, CURRENT_VERSION) {
                print_binary_nudge(&cached.latest_version, cached.install_method.as_deref());
            }
            return;
        }
    }

    // Cache is stale or missing — fetch in background-ish (blocking but with
    // a short timeout so it doesn't delay the user noticeably).
    let latest = match fetch_latest_version() {
        Some(v) => v,
        None => return,
    };

    let method = detect_install_method();

    let cache = UpdateCache {
        latest_version: latest.clone(),
        checked_at_secs: now,
        install_method: method.clone(),
    };
    let _ = save_cache(&cache_file, &cache);

    if is_newer(&latest, CURRENT_VERSION) {
        print_binary_nudge(&latest, method.as_deref());
    }
}

fn print_binary_nudge(latest: &str, install_method: Option<&str>) {
    let update_cmd = match install_method {
        Some("npm") => "npm update -g @vera-ai/cli && vera agent install".to_string(),
        Some("bun") => "bunx @vera-ai/cli install".to_string(),
        Some("pip") => "pip install --upgrade vera-ai && vera agent install".to_string(),
        Some("uv") => "uvx vera-ai install".to_string(),
        _ => "bunx @vera-ai/cli install".to_string(),
    };
    eprintln!(
        "hint: vera v{} is available (current: v{}). Update: `{}`",
        latest, CURRENT_VERSION, update_cmd,
    );
}

/// Compare two semver-ish strings. Returns true if `latest` > `current`.
fn is_newer(latest: &str, current: &str) -> bool {
    let parse = |s: &str| -> (u32, u32, u32) {
        let s = s.strip_prefix('v').unwrap_or(s);
        let mut parts = s.split('.').map(|p| p.parse::<u32>().unwrap_or(0));
        (
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
        )
    };
    parse(latest) > parse(current)
}

fn fetch_latest_version() -> Option<String> {
    // Use a small blocking runtime since we're called from sync main().
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;

    rt.block_on(async {
        let url = format!(
            "https://api.github.com/repos/{}/releases/latest",
            REPO
        );
        let client = reqwest::Client::builder()
            .timeout(GITHUB_API_TIMEOUT)
            .build()
            .ok()?;
        let resp = client
            .get(&url)
            .header("User-Agent", format!("vera/{}", CURRENT_VERSION))
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let body: serde_json::Value = resp.json().await.ok()?;
        let tag = body.get("tag_name")?.as_str()?;
        Some(tag.strip_prefix('v').unwrap_or(tag).to_string())
    })
}

fn detect_install_method() -> Option<String> {
    // Check for global npm install.
    if std::process::Command::new("npm")
        .args(["list", "-g", "--depth=0", "@vera-ai/cli"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return Some("npm".to_string());
    }

    // Check for bun global install.
    if let Ok(output) = std::process::Command::new("bun")
        .args(["pm", "ls", "-g"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
    {
        if String::from_utf8_lossy(&output.stdout).contains("@vera-ai/cli") {
            return Some("bun".to_string());
        }
    }

    // Check for pip install.
    if std::process::Command::new("pip")
        .args(["show", "vera-ai"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return Some("pip".to_string());
    }

    // Check for uv install.
    if std::process::Command::new("uv")
        .args(["pip", "show", "vera-ai"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return Some("uv".to_string());
    }

    None
}

fn load_cache(path: &Path) -> Option<UpdateCache> {
    let data = fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

fn save_cache(path: &Path, cache: &UpdateCache) -> Option<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok()?;
    }
    let data = serde_json::to_string(cache).ok()?;
    fs::write(path, data).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_newer_works() {
        assert!(is_newer("0.4.0", "0.3.1"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(is_newer("0.3.2", "0.3.1"));
        assert!(!is_newer("0.3.1", "0.3.1"));
        assert!(!is_newer("0.3.0", "0.3.1"));
        assert!(is_newer("v0.4.0", "0.3.1"));
    }

    #[test]
    fn read_skill_version_missing_dir() {
        assert_eq!(read_skill_version(Path::new("/nonexistent/path")), None);
    }
}

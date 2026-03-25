//! `vera agent ...` — install and manage the Vera skill for coding agents.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use clap::ValueEnum;
use serde::Serialize;

use crate::skill_assets::{VERA_SKILL_FILES, VERA_SKILL_NAME};
use crate::state;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AgentCommand {
    Install,
    Status,
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentClient {
    All,
    Claude,
    Codex,
    Copilot,
    Cursor,
    Kiro,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentScope {
    Global,
    Project,
    All,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillLocationReport {
    client: AgentClient,
    scope: AgentScope,
    path: String,
    installed: bool,
}

pub fn run(
    command: AgentCommand,
    client: AgentClient,
    scope: AgentScope,
    json_output: bool,
) -> anyhow::Result<()> {
    match command {
        AgentCommand::Install => install(client, scope, json_output),
        AgentCommand::Status => status(client, scope, json_output),
        AgentCommand::Remove => remove(client, scope, json_output),
    }
}

fn install(client: AgentClient, scope: AgentScope, json_output: bool) -> anyhow::Result<()> {
    let locations = resolve_locations(client, scope)?;

    for location in &locations {
        if location.path.exists() {
            fs::remove_dir_all(&location.path).with_context(|| {
                format!(
                    "failed to replace existing skill at {}",
                    location.path.display()
                )
            })?;
        }
        install_skill_to(&location.path)?;
    }

    let reports = locations
        .into_iter()
        .map(|location| SkillLocationReport {
            client: location.client,
            scope: location.scope,
            path: location.path.display().to_string(),
            installed: true,
        })
        .collect::<Vec<_>>();

    if json_output {
        println!("{}", serde_json::to_string_pretty(&reports)?);
    } else {
        println!("Installed Vera skill:");
        println!();
        for report in &reports {
            println!(
                "  {:<8} {:<7} {}",
                format!("{:?}", report.client).to_lowercase(),
                format!("{:?}", report.scope).to_lowercase(),
                report.path
            );
        }
    }

    Ok(())
}

fn status(client: AgentClient, scope: AgentScope, json_output: bool) -> anyhow::Result<()> {
    let reports = resolve_locations(client, scope)?
        .into_iter()
        .map(|location| SkillLocationReport {
            client: location.client,
            scope: location.scope,
            path: location.path.display().to_string(),
            installed: location.path.join("SKILL.md").exists(),
        })
        .collect::<Vec<_>>();

    if json_output {
        println!("{}", serde_json::to_string_pretty(&reports)?);
    } else {
        println!("Vera skill status:");
        println!();
        for report in &reports {
            let status = if report.installed {
                "installed"
            } else {
                "missing"
            };
            println!(
                "  {:<8} {:<7} {:<10} {}",
                format!("{:?}", report.client).to_lowercase(),
                format!("{:?}", report.scope).to_lowercase(),
                status,
                report.path
            );
        }
    }

    Ok(())
}

fn remove(client: AgentClient, scope: AgentScope, json_output: bool) -> anyhow::Result<()> {
    let locations = resolve_locations(client, scope)?;
    let mut reports = Vec::with_capacity(locations.len());

    for location in locations {
        let installed = location.path.join("SKILL.md").exists();
        if installed {
            fs::remove_dir_all(&location.path).with_context(|| {
                format!(
                    "failed to remove installed skill at {}",
                    location.path.display()
                )
            })?;
        }
        reports.push(SkillLocationReport {
            client: location.client,
            scope: location.scope,
            path: location.path.display().to_string(),
            installed: false,
        });
    }

    if json_output {
        println!("{}", serde_json::to_string_pretty(&reports)?);
    } else {
        println!("Removed Vera skill from:");
        println!();
        for report in &reports {
            println!(
                "  {:<8} {:<7} {}",
                format!("{:?}", report.client).to_lowercase(),
                format!("{:?}", report.scope).to_lowercase(),
                report.path
            );
        }
    }

    Ok(())
}

fn install_skill_to(target_dir: &Path) -> anyhow::Result<()> {
    for file in VERA_SKILL_FILES {
        let path = target_dir.join(file.relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&path, file.contents)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct SkillLocation {
    client: AgentClient,
    scope: AgentScope,
    path: PathBuf,
}

fn resolve_locations(client: AgentClient, scope: AgentScope) -> anyhow::Result<Vec<SkillLocation>> {
    let cwd = std::env::current_dir().context("failed to resolve current directory")?;
    let home = state::user_home_dir()?;
    resolve_locations_with_roots(client, scope, &cwd, &home)
}

fn resolve_locations_with_roots(
    client: AgentClient,
    scope: AgentScope,
    cwd: &Path,
    home: &Path,
) -> anyhow::Result<Vec<SkillLocation>> {
    let clients = match client {
        AgentClient::All => vec![
            AgentClient::Claude,
            AgentClient::Codex,
            AgentClient::Copilot,
            AgentClient::Cursor,
            AgentClient::Kiro,
        ],
        single => vec![single],
    };
    let scopes = match scope {
        AgentScope::All => vec![AgentScope::Global, AgentScope::Project],
        single => vec![single],
    };

    let mut locations = Vec::new();
    for client in clients {
        for scope in &scopes {
            locations.push(SkillLocation {
                client,
                scope: *scope,
                path: skill_path_for(client, *scope, &cwd, &home)?,
            });
        }
    }

    Ok(locations)
}

fn skill_path_for(
    client: AgentClient,
    scope: AgentScope,
    cwd: &Path,
    home: &Path,
) -> anyhow::Result<PathBuf> {
    if scope == AgentScope::All {
        bail!("scope=all is only valid before path resolution");
    }

    let base = match (client, scope) {
        (AgentClient::Claude, AgentScope::Global) => home.join(".claude").join("skills"),
        (AgentClient::Claude, AgentScope::Project) => cwd.join(".claude").join("skills"),
        (AgentClient::Codex, AgentScope::Global) => home.join(".codex").join("skills"),
        (AgentClient::Codex, AgentScope::Project) => cwd.join(".codex").join("skills"),
        (AgentClient::Copilot, AgentScope::Global) => home.join(".copilot").join("skills"),
        (AgentClient::Copilot, AgentScope::Project) => cwd.join(".github").join("skills"),
        (AgentClient::Cursor, AgentScope::Global) => home.join(".cursor").join("skills"),
        (AgentClient::Cursor, AgentScope::Project) => cwd.join(".cursor").join("skills"),
        (AgentClient::Kiro, AgentScope::Global) => home.join(".kiro").join("skills"),
        (AgentClient::Kiro, AgentScope::Project) => cwd.join(".kiro").join("skills"),
        (AgentClient::All, _) => bail!("client=all is only valid before path resolution"),
        (_, AgentScope::All) => bail!("scope=all is only valid before path resolution"),
    };

    Ok(base.join(VERA_SKILL_NAME))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_locations_expands_scope_all() {
        let cwd = Path::new("/tmp/project");
        let home = Path::new("/tmp/home");
        let locations =
            resolve_locations_with_roots(AgentClient::Codex, AgentScope::All, cwd, home).unwrap();

        assert_eq!(locations.len(), 2);
        assert!(
            locations
                .iter()
                .any(|location| location.scope == AgentScope::Global)
        );
        assert!(
            locations
                .iter()
                .any(|location| location.scope == AgentScope::Project)
        );
    }

    #[test]
    fn copilot_project_skill_uses_github_dir() {
        let cwd = Path::new("/tmp/project");
        let home = Path::new("/tmp/home");
        let path = skill_path_for(AgentClient::Copilot, AgentScope::Project, cwd, home).unwrap();
        assert_eq!(path, PathBuf::from("/tmp/project/.github/skills/vera"));
    }
}

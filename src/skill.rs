//! The agent skill, embedded in the binary so it always matches this CLI version, and its installer.

use crate::cli::{ClientArg, SkillInstallArgs};
use crate::error::{CliError, Result};
use crate::output::Success;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

pub const SKILL_MD: &str = include_str!("../skills/taskhub/SKILL.md");
pub const WRITES_MD: &str = include_str!("../skills/taskhub/references/writes.md");
/// The frontmatter key that marks a bundle this CLI wrote, so it may be updated in place.
const MARKER: &str = "taskhub-cli:";

/// The files of the bundle, relative to the skill directory.
pub fn files() -> [(&'static str, &'static str); 2] {
    [("SKILL.md", SKILL_MD), ("references/writes.md", WRITES_MD)]
}

/// The skill body without frontmatter, for `taskhub guide`.
pub fn guide() -> String {
    let body = SKILL_MD.split_once("\n---\n").map_or(SKILL_MD, |(_, body)| body);
    format!("{}\n\n{}", body.trim(), WRITES_MD.trim())
}

/// True when an installed SKILL.md was written by this CLI (it carries the marker in its frontmatter).
fn written_by_us(skill_md: &str) -> bool {
    let Some(rest) = skill_md.strip_prefix("---\n") else { return false };
    let Some((frontmatter, _)) = rest.split_once("\n---") else { return false };
    frontmatter.lines().any(|line| line.trim_start().starts_with(MARKER))
}

fn base_dir(project: bool) -> Result<PathBuf> {
    if !project {
        return std::env::var_os("HOME")
            .filter(|h| !h.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| CliError::internal("HOME is not set."));
    }
    let output = std::process::Command::new("git").args(["rev-parse", "--show-toplevel"]).output();
    match output {
        Ok(out) if out.status.success() => Ok(PathBuf::from(String::from_utf8_lossy(&out.stdout).trim())),
        _ => Err(CliError::input("--project installs into a git repository, and this is not one.")
            .with_hint("Run it from inside the repository, or leave out --project to install for yourself.")),
    }
}

pub fn targets(client: ClientArg, project: bool) -> Result<Vec<(&'static str, PathBuf)>> {
    let base = base_dir(project)?;
    let mut out = Vec::new();
    if matches!(client, ClientArg::Claude | ClientArg::All) {
        out.push(("claude", base.join(".claude/skills/taskhub")));
    }
    if matches!(client, ClientArg::Codex | ClientArg::All) {
        out.push(("codex", base.join(".agents/skills/taskhub")));
    }
    Ok(out)
}

pub fn install(args: SkillInstallArgs) -> Result<Success> {
    let targets = targets(args.client, args.project)?;
    // Check every target before writing any, so a refusal leaves nothing half-installed.
    for (_, dir) in &targets {
        if let Ok(existing) = fs::read_to_string(dir.join("SKILL.md")) {
            if !written_by_us(&existing) {
                return Err(CliError::input(format!(
                    "{} holds a different skill named taskhub.",
                    dir.display()
                ))
                .with_hint("Move or rename that skill first; taskhub will not overwrite it."));
            }
        } else if dir.exists() && fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_some()) {
            return Err(CliError::input(format!("{} exists and is not a taskhub skill.", dir.display()))
                .with_hint("Move it aside first; taskhub will not overwrite it."));
        }
    }
    let mut installed = Vec::new();
    for (client, dir) in &targets {
        for (name, contents) in files() {
            write_file(&dir.join(name), contents)?;
        }
        installed.push(json!({ "client": client, "path": dir }));
    }
    let human = targets
        .iter()
        .map(|(client, dir)| format!("Installed the TaskHub skill for {client}: {}", dir.display()))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(Success::new(json!({ "installed": installed, "version": env!("CARGO_PKG_VERSION") }), human))
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    let dir = path.parent().ok_or_else(|| CliError::internal("Bad skill path."))?;
    fs::create_dir_all(dir)
        .map_err(|e| CliError::internal(format!("Could not create {}: {e}", dir.display())))?;
    let temp = dir.join(format!(".{}.tmp", uuid::Uuid::new_v4()));
    fs::write(&temp, contents).and_then(|()| fs::rename(&temp, path)).map_err(|e| {
        let _ = fs::remove_file(&temp);
        CliError::internal(format!("Could not write {}: {e}", path.display()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_its_own_bundle_only() {
        assert!(written_by_us(SKILL_MD));
        assert!(!written_by_us("---\nname: taskhub\ndescription: someone else's\n---\n# Theirs"));
        assert!(!written_by_us("# no frontmatter"));
    }

    #[test]
    fn skill_stays_within_its_budgets() {
        let frontmatter =
            SKILL_MD.strip_prefix("---\n").and_then(|r| r.split_once("\n---")).map(|(f, _)| f).unwrap();
        let field = |key: &str| {
            frontmatter
                .lines()
                .find_map(|l| l.strip_prefix(&format!("{key}: ")))
                .unwrap_or_default()
                .to_owned()
        };
        assert_eq!(field("name"), "taskhub");
        assert!(field("name").len() + field("description").len() <= 200, "name + description over 200 bytes");
        assert!(SKILL_MD.len() <= 4096, "SKILL.md is {} bytes", SKILL_MD.len());
        assert!(SKILL_MD.split_whitespace().count() <= 350, "SKILL.md is over 350 words");
        assert!(SKILL_MD.contains("](references/writes.md)"), "the entrypoint links the write reference");
    }
}

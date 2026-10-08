//! Git and GitHub CLI helpers: the current branch, branch names for items, and `--pr auto`.

use crate::error::{CliError, Code, Result};
use std::process::{Command, Stdio};

/// The current branch, or `None` outside a repository or on a detached HEAD.
pub fn current_branch() -> Option<String> {
    let output = Command::new("git")
        .args(["symbolic-ref", "--quiet", "--short", "HEAD"])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let name = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (output.status.success() && !name.is_empty()).then_some(name)
}

/// `web-12-fix-login-redirect`: the lowercase key, then the title as ASCII letters and digits joined by `-`,
/// at most 50 characters, cut at a word boundary.
pub fn branch_name(key: &str, title: &str) -> String {
    const MAX: usize = 50;
    let mut name = key.to_ascii_lowercase();
    let words = title
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| word.to_ascii_lowercase());
    for word in words {
        if name.len() + 1 + word.len() > MAX {
            break;
        }
        name.push('-');
        name.push_str(&word);
    }
    name
}

/// Switches to `name`, creating it from the current HEAD if it does not exist.
pub fn switch_to(name: &str) -> Result<bool> {
    let exists = Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", &format!("refs/heads/{name}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| CliError::input(format!("Could not run git: {e}")))?
        .success();
    let args: &[&str] = if exists { &["switch", name] } else { &["switch", "-c", name] };
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| CliError::input(format!("Could not run git: {e}")))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(CliError::input(format!("git switch failed: {message}")));
    }
    Ok(!exists)
}

/// The URL of the current branch's pull request, from `gh pr view`. Fails before any write when there is none.
pub fn pull_request_url() -> Result<String> {
    let missing = |why: &str| {
        CliError::new(Code::PrNotFound, format!("No pull request found: {why}."))
            .with_hint("Pass the URL instead: --pr https://github.com/OWNER/REPO/pull/N")
    };
    let output = Command::new("gh")
        .args(["pr", "view", "--json", "url", "-q", ".url"])
        .stderr(Stdio::piped())
        .output()
        .map_err(|_| missing("the GitHub CLI (gh) is not installed"))?;
    let url = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !output.status.success() || url.is_empty() {
        return Err(missing("gh found no pull request for this branch"));
    }
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(missing("gh returned something that is not a URL"));
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_names_follow_the_plan() {
        assert_eq!(branch_name("WEB-12", "Fix login redirect"), "web-12-fix-login-redirect");
        assert_eq!(branch_name("WEB-12", "Écran: «menu» — focus!"), "web-12-cran-menu-focus");
        let long = branch_name("WEB-12", "one two three four five six seven eight nine ten eleven");
        assert!(long.len() <= 50 && !long.ends_with('-'), "{long}");
        assert_eq!(long, "web-12-one-two-three-four-five-six-seven-eight");
        assert_eq!(branch_name("WEB-12", "!!!"), "web-12");
    }
}

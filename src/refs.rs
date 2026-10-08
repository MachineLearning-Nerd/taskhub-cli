//! Item keys (`WEB-12`) and project keys (`WEB`): parsing, and inference from the git branch.

use crate::error::{CliError, Code, Result};
use crate::output::RefMeta;

/// A validated, uppercase item key such as `WEB-12`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemKey(String);

impl ItemKey {
    /// Accepts `web-12` or `WEB-12`: a project key (a letter, then 1–9 letters or digits), a dash, a number ≥ 1.
    pub fn parse(text: &str) -> Option<ItemKey> {
        let (project, number) = text.split_once('-')?;
        let number_ok =
            !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()) && !number.starts_with('0');
        (is_project_key(project) && number_ok)
            .then(|| ItemKey(format!("{}-{number}", project.to_ascii_uppercase())))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn project(&self) -> &str {
        self.0.split_once('-').map_or("", |(project, _)| project)
    }
}

impl std::fmt::Display for ItemKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

pub fn is_project_key(text: &str) -> bool {
    let bytes = text.as_bytes();
    (2..=10).contains(&bytes.len())
        && bytes[0].is_ascii_alphabetic()
        && bytes[1..].iter().all(|b| b.is_ascii_alphanumeric())
}

pub fn project_key(text: &str) -> Result<String> {
    if is_project_key(text) {
        Ok(text.to_ascii_uppercase())
    } else {
        Err(CliError::input(format!("{text:?} is not a project key."))
            .with_hint("Project keys look like WEB or OPS2."))
    }
}

/// A resolved item reference and where it came from.
#[derive(Clone, Debug)]
pub struct Resolved {
    pub key: ItemKey,
    pub meta: RefMeta,
}

/// Uses the given reference, or the one in the current git branch name.
pub fn resolve(given: Option<&str>, command: &str) -> Result<Resolved> {
    if let Some(text) = given {
        let key = ItemKey::parse(text).ok_or_else(|| {
            CliError::input(format!("{text:?} is not an item key.")).with_hint("Item keys look like WEB-12.")
        })?;
        return Ok(Resolved { meta: RefMeta { key: key.to_string(), source: "argument" }, key });
    }
    let branch = crate::git::current_branch();
    match branch.as_deref().and_then(key_from_branch) {
        Some(key) => {
            if let Some(branch) = &branch {
                crate::output::note(&format!("Using {key} from branch {branch}"));
            }
            Ok(Resolved { meta: RefMeta { key: key.to_string(), source: "branch" }, key })
        }
        None => {
            let place = match &branch {
                Some(branch) => format!("branch {branch} has none"),
                None => "this is not a git branch".to_owned(),
            };
            Err(CliError::new(Code::RefRequired, format!("No item key given and {place}."))
                .with_hint(format!("Pass a key, for example: taskhub {command} WEB-12")))
        }
    }
}

/// `feature/web-12-fix-login` → `WEB-12`: the last path segment must start with `<key>-<number>`,
/// followed by the end or another `-`.
pub fn key_from_branch(branch: &str) -> Option<ItemKey> {
    let segment = branch.rsplit('/').next()?;
    let mut parts = segment.splitn(3, '-');
    let project = parts.next()?;
    let number = parts.next()?;
    ItemKey::parse(&format!("{project}-{number}"))
}

/// Splits `[REF] REST...` positionals: the first is a reference only if it looks like `KEY-N`.
pub fn split_leading_ref(positionals: &[String]) -> (Option<&str>, &[String]) {
    match positionals.split_first() {
        Some((first, rest)) if ItemKey::parse(first).is_some() && !rest.is_empty() => {
            (Some(first.as_str()), rest)
        }
        _ => (None, positionals),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_normalizes_item_keys() {
        assert_eq!(ItemKey::parse("web-12").unwrap().as_str(), "WEB-12");
        assert_eq!(ItemKey::parse("OPS2-1").unwrap().project(), "OPS2");
        for bad in ["WEB", "WEB-0", "WEB-012", "W-1", "1WEB-1", "WEB-1a", "TOOLONGKEY1-1", "WEB--1", ""] {
            assert!(ItemKey::parse(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn finds_keys_in_branch_names() {
        assert_eq!(key_from_branch("feature/web-12-fix-login").unwrap().as_str(), "WEB-12");
        assert_eq!(key_from_branch("web-12").unwrap().as_str(), "WEB-12");
        assert_eq!(key_from_branch("ops-3-x").unwrap().as_str(), "OPS-3");
        assert!(key_from_branch("main").is_none());
        assert!(key_from_branch("web-12/notes").is_none());
        assert!(key_from_branch("fix-login").is_none());
    }

    #[test]
    fn leading_ref_is_taken_only_when_it_looks_like_a_key() {
        let args = vec!["WEB-12".to_owned(), "https://x.example".to_owned()];
        assert_eq!(split_leading_ref(&args), (Some("WEB-12"), &args[1..]));
        let args = vec!["https://x.example".to_owned()];
        assert_eq!(split_leading_ref(&args), (None, &args[..]));
        let args = vec!["WEB-12".to_owned()];
        assert_eq!(split_leading_ref(&args), (None, &args[..]), "a lone value is the command's own argument");
    }
}

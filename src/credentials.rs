//! Where the token comes from, in order: `TASKHUB_TOKEN_FILE`, `TASKHUB_TOKEN`, then the saved login.

use crate::config::{self, Config, DEFAULT_ORIGIN, Origin};
use crate::error::{CliError, Code, Result};
use crate::token::Secret;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    TokenFile,
    Environment,
    CredentialsFile,
}

impl Source {
    pub fn describe(self) -> &'static str {
        match self {
            Source::TokenFile => "TASKHUB_TOKEN_FILE",
            Source::Environment => "TASKHUB_TOKEN",
            Source::CredentialsFile => "saved login",
        }
    }
}

/// A usable credential: the token, the server it belongs to, and where it came from.
#[derive(Clone, Debug)]
pub struct Credential {
    pub token: Secret,
    pub origin: Origin,
    pub source: Source,
    /// Known for a saved login; environment tokens learn it from `/me`.
    pub username: Option<String>,
}

/// The saved login in `credentials.toml`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Saved {
    origin: Origin,
    token: String,
    username: String,
    saved_at: String,
}

pub fn credentials_path() -> Result<PathBuf> {
    Ok(config::config_dir()?.join("credentials.toml"))
}

/// The origin used with environment tokens: the user config's `origin`, else the default.
fn configured_origin() -> Result<Origin> {
    match Config::load()?.origin {
        Some(origin) => Origin::parse(&origin),
        None => Origin::parse(DEFAULT_ORIGIN),
    }
}

pub fn load() -> Result<Credential> {
    if let Some(path) = std::env::var_os("TASKHUB_TOKEN_FILE").filter(|v| !v.is_empty()) {
        let path = PathBuf::from(path);
        let text = config::read_private(&path)?.ok_or_else(|| {
            CliError::new(
                Code::CredentialsMissing,
                format!("TASKHUB_TOKEN_FILE points to {}, which does not exist.", path.display()),
            )
        })?;
        return Ok(Credential {
            token: Secret::parse(&text)?,
            origin: configured_origin()?,
            source: Source::TokenFile,
            username: None,
        });
    }
    if let Some(value) = std::env::var("TASKHUB_TOKEN").ok().filter(|v| !v.is_empty()) {
        return Ok(Credential {
            token: Secret::parse(&value)?,
            origin: configured_origin()?,
            source: Source::Environment,
            username: None,
        });
    }
    let path = credentials_path()?;
    let Some(text) = config::read_private(&path)? else {
        return Err(CliError::new(Code::CredentialsMissing, "You are not logged in to TaskHub.")
            .with_hint("Create a token in TaskHub (user menu → API tokens), then run: taskhub auth login --with-token < token-file"));
    };
    let saved: Saved = toml::from_str(&text).map_err(|e| {
        CliError::new(Code::CredentialsMissing, format!("{} is damaged: {}", path.display(), e.message()))
            .with_hint("Run taskhub auth logout, then log in again.")
    })?;
    Ok(Credential {
        token: Secret::parse(&saved.token)?,
        origin: saved.origin,
        source: Source::CredentialsFile,
        username: Some(saved.username),
    })
}

pub fn save(origin: &Origin, token: &Secret, username: &str) -> Result<PathBuf> {
    let saved = Saved {
        origin: origin.clone(),
        token: token.expose_for_storage().to_owned(),
        username: username.to_owned(),
        saved_at: crate::clock::now_rfc3339(),
    };
    let text = toml::to_string(&saved)
        .map_err(|e| CliError::internal(format!("Could not encode the login: {e}")))?;
    let path = credentials_path()?;
    config::write_private(&path, text.as_bytes())?;
    Ok(path)
}

/// Deletes the saved login. Returns whether there was one.
pub fn delete() -> Result<bool> {
    let path = credentials_path()?;
    remove(&path)
}

fn remove(path: &Path) -> Result<bool> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(CliError::internal(format!("Could not delete {}: {e}", path.display()))),
    }
}

//! Paths, settings and the origin rules.
//!
//! The origin (which server gets the token) comes only from the credentials file or, for environment tokens,
//! from the user config. No flag or environment variable on an ordinary command can change it, so instructions
//! injected into an agent cannot redirect a token.

use crate::error::{CliError, Code, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

pub const DEFAULT_ORIGIN: &str = "https://taskhub.dineshjinjala.com";

fn home() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| CliError::internal("HOME is not set."))
}

fn xdg(variable: &str, fallback: &str) -> Result<PathBuf> {
    match std::env::var_os(variable).filter(|value| !value.is_empty()).map(PathBuf::from) {
        Some(path) if path.is_absolute() => Ok(path),
        _ => Ok(home()?.join(fallback)),
    }
}

pub fn config_dir() -> Result<PathBuf> {
    Ok(xdg("XDG_CONFIG_HOME", ".config")?.join("taskhub"))
}

pub fn state_dir() -> Result<PathBuf> {
    Ok(xdg("XDG_STATE_HOME", ".local/state")?.join("taskhub"))
}

/// Non-secret settings in `~/.config/taskhub/config.toml`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_project: Option<String>,
    /// Used only with TASKHUB_TOKEN or TASKHUB_TOKEN_FILE; a saved login carries its own origin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
}

impl Config {
    pub fn load() -> Result<Config> {
        let path = config_dir()?.join("config.toml");
        match fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text)
                .map_err(|e| CliError::input(format!("{} is not valid: {}", path.display(), e.message()))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(CliError::input(format!("Could not read {}: {e}", path.display()))),
        }
    }
}

/// `<repository>/.taskhub.toml`: a default project for the team. It cannot set an origin or credentials.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepoConfig {
    default_project: Option<String>,
}

/// The default project: the repository's `.taskhub.toml` (nearest parent), then the user config.
pub fn default_project() -> Result<Option<String>> {
    let mut dir = std::env::current_dir().ok();
    while let Some(current) = dir {
        let path = current.join(".taskhub.toml");
        if let Ok(text) = fs::read_to_string(&path) {
            let repo: RepoConfig = toml::from_str(&text).map_err(|e| {
                CliError::input(format!("{} is not valid: {}", path.display(), e.message()))
                    .with_hint("It may only contain default_project = \"KEY\".")
            })?;
            if let Some(project) = repo.default_project {
                return Ok(Some(project));
            }
        }
        if current.join(".git").exists() {
            break;
        }
        dir = current.parent().map(Path::to_path_buf);
    }
    Ok(Config::load()?.default_project)
}

/// A server origin such as `https://taskhub.example.com`: scheme, host and optional port only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Origin(String);

impl Origin {
    /// HTTPS is required, except plain HTTP to the literal addresses 127.0.0.1 and [::1].
    /// `localhost` is refused because a name can be redirected.
    pub fn parse(text: &str) -> Result<Origin> {
        let refuse = |why: &str| {
            CliError::new(
                Code::OriginNotAllowed,
                format!("{text:?} cannot be used as the TaskHub server: {why}."),
            )
            .with_hint(format!("Use an https:// address such as {DEFAULT_ORIGIN}, or http://127.0.0.1:PORT."))
        };
        let url = reqwest::Url::parse(text.trim()).map_err(|_| refuse("it is not a URL"))?;
        if !url.username().is_empty() || url.password().is_some() {
            return Err(refuse("it contains a user name or password"));
        }
        if url.query().is_some() || url.fragment().is_some() || !matches!(url.path(), "" | "/") {
            return Err(refuse("give only the scheme, host and port, with no path"));
        }
        let host = url.host_str().ok_or_else(|| refuse("it has no host"))?;
        match url.scheme() {
            "https" => {}
            "http" if host == "127.0.0.1" || host == "[::1]" => {}
            "http" => return Err(refuse("plain http is allowed only for 127.0.0.1 and [::1]")),
            _ => return Err(refuse("it must start with https://")),
        }
        let origin = url.origin().ascii_serialization();
        Ok(Origin(origin))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.0)
    }
}

impl TryFrom<String> for Origin {
    type Error = CliError;
    fn try_from(value: String) -> Result<Origin> {
        Origin::parse(&value)
    }
}

impl From<Origin> for String {
    fn from(origin: Origin) -> String {
        origin.0
    }
}

impl std::fmt::Display for Origin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Creates `dir` (and parents) with mode 0700.
pub fn private_dir(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)
        .map_err(|e| CliError::internal(format!("Could not create {}: {e}", dir.display())))?;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
        .map_err(|e| CliError::internal(format!("Could not secure {}: {e}", dir.display())))
}

/// Writes a file atomically with mode 0600: a new temporary file in the same directory, synced, then renamed.
pub fn write_private(path: &Path, contents: &[u8]) -> Result<()> {
    let dir = path.parent().ok_or_else(|| CliError::internal("A private file needs a directory."))?;
    private_dir(dir)?;
    let temp = dir.join(format!(
        ".{}.{}.tmp",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
        uuid::Uuid::new_v4()
    ));
    let write = || -> std::io::Result<()> {
        let mut file = fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&temp)?;
        file.write_all(contents)?;
        file.sync_all()?;
        fs::rename(&temp, path)?;
        fs::File::open(dir)?.sync_all()
    };
    write().map_err(|e| {
        let _ = fs::remove_file(&temp);
        CliError::internal(format!("Could not write {}: {e}", path.display()))
    })
}

/// Reads a file that must be private: owned by this user and not readable or writable by anyone else.
pub fn read_private(path: &Path) -> Result<Option<String>> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(CliError::internal(format!("Could not read {}: {e}", path.display()))),
    };
    let meta = file
        .metadata()
        .map_err(|e| CliError::internal(format!("Could not inspect {}: {e}", path.display())))?;
    let owner_ok = meta.uid() == rustix::process::geteuid().as_raw();
    if !owner_ok || meta.mode() & 0o077 != 0 {
        return Err(CliError::new(
            Code::CredentialsInsecure,
            format!("{} must be private: owned by you with mode 0600.", path.display()),
        )
        .with_hint(format!("chmod 600 {}", path.display())));
    }
    let mut text = String::new();
    use std::io::Read;
    (&file)
        .take(64 * 1024)
        .read_to_string(&mut text)
        .map_err(|e| CliError::internal(format!("Could not read {}: {e}", path.display())))?;
    Ok(Some(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origins_follow_the_https_rule() {
        assert_eq!(
            Origin::parse("https://taskhub.example.com/").unwrap().as_str(),
            "https://taskhub.example.com"
        );
        assert_eq!(
            Origin::parse("https://TaskHub.example.com:8443").unwrap().as_str(),
            "https://taskhub.example.com:8443"
        );
        assert_eq!(Origin::parse("http://127.0.0.1:3100").unwrap().as_str(), "http://127.0.0.1:3100");
        assert_eq!(Origin::parse("http://[::1]:3100").unwrap().as_str(), "http://[::1]:3100");
        for bad in [
            "http://localhost:3000",
            "http://taskhub.example.com",
            "ftp://taskhub.example.com",
            "https://user:pw@taskhub.example.com",
            "https://taskhub.example.com/api",
            "https://taskhub.example.com/?x=1",
            "taskhub.example.com",
        ] {
            let error = Origin::parse(bad).unwrap_err();
            assert_eq!(error.code, Code::OriginNotAllowed, "{bad}");
        }
    }

    #[test]
    fn private_files_are_written_0600_and_wider_permissions_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/secret.toml");
        write_private(&path, b"x = 1").unwrap();
        assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o600);
        assert_eq!(fs::metadata(path.parent().unwrap()).unwrap().mode() & 0o777, 0o700);
        assert_eq!(read_private(&path).unwrap().as_deref(), Some("x = 1"));
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(read_private(&path).unwrap_err().code, Code::CredentialsInsecure);
        assert_eq!(read_private(&dir.path().join("missing")).unwrap(), None);
    }
}

//! `taskhub auth login | status | logout`.

use super::Session;
use crate::api::{Client, HintContext};
use crate::cli::{AuthCommand, LoginArgs};
use crate::config::{DEFAULT_ORIGIN, Origin};
use crate::credentials::{self, Credential, Source};
use crate::error::{CliError, Code, Result};
use crate::output::{Success, note};
use crate::token::Secret;
use crate::types::Me;
use serde_json::{Value, json};
use std::io::{IsTerminal, Read};

pub fn run(command: AuthCommand) -> Result<Success> {
    match command {
        AuthCommand::Login(args) => login(args),
        AuthCommand::Status => status(),
        AuthCommand::Logout => logout(),
    }
}

fn read_token(with_token: bool) -> Result<Secret> {
    let stdin = std::io::stdin();
    // `--with-token` typed at a terminal, with nothing piped in, would wait silently for EOF; prompt instead.
    let interactive = stdin.is_terminal() && std::io::stderr().is_terminal();
    let text = if with_token && !interactive {
        let mut text = String::new();
        stdin
            .lock()
            .take(4096)
            .read_to_string(&mut text)
            .map_err(|e| CliError::input(format!("Could not read the token from stdin: {e}")))?;
        text
    } else if interactive {
        rpassword::prompt_password("TaskHub API token: ")
            .map_err(|e| CliError::input(format!("Could not read the token: {e}")))?
    } else {
        return Err(CliError::input("No token given.")
            .with_hint("Pipe the token in: taskhub auth login --with-token < token-file"));
    };
    Secret::parse(&text)
}

fn login(args: LoginArgs) -> Result<Success> {
    let origin = match &args.origin {
        Some(origin) => Origin::parse(origin)?,
        None => Origin::parse(DEFAULT_ORIGIN)?,
    };
    let token = read_token(args.with_token)?;
    let candidate = Credential {
        token: token.clone(),
        origin: origin.clone(),
        source: Source::CredentialsFile,
        username: None,
    };
    let client = Client::new(&candidate, super::default_deadline())?;
    let me = fetch_me(&client)?;
    let path = credentials::save(&origin, &token, &me.owner.username)?;
    if ["TASKHUB_TOKEN", "TASKHUB_TOKEN_FILE"]
        .iter()
        .any(|v| std::env::var_os(v).is_some_and(|v| !v.is_empty()))
    {
        note("Note: TASKHUB_TOKEN or TASKHUB_TOKEN_FILE is set and takes precedence over this login.");
    }
    let human = format!(
        "Logged in to {origin} as {}.\n{}\nSaved to {}",
        me.owner.username,
        describe(&me),
        path.display()
    );
    Ok(Success::new(json!({ "origin": origin, "me": me, "credentialsFile": path }), human))
}

fn status() -> Result<Success> {
    let session = Session::open(super::default_deadline())?;
    let me = fetch_me(&session.client)?;
    let source = session.credential.source;
    let mut human = format!(
        "Logged in to {} as {} (from {}).\n{}",
        session.credential.origin,
        me.owner.username,
        source.describe(),
        describe(&me)
    );
    let mut warnings = Vec::new();
    if matches!(source, Source::Environment | Source::TokenFile) {
        let warning = format!(
            "{} is set: every program the agent runs inherits it. Prefer taskhub auth login for interactive use.",
            source.describe()
        );
        human.push_str(&format!("\nWarning: {warning}"));
        warnings.push(warning);
    }
    Ok(Success::new(
        json!({ "origin": session.credential.origin, "source": source, "me": me, "warnings": warnings }),
        human,
    ))
}

fn logout() -> Result<Success> {
    let removed = credentials::delete()?;
    let human = if removed {
        "Logged out. The token still works until you revoke it in TaskHub (Settings → API tokens)."
    } else {
        "You were not logged in."
    };
    Ok(Success::new(json!({ "removed": removed }), human))
}

pub fn fetch_me(client: &Client) -> Result<Me> {
    let envelope = client.get("/me", &[], &HintContext::default())?;
    serde_json::from_value(envelope.get("data").cloned().unwrap_or(Value::Null))
        .map_err(|e| CliError::new(Code::ProtocolError, format!("Unexpected /me response: {e}")))
}

fn describe(me: &Me) -> String {
    let projects = if me.projects.is_empty() { "none".to_owned() } else { me.projects.join(", ") };
    let expiry = match &me.expires_at {
        Some(at) => format!("{} ({})", at, crate::clock::relative(at)),
        None => "never".to_owned(),
    };
    format!("Role: {}\nToken: {} access\nProjects: {projects}\nExpires: {expiry}", me.role, me.profile)
}

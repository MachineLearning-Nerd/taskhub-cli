//! The operation journal. Every write is recorded (and fsynced) before it is sent, so a write is never lost or
//! duplicated even if the process is killed after the server commits. Only this module decides whether a
//! request may be resent: identical bytes, the same Idempotency-Key, at most one automatic resend.
//!
//! An operation has one or more steps. `submit --attach a.png` is three: the upload, then the submission that
//! references the uploaded file's ID. A retry resumes from the first unfinished step.

use crate::api::{Client, HintContext, WriteResponse, problem_error};
use crate::config;
use crate::credentials::Credential;
use crate::digest::sha256_hex;
use crate::error::{CliError, Code, OperationMeta, Outcome, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use time::OffsetDateTime;

/// Replays are refused from 6 days 23 hours: the server keeps receipts 7 days; the hour covers clock skew.
const REPLAY_WINDOW: time::Duration = time::Duration::hours(6 * 24 + 23);
/// Finished entries are deleted after a day.
const KEEP_FINISHED: time::Duration = time::Duration::hours(24);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Pending,
    Committed,
    Rejected,
}

/// What a step sends.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Body {
    /// A JSON body. With `attachUploads`, the IDs from this operation's committed upload steps are added as
    /// `attachmentIds` when sending, so the bytes are the same on every attempt.
    #[serde(rename_all = "camelCase")]
    Json { json: Value, attach_uploads: bool },
    /// One file as multipart/form-data; the file must still have this size and SHA-256 when sent.
    #[serde(rename_all = "camelCase")]
    Upload { path: PathBuf, filename: String, content_type: String, size: u64, sha256: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    /// Sent as the Idempotency-Key.
    pub id: String,
    pub method: String,
    pub path: String,
    pub body: Body,
    pub state: State,
    /// The server's `data` once committed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replayed: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub origin: String,
    pub owner: String,
    pub command: String,
    /// The item the write is about, for hints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    pub created_at: String,
    pub state: State,
    pub steps: Vec<Step>,
    /// The error that rejected the operation, as an envelope `error` object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}

/// A step to plan before anything is sent.
pub struct PlannedStep {
    pub method: &'static str,
    pub path: String,
    pub body: Body,
}

impl PlannedStep {
    pub fn json(method: &'static str, path: String, json: Value) -> Self {
        PlannedStep { method, path, body: Body::Json { json, attach_uploads: false } }
    }

    pub fn json_with_uploads(method: &'static str, path: String, json: Value) -> Self {
        PlannedStep { method, path, body: Body::Json { json, attach_uploads: true } }
    }
}

/// The result of a committed operation.
pub struct Committed {
    /// The last step's receipt.
    pub data: Value,
    pub operation: OperationMeta,
    pub entry: Entry,
}

fn operations_dir() -> Result<PathBuf> {
    Ok(config::state_dir()?.join("operations"))
}

fn entry_path(id: &str) -> Result<PathBuf> {
    Ok(operations_dir()?.join(format!("{id}.json")))
}

fn now() -> OffsetDateTime {
    crate::clock::now()
}

impl Entry {
    fn save(&self) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|e| CliError::internal(format!("Could not encode the journal: {e}")))?;
        config::write_private(&entry_path(&self.id)?, &bytes)
    }

    pub fn load(id: &str) -> Result<Entry> {
        let missing = || {
            CliError::new(Code::OperationNotFound, format!("No operation {id} in the journal."))
                .with_hint("taskhub pending lists the writes still waiting.")
        };
        if uuid::Uuid::parse_str(id).is_err() {
            return Err(missing());
        }
        let text = config::read_private(&entry_path(id)?)?.ok_or_else(missing)?;
        serde_json::from_str(&text)
            .map_err(|e| CliError::internal(format!("Journal entry {id} is damaged: {e}")))
    }

    pub fn created(&self) -> Option<OffsetDateTime> {
        crate::clock::parse(&self.created_at)
    }

    fn meta(&self, outcome: Outcome) -> OperationMeta {
        let replayed = self.steps.last().and_then(|step| step.replayed);
        OperationMeta { id: self.id.clone(), replayed, outcome }
    }

    fn uploaded_ids(&self) -> Vec<Value> {
        self.steps
            .iter()
            .filter(|step| matches!(step.body, Body::Upload { .. }) && step.state == State::Committed)
            .filter_map(|step| step.receipt.as_ref()?.get("attachmentIds")?.as_array().cloned())
            .flatten()
            .collect()
    }
}

/// Every journal entry, oldest first. Deletes finished entries older than a day on the way.
pub fn all() -> Result<Vec<Entry>> {
    let dir = operations_dir()?;
    let mut entries = Vec::new();
    let Ok(listing) = fs::read_dir(&dir) else { return Ok(entries) };
    for file in listing.flatten() {
        let path = file.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(Some(text)) = config::read_private(&path) else { continue };
        let Ok(entry) = serde_json::from_str::<Entry>(&text) else { continue };
        let old = entry.created().is_some_and(|at| now() - at > KEEP_FINISHED);
        if entry.state != State::Pending && old {
            let _ = fs::remove_file(&path);
            continue;
        }
        entries.push(entry);
    }
    entries.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    Ok(entries)
}

pub fn discard(id: &str) -> Result<Entry> {
    let entry = Entry::load(id)?;
    fs::remove_file(entry_path(id)?)
        .map_err(|e| CliError::internal(format!("Could not discard {id}: {e}")))?;
    Ok(entry)
}

/// The owner (username) for a credential: known for a saved login, asked from the server for env tokens.
pub fn owner_of(credential: &Credential, client: &Client) -> Result<String> {
    match &credential.username {
        Some(username) => Ok(username.clone()),
        None => Ok(crate::commands::fetch_me(client)?.owner.username),
    }
}

/// Records a new operation (or finds the one with this `--request-id`) and runs it.
pub fn start(
    credential: &Credential,
    client: &Client,
    request_id: Option<&str>,
    key: Option<&str>,
    planned: Vec<PlannedStep>,
) -> Result<Committed> {
    let id = match request_id {
        Some(given) => uuid::Uuid::parse_str(given)
            .map_err(|_| CliError::input(format!("--request-id {given:?} is not a UUID.")))?
            .to_string(),
        None => uuid::Uuid::new_v4().to_string(),
    };
    let owner = owner_of(credential, client)?;
    if request_id.is_some() {
        if let Ok(existing) = Entry::load(&id) {
            return resume(existing, credential, client, &owner);
        }
    }
    // A single-step write uses the operation ID as its Idempotency-Key, so `--request-id` is the key the server
    // sees. Steps of a multi-step write get their own keys, recorded here before anything is sent.
    let single = planned.len() == 1;
    let steps = planned
        .into_iter()
        .map(|step| Step {
            id: if single { id.clone() } else { uuid::Uuid::new_v4().to_string() },
            method: step.method.to_owned(),
            path: step.path,
            body: step.body,
            state: State::Pending,
            receipt: None,
            replayed: None,
        })
        .collect::<Vec<_>>();
    let entry = Entry {
        id: id.clone(),
        origin: credential.origin.to_string(),
        owner,
        command: command_line(),
        key: key.map(str::to_owned),
        created_at: crate::clock::now_rfc3339(),
        state: State::Pending,
        steps,
        error: None,
    };
    entry.save()?;
    execute(entry, client)
}

/// `taskhub retry OP`.
pub fn retry(id: &str, credential: &Credential, client: &Client) -> Result<Committed> {
    let entry = Entry::load(id)?;
    let owner = owner_of(credential, client)?;
    resume(entry, credential, client, &owner)
}

fn resume(entry: Entry, credential: &Credential, client: &Client, owner: &str) -> Result<Committed> {
    match entry.state {
        State::Committed => {
            // Already done: report the original receipt as a replay, without sending anything.
            let data = entry.steps.last().and_then(|s| s.receipt.clone()).unwrap_or(Value::Null);
            let mut operation = entry.meta(Outcome::Committed);
            operation.replayed = Some(true);
            return Ok(Committed { data, operation, entry });
        }
        State::Rejected => return Err(stored_error(&entry)),
        State::Pending => {}
    }
    if entry.origin != credential.origin.as_str() || entry.owner != owner {
        return Err(CliError::new(
            Code::OperationOwnerMismatch,
            format!(
                "Operation {} belongs to {} on {}, not the current login.",
                entry.id, entry.owner, entry.origin
            ),
        )
        .with_hint(
            "Log in as that user on that server to retry it, or discard it: taskhub pending discard OP",
        )
        .with_operation(entry.meta(Outcome::Unknown)));
    }
    if entry.created().is_none_or(|at| now() - at >= REPLAY_WINDOW) {
        let check = entry
            .key
            .as_deref()
            .map_or_else(|| "taskhub show REF".to_owned(), |key| format!("taskhub show {key}"));
        return Err(CliError::new(Code::ReplayWindowExpired, "This write is too old to retry safely.")
            .with_hint(format!(
                "{check} to check whether the write happened, then discard it: taskhub pending discard {}",
                entry.id
            ))
            .with_operation(entry.meta(Outcome::Unknown)));
    }
    execute(entry, client)
}

fn stored_error(entry: &Entry) -> CliError {
    let stored = entry.error.clone().unwrap_or(Value::Null);
    let code = Code::parse(stored.get("code").and_then(Value::as_str).unwrap_or("INTERNAL"));
    let message = stored.get("message").and_then(Value::as_str).unwrap_or("The write was rejected.");
    let mut error = CliError::new(code, message);
    error.http_status = stored.get("httpStatus").and_then(Value::as_u64).map(|s| s as u16);
    error.details = stored.get("details").and_then(Value::as_object).cloned().unwrap_or_default();
    error.hint = stored.get("hint").and_then(Value::as_str).map(str::to_owned);
    error.with_operation(entry.meta(Outcome::Rejected))
}

fn command_line() -> String {
    std::env::args().skip(1).collect::<Vec<_>>().join(" ")
}

/// Builds the exact bytes for a step. Uploads re-read the file and refuse if it changed.
fn request_bytes(entry: &Entry, step: &Step) -> Result<(String, Vec<u8>)> {
    match &step.body {
        Body::Json { json, attach_uploads } => {
            let mut json = json.clone();
            if *attach_uploads {
                let ids = entry.uploaded_ids();
                if !ids.is_empty() {
                    if let Value::Object(map) = &mut json {
                        map.insert("attachmentIds".into(), Value::Array(ids));
                    }
                }
            }
            let bytes = serde_json::to_vec(&json)
                .map_err(|e| CliError::internal(format!("Could not encode the request: {e}")))?;
            Ok(("application/json".into(), bytes))
        }
        Body::Upload { path, filename, content_type, size, sha256 } => {
            let bytes = fs::read(path).map_err(|e| {
                CliError::new(Code::UploadChanged, format!("{} can no longer be read: {e}", path.display()))
                    .with_hint("Restore the file, or discard the operation: taskhub pending discard OP")
            })?;
            if bytes.len() as u64 != *size || &sha256_hex(&bytes) != sha256 {
                return Err(CliError::new(
                    Code::UploadChanged,
                    format!("{} changed since the write was recorded.", path.display()),
                )
                .with_hint(
                    "Restore the original file, or discard the operation: taskhub pending discard OP",
                ));
            }
            let boundary = format!("taskhub-{}", step.id);
            Ok((
                format!("multipart/form-data; boundary={boundary}"),
                multipart(&boundary, filename, content_type, &bytes),
            ))
        }
    }
}

pub fn multipart(boundary: &str, filename: &str, content_type: &str, bytes: &[u8]) -> Vec<u8> {
    let safe: String =
        filename.chars().map(|c| if c == '"' || c == '\\' || c.is_control() { '_' } else { c }).collect();
    let mut body = Vec::with_capacity(bytes.len() + 256);
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{safe}\"\r\n").as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {content_type}\r\n\r\n").as_bytes());
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

enum Attempt {
    Committed(Value),
    Rejected(CliError),
    Unknown(String),
}

fn send_step(client: &Client, entry: &Entry, step: &Step) -> Result<Attempt> {
    let (content_type, bytes) = request_bytes(entry, step)?;
    let hint = HintContext { key: entry.key.clone() };
    let mut resent = false;
    loop {
        let response = client.send_write(&step.method, &step.path, &content_type, &bytes, &step.id);
        let (status, body, retry_after) = match response {
            WriteResponse::Http { status, body, retry_after } => (status, body, retry_after),
            // Same bytes, same key: the server either never saw it or replays its stored receipt.
            WriteResponse::NotSent { .. } | WriteResponse::Unknown { .. }
                if !resent && !client.remaining().is_zero() =>
            {
                resent = true;
                continue;
            }
            WriteResponse::NotSent { reason } | WriteResponse::Unknown { reason } => {
                return Ok(Attempt::Unknown(reason));
            }
        };
        if (200..300).contains(&status) {
            let envelope: Value = match serde_json::from_slice(&body) {
                Ok(value) => value,
                Err(_) => return Ok(Attempt::Unknown("the server's reply was not JSON".into())),
            };
            return Ok(Attempt::Committed(envelope));
        }
        let error = problem_error(client.origin(), status, &body, retry_after, &hint);
        let wait = match error.code {
            Code::IdempotencyInProgress | Code::TemporarilyUnavailable => Some(Duration::from_secs(1)),
            Code::RateLimited => error.retry_after.map(Duration::from_secs),
            _ => None,
        };
        match wait {
            Some(wait) if !resent && wait < client.remaining() => {
                std::thread::sleep(wait);
                resent = true;
            }
            _ => return Ok(Attempt::Rejected(error)),
        }
    }
}

/// Runs the unfinished steps in order, saving the journal after each one.
fn execute(mut entry: Entry, client: &Client) -> Result<Committed> {
    for index in 0..entry.steps.len() {
        if entry.steps[index].state == State::Committed {
            continue;
        }
        let attempt = match send_step(client, &entry, &entry.steps[index]) {
            Ok(attempt) => attempt,
            // The step could not be built (an upload changed): nothing was sent; the operation stays pending.
            Err(error) => return Err(error.with_operation(entry.meta(Outcome::Unknown))),
        };
        match attempt {
            Attempt::Committed(envelope) => {
                let step = &mut entry.steps[index];
                step.state = State::Committed;
                step.receipt = envelope.get("data").cloned();
                step.replayed = envelope.pointer("/meta/operation/replayed").and_then(Value::as_bool);
                entry.save()?;
            }
            Attempt::Rejected(error) => {
                entry.steps[index].state = State::Rejected;
                entry.state = State::Rejected;
                entry.error = Some(error_value(&error));
                entry.save()?;
                return Err(error.with_operation(entry.meta(Outcome::Rejected)));
            }
            Attempt::Unknown(reason) => {
                return Err(CliError::new(
                    Code::OutcomeUnknown,
                    format!("The write was sent, but {reason}; it may or may not have happened."),
                )
                .with_hint(format!("taskhub retry {}", entry.id))
                .with_operation(entry.meta(Outcome::Unknown)));
            }
        }
    }
    entry.state = State::Committed;
    entry.save()?;
    let data = entry.steps.last().and_then(|s| s.receipt.clone()).unwrap_or(Value::Null);
    let operation = entry.meta(Outcome::Committed);
    Ok(Committed { data, operation, entry })
}

fn error_value(error: &CliError) -> Value {
    let mut map = Map::new();
    map.insert("code".into(), Value::String(error.code.as_str().to_owned()));
    map.insert("message".into(), Value::String(error.message.clone()));
    map.insert("httpStatus".into(), error.http_status.map_or(Value::Null, Value::from));
    map.insert("details".into(), Value::Object(error.details.clone()));
    map.insert("hint".into(), error.hint.clone().map_or(Value::Null, Value::String));
    Value::Object(map)
}

/// Describes an upload for the journal: checks type and size first, so a bad file fails before any write.
pub fn plan_upload(key: &str, path: &Path) -> Result<PlannedStep> {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| CliError::input(format!("{} is not a file name.", path.display())))?
        .to_owned();
    let extension = filename.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase()).unwrap_or_default();
    let content_type = match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "md" => "text/markdown",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        _ => {
            return Err(CliError::new(
                Code::UnsupportedFileType,
                format!("{filename}: TaskHub does not accept this file type."),
            )
            .with_hint("Attach .png, .jpg, .gif, .webp, .pdf, .md or .docx files."));
        }
    };
    let bytes =
        fs::read(path).map_err(|e| CliError::input(format!("Could not read {}: {e}", path.display())))?;
    if bytes.is_empty() {
        return Err(CliError::input(format!("{filename} is empty.")));
    }
    if bytes.len() as u64 > crate::api::MAX_ATTACHMENT_BYTES {
        return Err(CliError::new(Code::PayloadTooLarge, format!("{filename} is larger than 4 MB."))
            .with_hint("Attach a smaller file, for example a cropped screenshot."));
    }
    let absolute = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    Ok(PlannedStep {
        method: "POST",
        path: format!("/items/{key}/attachments"),
        body: Body::Upload {
            path: absolute,
            filename,
            content_type: content_type.to_owned(),
            size: bytes.len() as u64,
            sha256: sha256_hex(&bytes),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multipart_bytes_are_stable_and_quote_safe() {
        let a = multipart("b1", "evi\"dence.md", "text/markdown", b"# hi\n");
        let b = multipart("b1", "evi\"dence.md", "text/markdown", b"# hi\n");
        assert_eq!(a, b);
        let text = String::from_utf8(a).unwrap();
        assert!(text.contains("filename=\"evi_dence.md\""));
        assert!(text.starts_with("--b1\r\n") && text.ends_with("\r\n--b1--\r\n"));
    }

    #[test]
    fn uploads_are_checked_before_anything_is_sent() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("notes.txt");
        fs::write(&bad, "x").unwrap();
        assert_eq!(plan_upload("WEB-1", &bad).err().unwrap().code, Code::UnsupportedFileType);
        let empty = dir.path().join("empty.png");
        fs::write(&empty, "").unwrap();
        assert_eq!(plan_upload("WEB-1", &empty).err().unwrap().code, Code::InvalidInput);
        let big = dir.path().join("big.pdf");
        fs::write(&big, vec![0u8; 4 * 1024 * 1024 + 1]).unwrap();
        assert_eq!(plan_upload("WEB-1", &big).err().unwrap().code, Code::PayloadTooLarge);
        let ok = dir.path().join("Shot.PNG");
        fs::write(&ok, "png").unwrap();
        let step = plan_upload("WEB-1", &ok).unwrap();
        assert!(matches!(step.body, Body::Upload { ref content_type, .. } if content_type == "image/png"));
    }
}

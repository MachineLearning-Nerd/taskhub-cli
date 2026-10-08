//! The only HTTP client. Enforces the plan's limits and deadlines, retries reads at most once, sends writes
//! exactly once per call (the journal decides about resending), and turns Problem Details into `CliError`s.

use crate::config::Origin;
use crate::credentials::Credential;
use crate::error::{CliError, Code, Result};
use crate::token::Secret;
use crate::types::Problem;
use reqwest::Method;
use reqwest::blocking::{Client as Http, Response};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue, RETRY_AFTER};
use serde_json::{Map, Value};
use std::io::Read;
use std::time::{Duration, Instant};

/// Largest JSON or error body accepted, in decoded bytes.
pub const MAX_JSON_BYTES: u64 = 2 * 1024 * 1024;
/// Largest attachment, matching TaskHub.
pub const MAX_ATTACHMENT_BYTES: u64 = 4 * 1024 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
pub const COMMAND_DEADLINE: Duration = Duration::from_secs(30);
pub const UPLOAD_DEADLINE: Duration = Duration::from_secs(60);

pub fn client_header() -> String {
    format!("taskhub-cli/{}", env!("CARGO_PKG_VERSION"))
}

pub struct Client {
    origin: Origin,
    token: Secret,
    http: Http,
    deadline: Instant,
}

/// Context that makes hints exact: the item key and its web URL when known.
#[derive(Clone, Debug, Default)]
pub struct HintContext {
    pub key: Option<String>,
}

/// A response to a write, before the journal interprets it.
pub enum WriteResponse {
    /// The server answered with a full body within the limit.
    Http { status: u16, body: Vec<u8>, retry_after: Option<u64> },
    /// The request may or may not have reached the server: timeout, connection reset, oversized body.
    Unknown { reason: String },
    /// The connection was never opened, so the server cannot have seen the request.
    NotSent { reason: String },
}

impl Client {
    pub fn new(credential: &Credential, deadline: Duration) -> Result<Client> {
        let http = Http::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .user_agent(client_header())
            .build()
            .map_err(|e| CliError::internal(format!("Could not start the HTTP client: {e}")))?;
        Ok(Client {
            origin: credential.origin.clone(),
            token: credential.token.clone(),
            http,
            deadline: Instant::now() + deadline,
        })
    }

    pub fn origin(&self) -> &Origin {
        &self.origin
    }

    pub fn remaining(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }

    fn headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        let mut auth = HeaderValue::from_str(&format!("Bearer {}", self.token.expose_for_header()))
            .map_err(|_| CliError::internal("The token cannot be sent as a header."))?;
        auth.set_sensitive(true);
        headers.insert(AUTHORIZATION, auth);
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(
            "TaskHub-Client",
            HeaderValue::from_str(&client_header()).map_err(|_| CliError::internal("Bad client header."))?,
        );
        Ok(headers)
    }

    fn request_timeout(&self) -> Option<Duration> {
        let remaining = self.remaining();
        (!remaining.is_zero()).then(|| remaining.min(REQUEST_TIMEOUT))
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api/v1{path}", self.origin)
    }

    /// GET a JSON envelope (`{data, meta}`). Retries once on a transport failure, a 503, or a 429 whose
    /// wait fits within the command deadline.
    pub fn get(&self, path: &str, query: &[(String, String)], hint: &HintContext) -> Result<Value> {
        let mut attempt = 0;
        loop {
            attempt += 1;
            let error = match self.get_once(path, query) {
                Ok(GetOutcome::Ok(value)) => return Ok(value),
                Ok(GetOutcome::Problem(status, body, retry_after)) => {
                    let error = self.problem(status, &body, retry_after, hint);
                    let wait = match error.code {
                        Code::RateLimited => error.retry_after.map(Duration::from_secs),
                        Code::TemporarilyUnavailable | Code::IdempotencyInProgress => {
                            Some(Duration::from_secs(1))
                        }
                        _ => None,
                    };
                    match wait {
                        Some(wait) if attempt == 1 && wait < self.remaining() => {
                            std::thread::sleep(wait);
                            continue;
                        }
                        _ => return Err(error),
                    }
                }
                Err(error) => error,
            };
            if error.code == Code::TransportFailed && attempt == 1 && !self.remaining().is_zero() {
                continue;
            }
            return Err(error);
        }
    }

    fn get_once(&self, path: &str, query: &[(String, String)]) -> Result<GetOutcome> {
        let timeout = self.request_timeout().ok_or_else(deadline_passed)?;
        let mut url = reqwest::Url::parse(&self.url(path))
            .map_err(|_| CliError::internal(format!("Bad API path {path}")))?;
        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query);
        }
        let response =
            self.http.get(url).headers(self.headers()?).timeout(timeout).send().map_err(|e| transport(&e))?;
        let status = response.status().as_u16();
        let retry_after = retry_after(&response);
        let body = read_limited(response, MAX_JSON_BYTES).map_err(|e| match e {
            ReadError::TooLarge => {
                CliError::new(Code::ProtocolError, "The server's response was larger than 2 MiB.")
                    .with_hint("Narrow the request, for example with --limit.")
            }
            ReadError::Io(message) => {
                CliError::new(Code::TransportFailed, format!("The response was cut off: {message}"))
            }
        })?;
        if (200..300).contains(&status) {
            let value: Value = serde_json::from_slice(&body).map_err(|_| {
                CliError::new(Code::ProtocolError, "The server sent a response that is not JSON.")
            })?;
            if !value.get("data").is_some() {
                return Err(CliError::new(Code::ProtocolError, "The server's response has no data."));
            }
            return Ok(GetOutcome::Ok(value));
        }
        Ok(GetOutcome::Problem(status, body, retry_after))
    }

    /// Sends one write. Never retries: the journal owns resending, with the same bytes and key.
    pub fn send_write(
        &self,
        method: &str,
        path: &str,
        content_type: &str,
        body: &[u8],
        idempotency_key: &str,
    ) -> WriteResponse {
        let Some(timeout) = self.request_timeout() else {
            return WriteResponse::Unknown { reason: "the command deadline passed before sending".into() };
        };
        let method = match Method::from_bytes(method.as_bytes()) {
            Ok(method) => method,
            Err(_) => return WriteResponse::Unknown { reason: format!("bad method {method}") },
        };
        let mut headers = match self.headers() {
            Ok(headers) => headers,
            Err(e) => return WriteResponse::Unknown { reason: e.message.clone() },
        };
        let (Ok(content), Ok(key)) =
            (HeaderValue::from_str(content_type), HeaderValue::from_str(idempotency_key))
        else {
            return WriteResponse::Unknown { reason: "bad request headers".into() };
        };
        headers.insert(CONTENT_TYPE, content);
        headers.insert("Idempotency-Key", key);
        let response = match self
            .http
            .request(method, self.url(path))
            .headers(headers)
            .body(body.to_vec())
            .timeout(timeout)
            .send()
        {
            Ok(response) => response,
            Err(e) if e.is_connect() && !e.is_timeout() => {
                return WriteResponse::NotSent { reason: describe(&e) };
            }
            Err(e) => return WriteResponse::Unknown { reason: describe(&e) },
        };
        let status = response.status().as_u16();
        let retry_after = retry_after(&response);
        match read_limited(response, MAX_JSON_BYTES) {
            Ok(body) => WriteResponse::Http { status, body, retry_after },
            Err(ReadError::TooLarge) => {
                WriteResponse::Unknown { reason: "the response was larger than 2 MiB".into() }
            }
            Err(ReadError::Io(message)) => {
                WriteResponse::Unknown { reason: format!("the response was cut off: {message}") }
            }
        }
    }

    /// Streams an attachment into `writer`, refusing anything over the 4 MB attachment limit.
    pub fn download(&self, id: &str, writer: &mut dyn std::io::Write, hint: &HintContext) -> Result<u64> {
        let timeout = self.request_timeout().ok_or_else(deadline_passed)?;
        let mut headers = self.headers()?;
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        let response = self
            .http
            .get(self.url(&format!("/attachments/{id}")))
            .headers(headers)
            .timeout(timeout)
            .send()
            .map_err(|e| transport(&e))?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            let retry_after = retry_after(&response);
            let body = read_limited(response, MAX_JSON_BYTES).unwrap_or_default();
            return Err(self.problem(status, &body, retry_after, hint));
        }
        let mut limited = response.take(MAX_ATTACHMENT_BYTES + 1);
        let copied = std::io::copy(&mut limited, writer)
            .map_err(|e| CliError::new(Code::TransportFailed, format!("The download was cut off: {e}")))?;
        if copied > MAX_ATTACHMENT_BYTES {
            return Err(CliError::new(Code::ProtocolError, "The file is larger than TaskHub's 4 MB limit."));
        }
        Ok(copied)
    }

    /// Turns an error response into a `CliError` with the CLI's hint for that code.
    pub fn problem(
        &self,
        status: u16,
        body: &[u8],
        retry_after: Option<u64>,
        hint: &HintContext,
    ) -> CliError {
        problem_error(&self.origin, status, body, retry_after, hint)
    }
}

enum GetOutcome {
    Ok(Value),
    Problem(u16, Vec<u8>, Option<u64>),
}

enum ReadError {
    TooLarge,
    Io(String),
}

fn read_limited(response: Response, limit: u64) -> std::result::Result<Vec<u8>, ReadError> {
    let mut body = Vec::new();
    response.take(limit + 1).read_to_end(&mut body).map_err(|e| ReadError::Io(e.to_string()))?;
    if body.len() as u64 > limit { Err(ReadError::TooLarge) } else { Ok(body) }
}

fn retry_after(response: &Response) -> Option<u64> {
    let value = response.headers().get(RETRY_AFTER)?.to_str().ok()?.trim();
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(seconds);
    }
    // HTTP-date form.
    let format = time::macros::format_description!(
        "[weekday repr:short], [day] [month repr:short] [year] [hour]:[minute]:[second] GMT"
    );
    let at = time::PrimitiveDateTime::parse(value, &format).ok()?.assume_utc();
    Some((at - crate::clock::now()).whole_seconds().max(0) as u64)
}

fn describe(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        "the request timed out".into()
    } else if error.is_connect() {
        "could not connect to the server".into()
    } else {
        "the connection failed".into()
    }
}

fn transport(error: &reqwest::Error) -> CliError {
    CliError::new(Code::TransportFailed, format!("TaskHub could not be reached: {}.", describe(error)))
        .with_hint("Check your network connection and run the command again.")
}

fn deadline_passed() -> CliError {
    CliError::new(Code::TransportFailed, "The command ran out of time.").with_hint("Run the command again.")
}

pub fn problem_error(
    origin: &Origin,
    status: u16,
    body: &[u8],
    retry_after: Option<u64>,
    hint: &HintContext,
) -> CliError {
    let Ok(problem) = serde_json::from_slice::<Problem>(body) else {
        let code = if status == 429 {
            Code::RateLimited
        } else if status >= 500 {
            Code::TemporarilyUnavailable
        } else {
            Code::ProtocolError
        };
        let mut error =
            CliError::new(code, format!("TaskHub answered HTTP {status} without a usable error body."));
        error.http_status = Some(status);
        error.retry_after = retry_after;
        return error;
    };
    let code = Code::parse(&problem.code);
    let details = problem.details.clone().unwrap_or_default();
    let mut error = CliError::new(code.clone(), message_for(&problem, &details));
    error.http_status = Some(problem.status);
    error.details = details.clone();
    error.retry_after = retry_after.or_else(|| details.get("retryAfterSeconds").and_then(Value::as_u64));
    error.hint = hint_for(&code, &details, origin, hint, error.retry_after).or(problem.hint.clone());
    error
}

fn message_for(problem: &Problem, details: &Map<String, Value>) -> String {
    let field_messages = || -> Option<String> {
        let fields = details.get("fields")?.as_object()?;
        let parts: Vec<String> = fields
            .iter()
            .map(|(field, messages)| {
                let text = messages.as_array().map_or_else(String::new, |list| {
                    list.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" ")
                });
                format!("{field}: {text}")
            })
            .collect();
        (!parts.is_empty()).then(|| parts.join("; "))
    };
    let allowed = || -> Option<String> {
        let list = details.get("allowed")?.as_array()?;
        Some(list.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", "))
    };
    match problem.code.as_str() {
        "STATUS_CONFLICT" => match details.get("currentStatus").and_then(Value::as_str) {
            Some(status) => format!("The item has moved: it is now in {status}."),
            None => problem.title.clone(),
        },
        "VERSION_CONFLICT" => match details.get("currentVersion").and_then(Value::as_u64) {
            Some(version) => format!("The item changed: it is now at version {version}."),
            None => problem.title.clone(),
        },
        "ALREADY_CLAIMED" => match details.get("assignee").and_then(Value::as_str) {
            Some(assignee) => format!("Already claimed by {assignee}."),
            None => problem.title.clone(),
        },
        "UNKNOWN_USER" | "UNKNOWN_LABEL" | "UNSUPPORTED_FILE_TYPE" => {
            let base = field_messages().unwrap_or_else(|| problem.title.clone());
            match allowed() {
                Some(list) if !list.is_empty() => format!("{base} Allowed: {list}."),
                _ => base,
            }
        }
        "VALIDATION_FAILED" => field_messages().unwrap_or_else(|| problem.title.clone()),
        "UNAUTHENTICATED" => match details.get("reason").and_then(Value::as_str) {
            Some("expired") => "The token has expired.".into(),
            Some("revoked") => "The token was revoked.".into(),
            Some("owner_inactive") => "The token's owner is deactivated in TaskHub.".into(),
            Some("password_change_required") => "The token's owner must change their password first.".into(),
            Some("api_access_disabled") => "API access is turned off for the token's owner.".into(),
            Some("missing") | Some("malformed") | Some("invalid") => {
                "TaskHub did not accept the token.".into()
            }
            _ => problem.title.clone(),
        },
        "CLIENT_UPGRADE_REQUIRED" => match details.get("minClientVersion").and_then(Value::as_str) {
            Some(min) => {
                format!("TaskHub needs taskhub {min} or newer; this is {}.", env!("CARGO_PKG_VERSION"))
            }
            None => problem.title.clone(),
        },
        _ => problem.title.clone(),
    }
}

fn hint_for(
    code: &Code,
    details: &Map<String, Value>,
    origin: &Origin,
    context: &HintContext,
    retry: Option<u64>,
) -> Option<String> {
    let key = context.key.as_deref();
    let show = key.map_or_else(|| "taskhub show REF".to_owned(), |key| format!("taskhub show {key}"));
    let item_url = key.and_then(|key| {
        let (project, number) = key.split_once('-')?;
        Some(origin.url(&format!("/projects/{project}/items/{number}")))
    });
    let tokens = origin.url("/settings/tokens");
    Some(match code {
        Code::VersionConflict => format!("{show}, then reapply only what still matches the request."),
        Code::StatusConflict | Code::InvalidTransition => show,
        Code::AlreadyClaimed => "taskhub next --claim".into(),
        Code::SignOffRequiresPerson => match item_url {
            Some(url) => format!("Ask a Tester to sign off in TaskHub: {url}"),
            None => "Ask a Tester to sign off in TaskHub.".into(),
        },
        Code::DeleteRequiresPerson => match item_url {
            Some(url) => format!("Ask a person to delete it in TaskHub: {url}"),
            None => "A person deletes items in TaskHub.".into(),
        },
        Code::Unauthenticated => match details.get("reason").and_then(Value::as_str) {
            Some("password_change_required") => format!("Sign in at {origin} and change your password, then try again."),
            Some("api_access_disabled") => "Ask the TaskHub Admin to turn API access back on for you.".into(),
            Some("owner_inactive") => "Ask the TaskHub Admin to reactivate the account.".into(),
            _ => format!("Create a token at {tokens}, then run: taskhub auth login --with-token"),
        },
        Code::ReadOnlyToken => format!("Create a Read and write token for this project at {tokens}."),
        Code::RateLimited => match retry {
            Some(seconds) => format!("Wait {seconds} seconds, then run the command again."),
            None => "Wait a minute, then run the command again.".into(),
        },
        Code::ClientUpgradeRequired => {
            "Upgrade taskhub: brew upgrade taskhub, your AUR helper (taskhub-cli-bin), or the installer from the releases page.".into()
        }
        Code::NotFound => "Check the key or ID. The item may have been deleted, or this token is not granted its project (taskhub auth status).".into(),
        Code::ProjectArchived => "The project is archived; ask the TaskHub Admin to restore it.".into(),
        Code::NotAgentComment => "Only comments written through a token can be edited. Add a new comment instead.".into(),
        Code::UnknownLabel => "Use one of the allowed labels (taskhub projects show KEY).".into(),
        Code::UnknownUser => "Use one of the allowed usernames (taskhub projects show KEY).".into(),
        Code::ValidationFailed | Code::BadRequest => "Fix the fields named in the error, then run the command again.".into(),
        Code::UnsupportedFileType => "Attach .png, .jpg, .gif, .webp, .pdf, .md or .docx files.".into(),
        Code::PayloadTooLarge => "Make the text or file smaller.".into(),
        Code::IdempotencyConflict => "This request ID was already used for a different write. Investigate; do not reuse it.".into(),
        Code::TemporarilyUnavailable => "TaskHub is busy. Try again in a moment.".into(),
        Code::InvalidCursor => "Start again without --cursor.".into(),
        Code::AttachmentNotAvailable => "Attach files you uploaded to this item that are not yet on a comment.".into(),
        Code::ResponseTooLarge => "Narrow the request, for example with --limit or fewer filters.".into(),
        Code::IdempotencyInProgress => "The same write is still being processed. Wait a moment, then run taskhub pending.".into(),
        Code::IdempotencyKeyRequired => "This is a taskhub bug; please report it with the command you ran.".into(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin() -> Origin {
        Origin::parse("https://taskhub.example.com").unwrap()
    }

    fn fixture(code: &str) -> Vec<u8> {
        std::fs::read(format!("{}/api/v1/fixtures/errors/{code}.json", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    #[test]
    fn problems_become_errors_with_exact_hints() {
        let context = HintContext { key: Some("WEB-12".into()) };
        let error = problem_error(&origin(), 409, &fixture("STATUS_CONFLICT"), None, &context);
        assert_eq!(error.code, Code::StatusConflict);
        assert_eq!(error.http_status, Some(409));
        assert_eq!(error.message, "The item has moved: it is now in dev_done.");
        assert_eq!(error.hint.as_deref(), Some("taskhub show WEB-12"));
        assert_eq!(error.details["currentStatus"], "dev_done");

        let error = problem_error(&origin(), 403, &fixture("SIGN_OFF_REQUIRES_PERSON"), None, &context);
        assert_eq!(
            error.hint.as_deref(),
            Some("Ask a Tester to sign off in TaskHub: https://taskhub.example.com/projects/WEB/items/12")
        );

        let error = problem_error(&origin(), 429, &fixture("RATE_LIMITED"), None, &context);
        assert_eq!(error.retry_after, Some(30));
        assert_eq!(error.exit_code(), 7);

        let error = problem_error(&origin(), 422, &fixture("UNKNOWN_LABEL"), None, &context);
        assert!(error.message.ends_with("Allowed: accessibility."), "{}", error.message);

        let error = problem_error(&origin(), 401, &fixture("UNAUTHENTICATED"), None, &context);
        assert_eq!(error.message, "The token was revoked.");
        assert!(error.hint.as_deref().unwrap().contains("https://taskhub.example.com/settings/tokens"));
    }

    #[test]
    fn every_error_fixture_gets_a_hint() {
        let dir = format!("{}/api/v1/fixtures/errors", env!("CARGO_MANIFEST_DIR"));
        for entry in std::fs::read_dir(dir).unwrap() {
            let body = std::fs::read(entry.unwrap().path()).unwrap();
            let error = problem_error(&origin(), 400, &body, None, &HintContext::default());
            assert!(error.hint.is_some(), "{} has no hint", error.code);
        }
    }
}

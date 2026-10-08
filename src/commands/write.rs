//! Write commands. Each validates its input locally, plans its steps, and hands them to the journal; nothing
//! here sends a request directly.

use super::{Session, render};
use crate::api;
use crate::cli::*;
use crate::error::{CliError, Code, Result};
use crate::journal::{self, PlannedStep};
use crate::output::{RefMeta, Success};
use crate::refs::{self, ItemKey, project_key};
use crate::types::*;
use serde_json::{Value, json};
use std::cell::Cell;
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};

const COMMENT_LIMIT: usize = 5000;
const TITLE_LIMIT: usize = 200;
const DESCRIPTION_LIMIT: usize = 20000;
const URL_LIMIT: usize = 500;
const MAX_INPUT_FILE: u64 = 1024 * 1024;

/// Only one option may read stdin (`--body-file -` and the like).
#[derive(Default)]
struct Stdin(Cell<bool>);

impl Stdin {
    fn read(&self, option: &str) -> Result<String> {
        if self.0.replace(true) {
            return Err(CliError::input(format!("{option} cannot read stdin: another option already does.")));
        }
        read_limited(std::io::stdin().lock(), option)
    }
}

fn read_limited(reader: impl Read, what: &str) -> Result<String> {
    let mut text = String::new();
    reader
        .take(MAX_INPUT_FILE + 1)
        .read_to_string(&mut text)
        .map_err(|e| CliError::input(format!("Could not read {what}: {e}")))?;
    if text.len() as u64 > MAX_INPUT_FILE {
        return Err(CliError::input(format!("{what} is larger than 1 MB.")));
    }
    Ok(text)
}

fn read_file(path: &Path, option: &str, stdin: &Stdin) -> Result<String> {
    if path == Path::new("-") {
        return stdin.read(option);
    }
    let file = std::fs::File::open(path)
        .map_err(|e| CliError::input(format!("Could not read {}: {e}", path.display())))?;
    read_limited(file, &path.display().to_string())
}

/// Text from `--x` or `--x-file`; clap already rejects both together.
fn text(flag: Option<String>, file: Option<&PathBuf>, option: &str, stdin: &Stdin) -> Result<Option<String>> {
    match (flag, file) {
        (Some(text), _) => Ok(Some(text)),
        (None, Some(path)) => read_file(path, option, stdin).map(Some),
        (None, None) => Ok(None),
    }
}

/// Non-blank and within TaskHub's UTF-16 limit.
fn checked(field: &str, value: String, limit: usize) -> Result<String> {
    if value.trim().is_empty() {
        return Err(CliError::new(Code::InvalidInput, format!("{field} is empty.")));
    }
    let length = utf16_len(&value);
    if length > limit {
        return Err(CliError::new(
            Code::InvalidInput,
            format!("{field} is {length} characters; the limit is {limit}."),
        )
        .with_hint("Shorten it, or attach the details as a .md file."));
    }
    Ok(value)
}

fn checked_url(url: &str) -> Result<String> {
    let parsed = reqwest::Url::parse(url).map_err(|_| CliError::input(format!("{url:?} is not a URL.")))?;
    if !matches!(parsed.scheme(), "http" | "https") || utf16_len(url) > URL_LIMIT {
        return Err(CliError::input(format!(
            "{url:?} must be an http or https URL of at most {URL_LIMIT} characters."
        )));
    }
    Ok(url.to_owned())
}

fn status_name(status: StatusArg) -> Status {
    status.as_status()
}

/// Runs a planned operation through the journal and shapes the result.
fn commit(
    session: &Session,
    write: &WriteOpts,
    reference: Option<&RefMeta>,
    key: Option<&str>,
    steps: Vec<PlannedStep>,
    action: &str,
) -> Result<Success> {
    let committed =
        journal::start(&session.credential, &session.client, write.request_id.as_deref(), key, steps)?;
    Ok(receipt_success(committed.data, &committed.operation, reference, action))
}

fn receipt_success(
    data: Value,
    operation: &crate::error::OperationMeta,
    reference: Option<&RefMeta>,
    action: &str,
) -> Success {
    let human = match serde_json::from_value::<Receipt>(data.clone()) {
        Ok(receipt) => render::receipt(action, &receipt),
        Err(_) => format!("{action}: done."),
    };
    let human = if operation.replayed == Some(true) {
        format!("{human}\n(Replayed: this write had already happened.)")
    } else {
        human
    };
    Success::new(data, human).with_operation(operation).with_ref(reference)
}

fn uploads(key: &ItemKey, files: &[PathBuf]) -> Result<Vec<PlannedStep>> {
    files.iter().map(|file| journal::plan_upload(key.as_str(), file)).collect()
}

fn deadline_for(files: usize) -> std::time::Duration {
    if files > 0 { api::UPLOAD_DEADLINE } else { api::COMMAND_DEADLINE }
}

pub fn claim(args: RefArg) -> Result<Success> {
    let resolved = refs::resolve(args.reference.as_deref(), "claim")?;
    let key = resolved.key.as_str();
    let session = Session::open(api::COMMAND_DEADLINE)?;
    let step = PlannedStep::json("POST", format!("/items/{key}/claim"), json!({}));
    commit(&session, &args.write, Some(&resolved.meta), Some(key), vec![step], "Claimed")
}

pub fn next(args: NextArgs) -> Result<Success> {
    let projects = args.projects.iter().map(|p| project_key(p)).collect::<Result<Vec<_>>>()?;
    let query =
        super::Query::default().many("project", projects).one("type", args.item_type.map(|t| t.as_type()));
    let session = Session::open(api::COMMAND_DEADLINE)?;
    let mut lost_races = Vec::new();
    for _ in 0..3 {
        let (data, meta) = session.get("/items/next", query.pairs(), None)?;
        if data.is_null() {
            let human = if lost_races.is_empty() {
                "Nothing to do: no item is waiting for you."
            } else {
                "Nothing left to claim."
            };
            return Ok(Success::new(Value::Null, human).with_server_meta(&meta));
        }
        let item: NextItem = serde_json::from_value(data.clone()).map_err(|e| {
            CliError::new(Code::ProtocolError, format!("Unexpected /items/next response: {e}"))
        })?;
        let key = item.summary.key.clone();
        let why = match item.reason {
            NextReason::SentBack => "sent back to you",
            NextReason::Assigned => "assigned to you",
            NextReason::Unassigned => "unassigned",
            NextReason::Unknown(_) => "suggested",
        };
        let summary = format!(
            "{key}  {}\n{} · {} · {}",
            item.summary.title,
            why,
            render::status_label(&item.summary.status),
            item.summary.priority
        );
        // A sent-back item is already In Progress and yours: nothing to claim.
        if !args.claim || item.reason == NextReason::SentBack {
            let mut success = Success::new(data, summary).with_server_meta(&meta);
            success.meta.insert("claimed".into(), json!(false));
            return Ok(success);
        }
        let step = PlannedStep::json("POST", format!("/items/{key}/claim"), json!({}));
        match journal::start(&session.credential, &session.client, None, Some(&key), vec![step]) {
            Ok(committed) => {
                let mut data = data;
                if let (Value::Object(item), Value::Object(receipt)) = (&mut data, &committed.data) {
                    for field in ["status", "version"] {
                        if let Some(value) = receipt.get(field) {
                            item.insert(field.into(), value.clone());
                        }
                    }
                }
                let mut success = Success::new(data, format!("{summary}\nClaimed {key}: now In Progress."))
                    .with_operation(&committed.operation);
                success.meta.insert("claimed".into(), json!(true));
                if !lost_races.is_empty() {
                    success.meta.insert("lostRaces".into(), json!(lost_races));
                }
                return Ok(success);
            }
            Err(error) if error.code == Code::AlreadyClaimed => lost_races.push(key),
            Err(error) => return Err(error),
        }
    }
    Err(CliError::new(
        Code::AlreadyClaimed,
        "Other agents claimed the next items first, three times in a row.",
    )
    .with_detail("lostRaces", json!(lost_races))
    .with_hint("taskhub next --claim"))
}

pub fn comment(args: CommentArgs) -> Result<Success> {
    let stdin = Stdin::default();
    let body = text(args.body, args.body_file.as_ref(), "--body-file", &stdin)?
        .ok_or_else(|| CliError::input("Give the comment with --body or --body-file."))?;
    let body = checked("The comment", body, COMMENT_LIMIT)?;
    let resolved = refs::resolve(args.reference.as_deref(), "comment")?;
    let key = resolved.key.clone();
    let mut steps = uploads(&key, &args.attach)?;
    steps.push(PlannedStep::json_with_uploads(
        "POST",
        format!("/items/{key}/comments"),
        json!({ "body": body }),
    ));
    let session = Session::open(deadline_for(args.attach.len()))?;
    commit(&session, &args.write, Some(&resolved.meta), Some(key.as_str()), steps, "Commented on")
}

#[derive(serde::Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct SubmitInput {
    summary: Option<String>,
    testing: Option<String>,
    limitations: Option<String>,
    pr: Option<String>,
    #[serde(default)]
    links: Vec<String>,
    #[serde(default)]
    attach: Vec<PathBuf>,
}

pub fn submit(args: SubmitArgs) -> Result<Success> {
    let stdin = Stdin::default();
    let input: SubmitInput = match &args.input_file {
        Some(path) => serde_json::from_str(&read_file(path, "--input-file", &stdin)?)
            .map_err(|e| CliError::input(format!("--input-file is not valid: {e}")).with_hint(
                "Use {\"summary\": \"…\", \"testing\": \"…\", \"limitations\": \"…\", \"pr\": \"URL\", \"links\": [], \"attach\": []}",
            ))?,
        None => SubmitInput::default(),
    };
    let merge =
        |field: &str, from_flags: Option<String>, from_file: Option<String>| -> Result<Option<String>> {
            match (from_flags, from_file) {
                (Some(_), Some(_)) => {
                    Err(CliError::input(format!("{field} is given both as an option and in --input-file.")))
                }
                (a, b) => Ok(a.or(b)),
            }
        };
    let summary = merge(
        "summary",
        text(args.summary, args.summary_file.as_ref(), "--summary-file", &stdin)?,
        input.summary,
    )?
    .ok_or_else(|| CliError::input("Say what changed with --summary or --summary-file."))?;
    let testing = merge(
        "testing",
        text(args.testing, args.testing_file.as_ref(), "--testing-file", &stdin)?,
        input.testing,
    )?
    .ok_or_else(|| {
        CliError::input("Say how it was tested with --testing or --testing-file.")
            .with_hint("Report tests exactly as they ran; name the ones you did not run under --limitations.")
    })?;
    let limitations = merge(
        "limitations",
        text(args.limitations, args.limitations_file.as_ref(), "--limitations-file", &stdin)?,
        input.limitations,
    )?;
    let summary = checked("The summary", summary, COMMENT_LIMIT)?;
    let testing = checked("The testing notes", testing, COMMENT_LIMIT)?;
    let limitations = match limitations {
        Some(text) if !text.trim().is_empty() => Some(checked("The limitations", text, COMMENT_LIMIT)?),
        _ => None,
    };
    let resolved = refs::resolve(args.reference.as_deref(), "submit")?;
    let key = resolved.key.clone();
    let from = status_name(args.from);
    if from == Status::Done {
        return Err(sign_off_error(&key));
    }
    // `--pr auto` asks gh before anything is written, so a missing PR fails cleanly.
    let pr = match merge("pr", args.pr, input.pr)? {
        Some(value) if value == "auto" => Some(crate::git::pull_request_url()?),
        Some(url) => Some(checked_url(&url)?),
        None => None,
    };
    let mut links: Vec<Value> = Vec::new();
    if let Some(url) = &pr {
        links.push(json!({ "kind": "pr", "url": url }));
    }
    for url in args.links.iter().chain(input.links.iter()) {
        links.push(json!({ "kind": "link", "url": checked_url(url)? }));
    }
    let mut files = args.attach.clone();
    files.extend(input.attach);
    let mut body = json!({ "from": from, "summary": summary, "testing": testing });
    if let Some(limitations) = limitations {
        body["limitations"] = json!(limitations);
    }
    if !links.is_empty() {
        body["links"] = json!(links);
    }
    let mut steps = uploads(&key, &files)?;
    steps.push(PlannedStep::json_with_uploads("POST", format!("/items/{key}/submissions"), body));
    let session = Session::open(deadline_for(files.len()))?;
    commit(&session, &args.write, Some(&resolved.meta), Some(key.as_str()), steps, "Submitted")
}

pub fn reject(args: RejectArgs) -> Result<Success> {
    let stdin = Stdin::default();
    let reason = text(args.reason, args.reason_file.as_ref(), "--reason-file", &stdin)?
        .ok_or_else(|| CliError::input("Give the reason with --reason or --reason-file."))?;
    let reason = checked("The reason", reason, COMMENT_LIMIT)?;
    let resolved = refs::resolve(Some(&args.reference), "reject")?;
    let key = resolved.key.clone();
    let mut steps = uploads(&key, &args.attach)?;
    steps.push(PlannedStep::json_with_uploads(
        "POST",
        format!("/items/{key}/rejections"),
        json!({ "reason": reason }),
    ));
    let session = Session::open(deadline_for(args.attach.len()))?;
    commit(&session, &args.write, Some(&resolved.meta), Some(key.as_str()), steps, "Sent back")
}

pub fn inbox_done(args: InboxDoneArgs) -> Result<Success> {
    let body = if args.all {
        json!({ "all": true })
    } else {
        let ids = args
            .ids
            .iter()
            .map(|id| {
                uuid::Uuid::parse_str(id)
                    .map(|u| u.to_string())
                    .map_err(|_| CliError::input(format!("{id:?} is not an inbox entry ID.")))
            })
            .collect::<Result<Vec<_>>>()?;
        json!({ "ids": ids })
    };
    let session = Session::open(api::COMMAND_DEADLINE)?;
    let committed = journal::start(
        &session.credential,
        &session.client,
        args.write.request_id.as_deref(),
        None,
        vec![PlannedStep::json("POST", "/notifications/read".into(), body)],
    )?;
    let count = committed.data.get("markedRead").and_then(Value::as_u64).unwrap_or(0);
    Ok(Success::new(
        committed.data.clone(),
        format!("Marked {count} entr{} read.", if count == 1 { "y" } else { "ies" }),
    )
    .with_operation(&committed.operation))
}

pub fn link(args: LinkArgs) -> Result<Success> {
    let (reference, rest) = refs::split_leading_ref(&args.positionals);
    let [url] = rest else {
        return Err(CliError::input("Give one URL: taskhub link [REF] URL"));
    };
    let url = checked_url(url)?;
    let resolved = refs::resolve(reference, "link")?;
    let key = resolved.key.clone();
    let session = Session::open(api::COMMAND_DEADLINE)?;
    let step = PlannedStep::json(
        "POST",
        format!("/items/{key}/links"),
        json!({ "kind": args.kind.as_kind(), "url": url }),
    );
    commit(&session, &args.write, Some(&resolved.meta), Some(key.as_str()), vec![step], "Linked")
}

pub fn attachments_add(args: AttachmentsAddArgs) -> Result<Success> {
    let (reference, files) = refs::split_leading_ref(&args.positionals);
    let resolved = refs::resolve(reference, "attachments add")?;
    let key = resolved.key.clone();
    let paths: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
    let steps = uploads(&key, &paths)?;
    let session = Session::open(deadline_for(paths.len()))?;
    let committed = journal::start(
        &session.credential,
        &session.client,
        args.write.request_id.as_deref(),
        Some(key.as_str()),
        steps,
    )?;
    // Report every uploaded file, not only the last step's receipt.
    let ids: Vec<Value> = committed
        .entry
        .steps
        .iter()
        .filter_map(|step| step.receipt.as_ref()?.get("attachmentIds")?.as_array().cloned())
        .flatten()
        .collect();
    let mut data = committed.data.clone();
    data["attachmentIds"] = json!(ids);
    Ok(Success::new(
        data,
        format!("Attached {} file{} to {key}.", ids.len(), if ids.len() == 1 { "" } else { "s" }),
    )
    .with_operation(&committed.operation)
    .with_ref(Some(&resolved.meta)))
}

fn sign_off_error(key: &ItemKey) -> CliError {
    let mut error =
        CliError::new(Code::SignOffRequiresPerson, "Only a person can move an item into or out of Done.");
    error.hint = Some(format!("Ask a Tester to sign off on {key} in TaskHub."));
    error
}

pub fn items_move(args: ItemsMoveArgs) -> Result<Success> {
    let (reference, rest) = refs::split_leading_ref(&args.positionals);
    let [status] = rest else {
        return Err(CliError::input("Give the new status: taskhub items move [REF] STATUS --from STATUS"));
    };
    let to = match Status::parse_known(status) {
        Some(status) => status,
        None => {
            return Err(CliError::input(format!("{status:?} is not a status."))
                .with_detail("allowed", json!(["todo", "in_progress", "dev_done"])));
        }
    };
    let resolved = refs::resolve(reference, "items move")?;
    let key = resolved.key.clone();
    let from = status_name(args.from);
    if to == Status::Done || from == Status::Done {
        // Refused locally: no request is sent.
        return Err(sign_off_error(&key));
    }
    let session = Session::open(api::COMMAND_DEADLINE)?;
    let step =
        PlannedStep::json("POST", format!("/items/{key}/transitions"), json!({ "from": from, "to": to }));
    commit(&session, &args.write, Some(&resolved.meta), Some(key.as_str()), vec![step], "Moved")
}

pub fn items_update(args: ItemsUpdateArgs) -> Result<Success> {
    let stdin = Stdin::default();
    let mut changes = ItemChanges::default();
    if let Some(item_type) = args.item_type {
        changes.item_type = Patch::Value(item_type.as_type());
    }
    if let Some(title) = args.title {
        changes.title = Patch::Value(checked("The title", title, TITLE_LIMIT)?);
    }
    if let Some(description) = text(args.description, args.body_file.as_ref(), "--body-file", &stdin)? {
        if utf16_len(&description) > DESCRIPTION_LIMIT {
            return Err(CliError::input(format!(
                "The description is longer than {DESCRIPTION_LIMIT} characters."
            )));
        }
        changes.description = Patch::Value(description);
    }
    if let Some(priority) = args.priority {
        changes.priority = Patch::Value(priority.as_priority());
    }
    match args.assignee.as_deref() {
        Some("none") => changes.assignee = Patch::Null,
        Some(username) => changes.assignee = Patch::Value(username.to_owned()),
        None => {}
    }
    if args.no_labels {
        changes.labels = Patch::Value(Vec::new());
    } else if !args.labels.is_empty() {
        changes.labels = Patch::Value(args.labels.clone());
    }
    if changes.is_empty() {
        return Err(CliError::input("Nothing to change.").with_hint("Give at least one of --title, --body-file, --priority, --assignee, --label, --no-labels, --type."));
    }
    if args.if_version == 0 {
        return Err(CliError::input("--if-version must be the item's current version (1 or more)."));
    }
    let resolved = refs::resolve(args.reference.as_deref(), "items update")?;
    let key = resolved.key.clone();
    let body = serde_json::to_value(PatchItemBody { expected_version: args.if_version, changes })
        .map_err(|e| CliError::internal(format!("Could not encode the change: {e}")))?;
    let session = Session::open(api::COMMAND_DEADLINE)?;
    let step = PlannedStep::json("PATCH", format!("/items/{key}"), body);
    commit(&session, &args.write, Some(&resolved.meta), Some(key.as_str()), vec![step], "Updated")
}

#[derive(serde::Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct CreateInput {
    project: Option<String>,
    #[serde(rename = "type")]
    item_type: Option<String>,
    title: Option<String>,
    description: Option<String>,
    priority: Option<String>,
    assignee: Option<String>,
    labels: Option<Vec<String>>,
}

pub fn items_create(args: ItemsCreateArgs) -> Result<Success> {
    let stdin = Stdin::default();
    let input: CreateInput = match &args.input_file {
        Some(path) => serde_json::from_str(&read_file(path, "--input-file", &stdin)?)
            .map_err(|e| CliError::input(format!("--input-file is not valid: {e}")))?,
        None => CreateInput::default(),
    };
    let both =
        |field: &str| CliError::input(format!("{field} is given both as an option and in --input-file."));
    let project = match (args.project, input.project) {
        (Some(_), Some(_)) => return Err(both("project")),
        (a, b) => a.or(b).or(crate::config::default_project()?),
    }
    .ok_or_else(|| {
        CliError::input("Which project? Give --project KEY.").with_hint("taskhub projects list")
    })?;
    let project = project_key(&project)?;
    let item_type = match (args.item_type, input.item_type) {
        (Some(_), Some(_)) => return Err(both("type")),
        (Some(t), None) => t.as_type(),
        (None, Some(t)) => ItemType::parse_known(&t)
            .ok_or_else(|| CliError::input(format!("type must be task or bug, not {t:?}.")))?,
        (None, None) => return Err(CliError::input("Give --type task or --type bug.")),
    };
    let title = match (args.title, input.title) {
        (Some(_), Some(_)) => return Err(both("title")),
        (a, b) => a.or(b).ok_or_else(|| CliError::input("Give a --title."))?,
    };
    let title = checked("The title", title, TITLE_LIMIT)?;
    let description =
        match (text(args.description, args.body_file.as_ref(), "--body-file", &stdin)?, input.description) {
            (Some(_), Some(_)) => return Err(both("description")),
            (a, b) => a.or(b),
        };
    if description.as_deref().is_some_and(|d| utf16_len(d) > DESCRIPTION_LIMIT) {
        return Err(CliError::input(format!(
            "The description is longer than {DESCRIPTION_LIMIT} characters."
        )));
    }
    let priority =
        match (args.priority, input.priority) {
            (Some(_), Some(_)) => return Err(both("priority")),
            (Some(p), None) => Some(p.as_priority()),
            (None, Some(p)) => Some(Priority::parse_known(&p).ok_or_else(|| {
                CliError::input(format!("priority must be high, medium or low, not {p:?}."))
            })?),
            (None, None) => None,
        };
    let assignee = match (args.assignee, input.assignee) {
        (Some(_), Some(_)) => return Err(both("assignee")),
        (a, b) => a.or(b),
    };
    let labels = match (args.labels.is_empty(), input.labels) {
        (false, Some(_)) => return Err(both("labels")),
        (false, None) => Some(args.labels),
        (true, labels) => labels,
    };
    let body = CreateItemBody { item_type, title, description, priority, assignee, labels };
    let body = serde_json::to_value(body)
        .map_err(|e| CliError::internal(format!("Could not encode the item: {e}")))?;
    let session = Session::open(api::COMMAND_DEADLINE)?;
    let step = PlannedStep::json("POST", format!("/projects/{project}/items"), body);
    commit(&session, &args.write, None, None, vec![step], "Created")
}

pub fn comments_edit(args: CommentsEditArgs) -> Result<Success> {
    let id = uuid::Uuid::parse_str(&args.id)
        .map_err(|_| {
            CliError::input(format!("{:?} is not a comment ID.", args.id))
                .with_hint("taskhub comments list REF")
        })?
        .to_string();
    let stdin = Stdin::default();
    let body = text(args.body, args.body_file.as_ref(), "--body-file", &stdin)?
        .ok_or_else(|| CliError::input("Give the new text with --body or --body-file."))?;
    let body = checked("The comment", body, COMMENT_LIMIT)?;
    let session = Session::open(api::COMMAND_DEADLINE)?;
    let step = PlannedStep::json("PATCH", format!("/comments/{id}"), json!({ "body": body }));
    commit(&session, &args.write, None, None, vec![step], "Edited the comment on")
}

/// `taskhub branch`: needs the title, so it reads the item; `--create` switches with git.
pub fn branch(args: BranchArgs) -> Result<Success> {
    let resolved = refs::resolve(args.reference.as_deref(), "branch")?;
    let key = resolved.key.as_str();
    let session = Session::open(api::COMMAND_DEADLINE)?;
    let (data, _) = session.get(&format!("/items/{key}"), &[], Some(key))?;
    let title = data.get("title").and_then(Value::as_str).unwrap_or_default();
    let name = crate::git::branch_name(key, title);
    if !args.create {
        return Ok(Success::new(json!({ "branch": name }), name.clone()).with_ref(Some(&resolved.meta)));
    }
    let created = crate::git::switch_to(&name)?;
    let verb = if created { "Created and switched to" } else { "Switched to" };
    Ok(Success::new(json!({ "branch": name, "created": created }), format!("{verb} {name}."))
        .with_ref(Some(&resolved.meta)))
}

/// `taskhub open`: the only command that launches anything, and only on a terminal.
pub fn open(args: RefArg) -> Result<Success> {
    let resolved = refs::resolve(args.reference.as_deref(), "open")?;
    let key = &resolved.key;
    let credential = crate::credentials::load()?;
    let number = key.as_str().rsplit_once('-').map_or("", |(_, n)| n);
    let url = credential.origin.url(&format!("/projects/{}/items/{number}", key.project()));
    if std::io::stdout().is_terminal() {
        let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
        let launched = std::process::Command::new(opener)
            .arg(&url)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        let human = if launched { format!("Opened {url}") } else { url.clone() };
        return Ok(
            Success::new(json!({ "url": url, "opened": launched }), human).with_ref(Some(&resolved.meta))
        );
    }
    Ok(Success::new(json!({ "url": url, "opened": false }), url.clone()).with_ref(Some(&resolved.meta)))
}

pub fn pending() -> Result<Success> {
    let entries: Vec<journal::Entry> =
        journal::all()?.into_iter().filter(|e| e.state == journal::State::Pending).collect();
    let data: Vec<Value> = entries
        .iter()
        .map(|entry| {
            let done = entry.steps.iter().filter(|s| s.state == journal::State::Committed).count();
            json!({
                "id": entry.id,
                "createdAt": entry.created_at,
                "age": crate::clock::relative(&entry.created_at),
                "command": entry.command,
                "key": entry.key,
                "origin": entry.origin,
                "owner": entry.owner,
                "stepsDone": done,
                "steps": entry.steps.len(),
            })
        })
        .collect();
    let human = if entries.is_empty() {
        "No pending writes.".to_owned()
    } else {
        let rows: Vec<Vec<String>> = entries
            .iter()
            .map(|e| {
                vec![e.id.clone(), crate::clock::relative(&e.created_at), crate::output::clip(&e.command, 60)]
            })
            .collect();
        format!(
            "{}\nFor each: taskhub retry OP, or taskhub pending discard OP after checking it yourself.",
            crate::output::table(&["OPERATION", "AGE", "COMMAND"], &rows)
        )
    };
    Ok(Success::new(json!(data), human))
}

pub fn pending_discard(args: OperationArg) -> Result<Success> {
    let entry = journal::discard(&args.id)?;
    Ok(Success::new(json!({ "id": entry.id, "discarded": true }), format!("Discarded {}.", entry.id)))
}

pub fn retry(args: RetryArgs) -> Result<Success> {
    let entry = journal::Entry::load(&args.id)?;
    let uploads = entry.steps.iter().filter(|s| matches!(s.body, journal::Body::Upload { .. })).count();
    let session = Session::open(deadline_for(uploads))?;
    let committed = journal::retry(&args.id, &session.credential, &session.client)?;
    let reference = entry.key.as_ref().map(|key| RefMeta { key: key.clone(), source: "journal" });
    Ok(receipt_success(committed.data, &committed.operation, reference.as_ref(), "Completed the write to"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Outcome;

    #[test]
    fn text_limits_count_utf16_and_reject_blank() {
        assert!(checked("x", " \n ".into(), 10).is_err());
        assert!(checked("x", "😀".repeat(5), 10).is_ok());
        assert!(checked("x", "😀".repeat(6), 10).is_err());
    }

    #[test]
    fn urls_must_be_http() {
        assert!(checked_url("https://github.com/o/r/pull/1").is_ok());
        assert!(checked_url("ftp://x.example").is_err());
        assert!(checked_url("not a url").is_err());
    }

    #[test]
    fn outcome_names_match_the_plan() {
        assert_eq!(serde_json::to_value(Outcome::Committed).unwrap(), "committed");
        assert_eq!(serde_json::to_value(Outcome::Unknown).unwrap(), "unknown");
    }
}

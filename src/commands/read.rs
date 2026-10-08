//! Read commands: nothing here writes to TaskHub, and nothing marks the inbox read.

use super::{Query, Session, default_deadline, render};
use crate::cli::*;
use crate::error::{CliError, Code, Result};
use crate::output::Success;
use crate::refs::{self, project_key};
use crate::types::*;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

fn typed<T: DeserializeOwned>(value: &Value) -> Result<T> {
    serde_json::from_value(value.clone())
        .map_err(|e| CliError::new(Code::ProtocolError, format!("Unexpected response from TaskHub: {e}")))
}

fn next_cursor(meta: &Value) -> Option<String> {
    meta.pointer("/page/nextCursor").and_then(Value::as_str).map(str::to_owned)
}

pub fn show(args: ShowArgs) -> Result<Success> {
    let resolved = refs::resolve(args.reference.as_deref(), "show")?;
    let key = resolved.key.as_str();
    let session = Session::open(default_deadline())?;
    let query =
        Query::default().one("comments", Some(args.comments)).one("attachments", Some(args.attachments));
    let (data, meta) = session.get(&format!("/items/{key}/context"), query.pairs(), Some(key))?;
    let context: Context = typed(&data)?;
    Ok(Success::new(data, render::context(&context)).with_server_meta(&meta).with_ref(Some(&resolved.meta)))
}

pub fn items_query(args: &ItemsListArgs) -> Result<Query> {
    let projects = args.projects.iter().map(|p| project_key(p)).collect::<Result<Vec<_>>>()?;
    Ok(Query::default()
        .many("project", projects)
        .one("type", args.item_type.map(|t| t.as_type()))
        .many("status", args.statuses.iter().map(|s| s.as_status()))
        .one("priority", args.priority.map(|p| p.as_priority()))
        .one("assignee", args.assignee.as_deref())
        .many("label", &args.labels)
        .one("q", args.search.as_deref())
        .flag("agent", args.agent)
        .one("sort", args.sort.map(|s| if s == SortArg::Priority { "priority" } else { "number" }))
        .one("limit", args.limit)
        .one("cursor", args.cursor.as_deref()))
}

fn list_items(query: &Query, command: &str) -> Result<Success> {
    let session = Session::open(default_deadline())?;
    let (data, meta) = session.get("/items", query.pairs(), None)?;
    let items: Vec<ItemSummary> = typed(&data)?;
    let human =
        format!("{}{}", render::item_rows(&items), render::more(next_cursor(&meta).as_deref(), command));
    Ok(Success::new(data, human).with_server_meta(&meta))
}

pub fn items_list(args: ItemsListArgs) -> Result<Success> {
    list_items(&items_query(&args)?, "taskhub items list")
}

pub fn mine(args: MineArgs) -> Result<Success> {
    let statuses =
        if args.statuses.is_empty() { vec![StatusArg::Todo, StatusArg::InProgress] } else { args.statuses };
    let query = Query::default()
        .one("assignee", Some("me"))
        .many("status", statuses.iter().map(|s| s.as_status()))
        .one("sort", Some("priority"))
        .one("limit", Some(100));
    list_items(&query, "taskhub items list --assignee me")
}

pub fn items_get(args: ItemsGetArgs) -> Result<Success> {
    let resolved = refs::resolve(args.reference.as_deref(), "items get")?;
    let key = resolved.key.as_str();
    let session = Session::open(default_deadline())?;
    let (data, meta) = session.get(&format!("/items/{key}"), &[], Some(key))?;
    let item: Item = typed(&data)?;
    Ok(Success::new(data, render::item(&item)).with_server_meta(&meta).with_ref(Some(&resolved.meta)))
}

fn paged(args: &PagedRefArgs) -> Query {
    Query::default().one("limit", args.limit).one("cursor", args.cursor.as_deref())
}

pub fn activity(args: PagedRefArgs) -> Result<Success> {
    let resolved = refs::resolve(args.reference.as_deref(), "items activity")?;
    let key = resolved.key.as_str();
    let session = Session::open(default_deadline())?;
    let (data, meta) = session.get(&format!("/items/{key}/activity"), paged(&args).pairs(), Some(key))?;
    let events: Vec<Activity> = typed(&data)?;
    let human = format!(
        "{}{}",
        render::activity(&events),
        render::more(next_cursor(&meta).as_deref(), &format!("taskhub items activity {key}"))
    );
    Ok(Success::new(data, human).with_server_meta(&meta).with_ref(Some(&resolved.meta)))
}

pub fn comments_list(args: PagedRefArgs) -> Result<Success> {
    let resolved = refs::resolve(args.reference.as_deref(), "comments list")?;
    let key = resolved.key.as_str();
    let session = Session::open(default_deadline())?;
    let (data, meta) = session.get(&format!("/items/{key}/comments"), paged(&args).pairs(), Some(key))?;
    let list: Vec<Comment> = typed(&data)?;
    let human = format!(
        "{}{}",
        render::comments(&list),
        render::more(next_cursor(&meta).as_deref(), &format!("taskhub comments list {key}"))
    );
    Ok(Success::new(data, human).with_server_meta(&meta).with_ref(Some(&resolved.meta)))
}

pub fn attachments_list(args: PagedRefArgs) -> Result<Success> {
    let resolved = refs::resolve(args.reference.as_deref(), "attachments list")?;
    let key = resolved.key.as_str();
    let session = Session::open(default_deadline())?;
    let (data, meta) = session.get(&format!("/items/{key}/attachments"), paged(&args).pairs(), Some(key))?;
    let files: Vec<Attachment> = typed(&data)?;
    let human = format!(
        "{}{}",
        render::attachments(&files),
        render::more(next_cursor(&meta).as_deref(), &format!("taskhub attachments list {key}"))
    );
    Ok(Success::new(data, human).with_server_meta(&meta).with_ref(Some(&resolved.meta)))
}

pub fn projects_list() -> Result<Success> {
    let session = Session::open(default_deadline())?;
    let (data, meta) = session.get("/projects", &[("limit".into(), "100".into())], None)?;
    let list: Vec<Project> = typed(&data)?;
    Ok(Success::new(data, render::projects(&list)).with_server_meta(&meta))
}

pub fn project_show(args: ProjectShowArgs) -> Result<Success> {
    let key = project_key(&args.key)?;
    let session = Session::open(default_deadline())?;
    let (data, meta) = session.get(&format!("/projects/{key}"), &[], None)?;
    let detail: ProjectDetail = typed(&data)?;
    Ok(Success::new(data, render::project(&detail)).with_server_meta(&meta))
}

pub fn inbox_query(args: &InboxArgs) -> Query {
    let kinds = args.kinds.iter().map(|kind| match kind {
        KindArg::Mention => "mention",
        KindArg::Assigned => "assigned",
        KindArg::Rejected => "rejected",
        KindArg::Review => "review",
    });
    Query::default()
        .flag("unread", args.unread)
        .many("kind", kinds)
        .one("limit", args.limit)
        .one("cursor", args.cursor.as_deref())
}

pub fn inbox(args: InboxArgs) -> Result<Success> {
    let session = Session::open(default_deadline())?;
    let (data, meta) = session.get("/notifications", inbox_query(&args).pairs(), None)?;
    let list: Vec<Notification> = typed(&data)?;
    let human = format!(
        "{}{}",
        render::notifications(&list),
        render::more(next_cursor(&meta).as_deref(), "taskhub inbox")
    );
    Ok(Success::new(data, human).with_server_meta(&meta))
}

/// Streams into a private temporary file next to the destination, then publishes it without replacing an
/// existing file (unless `--force`). The server's filename is never used as a path.
pub fn download(args: AttachmentsDownloadArgs) -> Result<Success> {
    let id = uuid::Uuid::parse_str(&args.id)
        .map_err(|_| {
            CliError::input(format!("{:?} is not a file ID.", args.id))
                .with_hint("taskhub attachments list REF")
        })?
        .to_string();
    let destination = args.output.clone();
    if !args.force && fs::symlink_metadata(&destination).is_ok() {
        return Err(CliError::new(Code::OutputExists, format!("{} already exists.", destination.display()))
            .with_hint("Choose another --output path, or add --force to replace it."));
    }
    let dir = match destination.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => Path::new(".").to_path_buf(),
    };
    let temp = dir.join(format!(".taskhub-download-{}.part", uuid::Uuid::new_v4()));
    let session = Session::open(default_deadline())?;
    let result = (|| -> Result<u64> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)
            .map_err(|e| CliError::input(format!("Could not write in {}: {e}", dir.display())))?;
        let size = session.client.download(&id, &mut file, &Default::default())?;
        file.flush()
            .and_then(|()| file.sync_all())
            .map_err(|e| CliError::internal(format!("Could not save the file: {e}")))?;
        publish(&temp, &destination, args.force)?;
        Ok(size)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    let size = result?;
    Ok(Success::new(
        json!({ "id": id, "path": destination, "bytes": size }),
        format!("Saved {} ({}).", destination.display(), render::size(size)),
    ))
}

fn publish(temp: &Path, destination: &Path, force: bool) -> Result<()> {
    if force {
        return fs::rename(temp, destination)
            .map_err(|e| CliError::internal(format!("Could not save {}: {e}", destination.display())));
    }
    use rustix::fs::{CWD, RenameFlags, renameat_with};
    renameat_with(CWD, temp, CWD, destination, RenameFlags::NOREPLACE).map_err(|e| {
        if e == rustix::io::Errno::EXIST {
            CliError::new(Code::OutputExists, format!("{} already exists.", destination.display()))
                .with_hint("Choose another --output path, or add --force to replace it.")
        } else {
            CliError::internal(format!("Could not save {}: {e}", destination.display()))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The query TaskHub's listItems request fixture describes, built from the equivalent flags.
    #[test]
    fn items_query_matches_the_contract_fixture() {
        let args = ItemsListArgs {
            projects: vec!["web".into(), "ops".into()],
            item_type: Some(ItemTypeArg::Bug),
            statuses: vec![StatusArg::Todo, StatusArg::InProgress],
            priority: None,
            assignee: Some("me".into()),
            labels: vec!["accessibility".into()],
            search: None,
            agent: true,
            sort: Some(SortArg::Priority),
            limit: Some(20),
            cursor: None,
        };
        let mut got: Vec<(String, String)> = items_query(&args).unwrap().pairs().to_vec();
        let fixture: Value = serde_json::from_str(
            &fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/api/v1/fixtures/requests/listItems.json"
            ))
            .unwrap(),
        )
        .unwrap();
        let mut want = Vec::new();
        for (key, value) in fixture["query"].as_object().unwrap() {
            match value {
                Value::Array(values) => {
                    want.extend(values.iter().map(|v| (key.clone(), v.as_str().unwrap().to_owned())))
                }
                Value::String(value) => want.push((key.clone(), value.clone())),
                _ => unreachable!(),
            }
        }
        // TaskHub uppercases project keys itself; the CLI normalizes them first.
        for pair in &mut want {
            if pair.0 == "project" {
                pair.1 = pair.1.to_uppercase();
            }
        }
        got.sort();
        want.sort();
        assert_eq!(got, want);
    }

    #[test]
    fn inbox_query_matches_the_contract_fixture() {
        let args = InboxArgs {
            action: None,
            unread: true,
            kinds: vec![KindArg::Review, KindArg::Mention],
            limit: Some(20),
            cursor: None,
        };
        let got = inbox_query(&args).pairs().to_vec();
        assert_eq!(
            got,
            vec![
                ("unread".into(), "true".into()),
                ("kind".into(), "review".into()),
                ("kind".into(), "mention".into()),
                ("limit".into(), "20".into())
            ]
        );
    }
}

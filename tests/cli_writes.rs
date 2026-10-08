//! C2: the journal and the write commands, including crashes between sending and receiving.

mod common;

use common::*;
use serde_json::{Value, json};
use std::process::{Child, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

fn receipt(key: &str, version: u64, status: &str, extra: Value, op: &str, replayed: bool) -> String {
    let mut data = json!({ "key": key, "version": version, "status": status });
    if let (Value::Object(data), Value::Object(extra)) = (&mut data, extra) {
        data.extend(extra);
    }
    json!({ "data": data, "meta": { "apiVersion": 1, "requestId": "r", "operation": { "id": op, "replayed": replayed } } }).to_string()
}

fn me(username: &str) -> Reply {
    let mut value: Value = serde_json::from_str(&fixture("success/getMe.json")).unwrap();
    value["data"]["owner"]["username"] = json!(username);
    Reply::json(200, value.to_string())
}

fn key_of(request: &Recorded) -> String {
    request.header("Idempotency-Key").unwrap_or_default().to_owned()
}

fn wait_for(server: &MockServer, method: &str, prefix: &str, count: usize) {
    let start = Instant::now();
    while server.count(method, prefix) < count {
        assert!(start.elapsed() < Duration::from_secs(10), "timed out waiting for {method} {prefix}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn spawn(sandbox: &Sandbox, args: &[&str]) -> Child {
    sandbox
        .command()
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

fn write_file(sandbox: &Sandbox, name: &str, bytes: &[u8]) {
    std::fs::write(sandbox.work().join(name), bytes).unwrap();
}

#[test]
fn comment_with_files_uploads_first_then_references_them() {
    let server = MockServer::start(|r| match (r.method.as_str(), r.url.as_str()) {
        ("GET", "/api/v1/me") => me("dev.example"),
        ("POST", "/api/v1/items/WEB-12/attachments") => Reply::json(
            200,
            receipt(
                "WEB-12",
                8,
                "in_progress",
                json!({"attachmentIds": ["00000000-0000-4000-8000-0000000000aa"]}),
                &key_of(r),
                false,
            ),
        ),
        ("POST", "/api/v1/items/WEB-12/comments") => Reply::json(
            200,
            receipt(
                "WEB-12",
                8,
                "in_progress",
                json!({"commentId": "00000000-0000-4000-8000-000000000005", "mentioned": ["qa.example"], "unmatchedMentions": ["ghost"]}),
                &key_of(r),
                false,
            ),
        ),
        _ => Reply::problem(404, "NOT_FOUND"),
    });
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.login(&server.origin, &token('C')).code, 0);
    write_file(&sandbox, "shot.png", b"\x89PNG fake");
    let run =
        sandbox.run(&["comment", "WEB-12", "--body", "Done. @qa.example @ghost", "--attach", "shot.png"]);
    assert_eq!(run.code, 0, "{}", run.stdout);
    let out = run.json();
    assert_eq!(out["data"]["unmatchedMentions"], json!(["ghost"]));
    assert_eq!(out["meta"]["operation"]["outcome"], "committed");

    let writes: Vec<Recorded> = server.requests().into_iter().filter(|r| r.method == "POST").collect();
    assert_eq!(writes.len(), 2);
    let (upload, comment) = (&writes[0], &writes[1]);
    assert!(upload.header("Content-Type").unwrap().starts_with("multipart/form-data; boundary=taskhub-"));
    assert!(String::from_utf8_lossy(&upload.body).contains("filename=\"shot.png\""));
    assert_eq!(
        comment.json(),
        json!({"body": "Done. @qa.example @ghost", "attachmentIds": ["00000000-0000-4000-8000-0000000000aa"]})
    );
    assert_ne!(key_of(upload), key_of(comment), "each step has its own Idempotency-Key");
    assert!(uuid::Uuid::parse_str(&key_of(comment)).is_ok());
}

#[test]
fn a_write_killed_after_sending_is_pending_and_retry_finishes_it_once() {
    let comments = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&comments);
    let server = MockServer::start(move |r| match (r.method.as_str(), r.url.as_str()) {
        ("GET", "/api/v1/me") => me("dev.example"),
        ("POST", "/api/v1/items/WEB-12/comments") => {
            let first = seen.fetch_add(1, Ordering::SeqCst) == 0;
            let reply = Reply::json(
                200,
                receipt(
                    "WEB-12",
                    8,
                    "in_progress",
                    json!({"commentId": "00000000-0000-4000-8000-000000000005", "mentioned": [], "unmatchedMentions": []}),
                    &key_of(r),
                    !first,
                ),
            );
            // The first answer arrives too late: the process is killed while waiting.
            if first { reply.delayed(5000) } else { reply }
        }
        _ => Reply::problem(404, "NOT_FOUND"),
    });
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.login(&server.origin, &token('K')).code, 0);
    let mut child = spawn(&sandbox, &["comment", "WEB-12", "--body", "Progress update"]);
    wait_for(&server, "POST", "/api/v1/items/WEB-12/comments", 1);
    child.kill().unwrap();
    child.wait().unwrap();

    let pending = sandbox.run(&["pending"]).json();
    let entries = pending["data"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert!(entries[0]["command"].as_str().unwrap().contains("comment WEB-12"));
    let op = entries[0]["id"].as_str().unwrap().to_owned();

    let retry = sandbox.run(&["retry", &op]);
    assert_eq!(retry.code, 0, "{}", retry.stdout);
    assert_eq!(
        retry.json()["meta"]["operation"],
        json!({"id": op, "replayed": true, "outcome": "committed"})
    );
    let sent: Vec<Recorded> = server.requests().into_iter().filter(|r| r.method == "POST").collect();
    assert_eq!(sent.len(), 2);
    assert_eq!(key_of(&sent[0]), op, "a one-step write uses the operation ID as its key");
    assert_eq!(key_of(&sent[0]), key_of(&sent[1]));
    assert_eq!(sent[0].body, sent[1].body, "the retry resends identical bytes");
    assert!(sandbox.run(&["pending"]).json()["data"].as_array().unwrap().is_empty());
    // Retrying a finished operation sends nothing more.
    assert_eq!(sandbox.run(&["retry", &op]).code, 0);
    assert_eq!(server.count("POST", "/api/v1/items/WEB-12/comments"), 2);
}

#[test]
fn an_interrupted_submission_resumes_without_uploading_again() {
    let submissions = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&submissions);
    let server = MockServer::start(move |r| match (r.method.as_str(), r.url.as_str()) {
        ("GET", "/api/v1/me") => me("dev.example"),
        ("POST", "/api/v1/items/WEB-12/attachments") => Reply::json(
            200,
            receipt(
                "WEB-12",
                8,
                "in_progress",
                json!({"attachmentIds": ["00000000-0000-4000-8000-0000000000bb"]}),
                &key_of(r),
                false,
            ),
        ),
        ("POST", "/api/v1/items/WEB-12/submissions") => {
            let first = seen.fetch_add(1, Ordering::SeqCst) == 0;
            let reply = Reply::json(
                200,
                receipt(
                    "WEB-12",
                    9,
                    "dev_done",
                    json!({
                "commentId": "00000000-0000-4000-8000-000000000005", "mentioned": [], "unmatchedMentions": [],
                "attachmentIds": ["00000000-0000-4000-8000-0000000000bb"], "linkIds": []}),
                    &key_of(r),
                    !first,
                ),
            );
            if first { reply.delayed(5000) } else { reply }
        }
        _ => Reply::problem(404, "NOT_FOUND"),
    });
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.login(&server.origin, &token('S')).code, 0);
    write_file(&sandbox, "evidence.md", b"# Evidence\n");
    let mut child = spawn(
        &sandbox,
        &["submit", "WEB-12", "--summary", "Fixed", "--testing", "Unit tests", "--attach", "evidence.md"],
    );
    wait_for(&server, "POST", "/api/v1/items/WEB-12/submissions", 1);
    child.kill().unwrap();
    child.wait().unwrap();
    let op = sandbox.run(&["pending"]).json()["data"][0]["id"].as_str().unwrap().to_owned();
    let retry = sandbox.run(&["retry", &op]);
    assert_eq!(retry.code, 0, "{}", retry.stdout);
    assert_eq!(retry.json()["data"]["status"], "dev_done");
    assert_eq!(server.count("POST", "/api/v1/items/WEB-12/attachments"), 1, "the finished upload is reused");
    let subs: Vec<Recorded> =
        server.requests().into_iter().filter(|r| r.url.ends_with("/submissions")).collect();
    assert_eq!(subs.len(), 2);
    assert_eq!(subs[0].body, subs[1].body);
    assert_eq!(subs[1].json()["attachmentIds"], json!(["00000000-0000-4000-8000-0000000000bb"]));
    assert_eq!(subs[1].json()["from"], "in_progress");
}

#[test]
fn retry_refuses_changed_files_other_owners_and_old_operations() {
    let server = MockServer::start(|r| match (r.method.as_str(), r.url.as_str()) {
        ("GET", "/api/v1/me") => me(if r.header("Authorization").unwrap().contains(&token('O')) {
            "other.example"
        } else {
            "dev.example"
        }),
        ("POST", "/api/v1/items/WEB-12/attachments") => Reply::json(200, "{}").delayed(5000),
        _ => Reply::problem(404, "NOT_FOUND"),
    });
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.login(&server.origin, &token('U')).code, 0);
    write_file(&sandbox, "shot.png", b"original");
    let mut child = spawn(&sandbox, &["attachments", "add", "WEB-12", "shot.png"]);
    wait_for(&server, "POST", "/api/v1/items/WEB-12/attachments", 1);
    child.kill().unwrap();
    child.wait().unwrap();
    let op = sandbox.run(&["pending"]).json()["data"][0]["id"].as_str().unwrap().to_owned();

    write_file(&sandbox, "shot.png", b"changed!");
    let changed = sandbox.run(&["retry", &op]);
    assert_eq!((changed.code, changed.error_code().as_str()), (6, "UPLOAD_CHANGED"));
    write_file(&sandbox, "shot.png", b"original");

    assert_eq!(sandbox.login(&server.origin, &token('O')).code, 0);
    let other = sandbox.run(&["retry", &op]);
    assert_eq!((other.code, other.error_code().as_str()), (4, "OPERATION_OWNER_MISMATCH"));
    assert_eq!(sandbox.login(&server.origin, &token('U')).code, 0);

    let path = sandbox.home().join(format!(".local/state/taskhub/operations/{op}.json"));
    let mut entry: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    entry["createdAt"] = json!("2026-01-01T00:00:00Z");
    std::fs::write(&path, entry.to_string()).unwrap();
    let old = sandbox.run(&["retry", &op]);
    assert_eq!((old.code, old.error_code().as_str()), (6, "REPLAY_WINDOW_EXPIRED"));
    assert!(old.json()["error"]["hint"].as_str().unwrap().contains("taskhub show WEB-12"));

    let discard = sandbox.run(&["pending", "discard", &op]);
    assert_eq!(discard.code, 0);
    let gone = sandbox.run(&["retry", &op]);
    assert_eq!((gone.code, gone.error_code().as_str()), (5, "OPERATION_NOT_FOUND"));
    assert_eq!(server.count("POST", "/api/v1/items/WEB-12/attachments"), 1, "no refused retry sent anything");
}

#[test]
fn next_claim_moves_on_when_another_agent_wins() {
    let nexts = Arc::new(AtomicUsize::new(0));
    let n = Arc::clone(&nexts);
    let server = MockServer::start(move |r| {
        let path = r.url.split('?').next().unwrap();
        match (r.method.as_str(), path) {
            ("GET", "/api/v1/me") => me("dev.example"),
            ("GET", "/api/v1/items/next") => {
                let mut value: Value = serde_json::from_str(&fixture("success/getNextItem.json")).unwrap();
                let first = n.fetch_add(1, Ordering::SeqCst) == 0;
                value["data"]["key"] = json!(if first { "WEB-12" } else { "WEB-13" });
                value["data"]["number"] = json!(if first { 12 } else { 13 });
                value["data"]["reason"] = json!("unassigned");
                Reply::json(200, value.to_string())
            }
            ("POST", "/api/v1/items/WEB-12/claim") => Reply::problem(409, "ALREADY_CLAIMED"),
            ("POST", "/api/v1/items/WEB-13/claim") => {
                Reply::json(200, receipt("WEB-13", 2, "in_progress", json!({}), &key_of(r), false))
            }
            _ => Reply::problem(404, "NOT_FOUND"),
        }
    });
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.login(&server.origin, &token('N')).code, 0);
    let run = sandbox.run(&["next", "--claim"]);
    assert_eq!(run.code, 0, "{}", run.stdout);
    let out = run.json();
    assert_eq!(out["data"]["key"], "WEB-13");
    assert_eq!(out["data"]["status"], "in_progress");
    assert_eq!(out["meta"]["claimed"], true);
    assert_eq!(out["meta"]["lostRaces"], json!(["WEB-12"]));
}

#[test]
fn next_returns_sent_back_work_without_claiming_and_reports_an_empty_queue() {
    let server = MockServer::start(|r| match r.url.split('?').next().unwrap() {
        "/api/v1/me" => me("dev.example"),
        "/api/v1/items/next" if r.url.contains("project=OPS") => {
            Reply::json(200, fixture("success/getNextItem-empty.json"))
        }
        "/api/v1/items/next" => {
            let mut value: Value = serde_json::from_str(&fixture("success/getNextItem.json")).unwrap();
            value["data"]["reason"] = json!("sent_back");
            Reply::json(200, value.to_string())
        }
        _ => Reply::problem(404, "NOT_FOUND"),
    });
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.login(&server.origin, &token('B')).code, 0);
    let run = sandbox.run(&["next", "--claim"]).json();
    assert_eq!(run["data"]["reason"], "sent_back");
    assert_eq!(run["meta"]["claimed"], false);
    assert_eq!(server.count("POST", "/"), 0);
    let empty = sandbox.run(&["next", "--project", "ops"]).json();
    assert_eq!(empty["data"], Value::Null);
}

#[test]
fn done_is_refused_locally_and_server_refusals_are_rejected_outcomes() {
    let server = MockServer::start(|r| match (r.method.as_str(), r.url.as_str()) {
        ("GET", "/api/v1/me") => me("dev.example"),
        ("POST", "/api/v1/items/WEB-12/comments") => Reply::problem(404, "NOT_FOUND"),
        _ => Reply::problem(404, "NOT_FOUND"),
    });
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.login(&server.origin, &token('D')).code, 0);
    for args in [
        vec!["items", "move", "WEB-12", "done", "--from", "dev_done"],
        vec!["items", "move", "WEB-12", "todo", "--from", "done"],
    ] {
        let run = sandbox.run(&args);
        assert_eq!((run.code, run.error_code().as_str()), (4, "SIGN_OFF_REQUIRES_PERSON"));
        assert_eq!(run.json()["error"]["httpStatus"], Value::Null);
    }
    assert_eq!(server.count("POST", "/"), 0, "no request for a Done move");

    // An item deleted to Trash mid-work: NOT_FOUND, and the outcome is known (rejected), not unknown.
    let gone = sandbox.run(&["comment", "WEB-12", "--body", "still there?"]);
    assert_eq!((gone.code, gone.error_code().as_str()), (5, "NOT_FOUND"));
    assert_eq!(gone.json()["meta"]["operation"]["outcome"], "rejected");
    assert!(sandbox.run(&["pending"]).json()["data"].as_array().unwrap().is_empty());
}

#[test]
fn busy_servers_get_one_resend_with_the_same_key_and_garbled_replies_are_unknown() {
    let calls = Arc::new(AtomicUsize::new(0));
    let c = Arc::clone(&calls);
    let server = MockServer::start(move |r| match (r.method.as_str(), r.url.as_str()) {
        ("GET", "/api/v1/me") => me("dev.example"),
        ("POST", "/api/v1/items/WEB-12/links") => {
            if c.fetch_add(1, Ordering::SeqCst) == 0 {
                Reply::problem(503, "TEMPORARILY_UNAVAILABLE")
            } else {
                Reply::json(
                    200,
                    receipt(
                        "WEB-12",
                        8,
                        "in_progress",
                        json!({"linkIds": ["00000000-0000-4000-8000-000000000003"]}),
                        &key_of(r),
                        false,
                    ),
                )
            }
        }
        ("POST", "/api/v1/items/WEB-12/claim") => {
            Reply::json(200, format!("{{\"data\":\"{}\"}}", "x".repeat(2 * 1024 * 1024 + 1)))
        }
        _ => Reply::problem(404, "NOT_FOUND"),
    });
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.login(&server.origin, &token('T')).code, 0);
    let run = sandbox.run(&["link", "WEB-12", "https://code.example/pull/42", "--kind", "pr"]);
    assert_eq!(run.code, 0, "{}", run.stdout);
    let links: Vec<Recorded> = server.requests().into_iter().filter(|r| r.url.ends_with("/links")).collect();
    assert_eq!(links.len(), 2);
    assert_eq!(key_of(&links[0]), key_of(&links[1]));
    assert_eq!(links[1].json(), json!({"kind": "pr", "url": "https://code.example/pull/42"}));

    let unknown = sandbox.run(&["claim", "WEB-12"]);
    assert_eq!((unknown.code, unknown.error_code().as_str()), (8, "OUTCOME_UNKNOWN"));
    let out = unknown.json();
    assert_eq!(out["meta"]["operation"]["outcome"], "unknown");
    let op = out["meta"]["operation"]["id"].as_str().unwrap();
    assert_eq!(out["error"]["hint"], format!("taskhub retry {op}"));
    assert_eq!(server.count("POST", "/api/v1/items/WEB-12/claim"), 2, "one automatic resend, then stop");
    assert_eq!(sandbox.run(&["pending"]).json()["data"][0]["id"], op);
}

#[test]
fn request_ids_are_used_as_keys_and_reuse_reports_the_original_write() {
    let server = MockServer::start(|r| match (r.method.as_str(), r.url.as_str()) {
        ("GET", "/api/v1/me") => me("dev.example"),
        ("POST", "/api/v1/items/WEB-12/claim") => {
            Reply::json(200, receipt("WEB-12", 2, "in_progress", json!({}), &key_of(r), false))
        }
        _ => Reply::problem(404, "NOT_FOUND"),
    });
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.login(&server.origin, &token('Q')).code, 0);
    let id = "8c0c6d87-3e8a-4e24-919f-cc62d4a43e5c";
    let first = sandbox.run(&["claim", "WEB-12", "--request-id", id]).json();
    assert_eq!(first["meta"]["operation"], json!({"id": id, "replayed": false, "outcome": "committed"}));
    let again = sandbox.run(&["claim", "WEB-12", "--request-id", id]).json();
    assert_eq!(again["meta"]["operation"]["replayed"], true);
    let claims: Vec<Recorded> = server.requests().into_iter().filter(|r| r.url.ends_with("/claim")).collect();
    assert_eq!(claims.len(), 1);
    assert_eq!(key_of(&claims[0]), id);
}

#[test]
fn input_rules_are_checked_before_anything_is_sent() {
    let server = MockServer::start(|r| match r.url.as_str() {
        "/api/v1/me" => me("dev.example"),
        _ => Reply::problem(404, "NOT_FOUND"),
    });
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.login(&server.origin, &token('I')).code, 0);
    let both_stdin = sandbox.run_with(
        &["submit", "WEB-12", "--summary-file", "-", "--testing-file", "-"],
        Some("text"),
        &[],
    );
    assert_eq!(both_stdin.error_code(), "INVALID_INPUT");
    let long = "x".repeat(5001);
    assert_eq!(sandbox.run(&["comment", "WEB-12", "--body", &long]).error_code(), "INVALID_INPUT");
    assert_eq!(sandbox.run(&["comment", "WEB-12", "--body", "   "]).error_code(), "INVALID_INPUT");
    write_file(&sandbox, "notes.txt", b"x");
    assert_eq!(
        sandbox.run(&["comment", "WEB-12", "--body", "hi", "--attach", "notes.txt"]).error_code(),
        "UNSUPPORTED_FILE_TYPE"
    );
    assert_eq!(
        sandbox.run(&["items", "update", "WEB-12", "--if-version", "3"]).error_code(),
        "INVALID_INPUT"
    );
    assert_eq!(sandbox.run(&["link", "WEB-12", "ftp://x.example"]).error_code(), "INVALID_INPUT");
    assert_eq!(server.count("POST", "/"), 0);
    assert_eq!(server.count("PATCH", "/"), 0);
}

#[test]
fn update_sends_only_the_given_fields_and_none_clears_the_assignee() {
    let server = MockServer::start(|r| match (r.method.as_str(), r.url.as_str()) {
        ("GET", "/api/v1/me") => me("dev.example"),
        ("PATCH", "/api/v1/items/WEB-12") => Reply::json(200, fixture("success/patchItem.json")),
        _ => Reply::problem(404, "NOT_FOUND"),
    });
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.login(&server.origin, &token('V')).code, 0);
    let run = sandbox.run(&[
        "items",
        "update",
        "WEB-12",
        "--if-version",
        "7",
        "--title",
        "Example edit",
        "--assignee",
        "none",
        "--no-labels",
    ]);
    assert_eq!(run.code, 0, "{}", run.stdout);
    let patch = server.requests().into_iter().find(|r| r.method == "PATCH").unwrap();
    let fixture: Value = serde_json::from_str(&fixture("requests/patchItem.json")).unwrap();
    assert_eq!(patch.json(), fixture["body"]);
}

//! The contract snapshot in api/v1 is TaskHub's: it must be unedited, and every fixture must round-trip
//! through the CLI's hand-written types without losing or inventing anything.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use taskhub_cli::error::Code;
use taskhub_cli::types::*;

fn contract_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("api/v1")
}

fn read_json(relative: &str) -> Value {
    let path = contract_dir().join("fixtures").join(relative);
    serde_json::from_str(&fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn files_under(dir: &Path, base: &Path, out: &mut Vec<String>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files_under(&path, base, out);
        } else {
            out.push(path.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/"));
        }
    }
}

#[test]
fn snapshot_matches_its_recorded_hashes() {
    let dir = contract_dir();
    let source: Value =
        serde_json::from_str(&fs::read_to_string(dir.join("contract-source.json")).unwrap()).unwrap();
    let recorded: BTreeMap<String, String> = serde_json::from_value(source["files"].clone()).unwrap();
    let mut present = Vec::new();
    files_under(&dir, &dir, &mut present);
    present.retain(|file| file != "contract-source.json");
    present.sort();
    assert_eq!(
        present,
        recorded.keys().cloned().collect::<Vec<_>>(),
        "api/v1 has files that were added or removed by hand; run scripts/sync-contract.sh"
    );
    for (file, hash) in &recorded {
        let actual = taskhub_cli::digest::sha256_hex(&fs::read(dir.join(file)).unwrap());
        assert_eq!(&actual, hash, "{file} was edited by hand; run scripts/sync-contract.sh");
    }
}

/// Deserializes into `T`, serializes back, and requires the same JSON: every field is modelled, none invented.
fn round_trip<T: DeserializeOwned + Serialize>(name: &str, value: &Value) {
    let typed: T = serde_json::from_value(value.clone()).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(&serde_json::to_value(&typed).unwrap(), value, "{name} does not round-trip");
}

fn index(section: &str) -> Vec<Value> {
    read_json("index.json")[section].as_array().unwrap().clone()
}

#[test]
fn every_success_fixture_round_trips() {
    for entry in index("success") {
        let endpoint = entry["endpoint"].as_str().unwrap();
        let file = entry["file"].as_str().unwrap();
        if !file.ends_with(".json") || endpoint == "getOpenApi" {
            continue; // Raw responses with no envelope: the download bytes and the OpenAPI document itself.
        }
        let value = read_json(file);
        match endpoint {
            "getMe" => round_trip::<Envelope<Me>>(file, &value),
            "listProjects" => round_trip::<Envelope<Vec<Project>>>(file, &value),
            "getProject" => round_trip::<Envelope<ProjectDetail>>(file, &value),
            "listItems" => round_trip::<Envelope<Vec<ItemSummary>>>(file, &value),
            "getNextItem" => round_trip::<Envelope<Option<NextItem>>>(file, &value),
            "getItem" => round_trip::<Envelope<Item>>(file, &value),
            "getContext" => round_trip::<Envelope<Context>>(file, &value),
            "listActivity" => round_trip::<Envelope<Vec<Activity>>>(file, &value),
            "listComments" => round_trip::<Envelope<Vec<Comment>>>(file, &value),
            "listAttachments" => round_trip::<Envelope<Vec<Attachment>>>(file, &value),
            "listNotifications" => round_trip::<Envelope<Vec<Notification>>>(file, &value),
            "markNotificationsRead" => round_trip::<Envelope<MarkedRead>>(file, &value),
            "createItem" | "patchItem" | "transitionItem" | "claimItem" | "submitItem" | "rejectItem"
            | "addComment" | "patchComment" | "uploadAttachment" | "addLink" => {
                round_trip::<Envelope<Receipt>>(file, &value)
            }
            other => panic!("No Rust type for success fixture {other}; add one"),
        }
    }
}

#[test]
fn every_request_body_round_trips() {
    for entry in index("requests") {
        let endpoint = entry["endpoint"].as_str().unwrap();
        let file = entry["file"].as_str().unwrap();
        let fixture = read_json(file);
        let Some(body) = fixture.get("body") else { continue };
        match endpoint {
            "createItem" => round_trip::<CreateItemBody>(file, body),
            "patchItem" => round_trip::<PatchItemBody>(file, body),
            "transitionItem" => round_trip::<TransitionBody>(file, body),
            "claimItem" => round_trip::<ClaimBody>(file, body),
            "submitItem" => round_trip::<SubmissionBody>(file, body),
            "rejectItem" => round_trip::<RejectionBody>(file, body),
            "addComment" => round_trip::<AddCommentBody>(file, body),
            "patchComment" => round_trip::<PatchCommentBody>(file, body),
            "addLink" => round_trip::<AddLinkBody>(file, body),
            "markNotificationsRead" => round_trip::<MarkReadBody>(file, body),
            "uploadAttachment" => {
                assert!(body["file"]["filename"].is_string(), "upload sends one multipart `file`")
            }
            other => panic!("No Rust request type for {other}; add one"),
        }
        if fixture.get("headers").is_some() {
            let key = fixture["headers"]["Idempotency-Key"].as_str().unwrap();
            assert!(uuid::Uuid::parse_str(key).is_ok(), "{file}: writes carry a UUID Idempotency-Key");
        }
    }
}

#[test]
fn every_error_fixture_is_a_known_code() {
    for entry in index("errors") {
        let file = entry["file"].as_str().unwrap();
        let value = read_json(file);
        round_trip::<Problem>(file, &value);
        let code = Code::parse(value["code"].as_str().unwrap());
        assert!(!matches!(code, Code::Other(_)), "{file}: the CLI has no exit code or hint for this code");
        assert_eq!(value["code"], entry["code"]);
    }
}

#[test]
fn request_types_reject_unknown_fields() {
    let bad = serde_json::json!({"body": "hi", "extra": true});
    assert!(serde_json::from_value::<AddCommentBody>(bad).is_err());
    let bad = serde_json::json!({"ids": ["x"], "all": true});
    assert!(serde_json::from_value::<MarkReadBody>(bad).is_err());
}

#[test]
fn openapi_snapshot_lists_the_endpoints_the_cli_uses() {
    let doc: Value =
        serde_json::from_str(&fs::read_to_string(contract_dir().join("openapi.json")).unwrap()).unwrap();
    assert_eq!(doc["openapi"], "3.1.0");
    let operations: Vec<&str> = doc["paths"]
        .as_object()
        .unwrap()
        .values()
        .flat_map(|ops| ops.as_object().unwrap().values().filter_map(|op| op["operationId"].as_str()))
        .collect();
    for needed in [
        "getMe",
        "listProjects",
        "getProject",
        "listItems",
        "getNextItem",
        "getItem",
        "getContext",
        "listActivity",
        "listComments",
        "listAttachments",
        "downloadAttachment",
        "listNotifications",
        "createItem",
        "patchItem",
        "transitionItem",
        "claimItem",
        "submitItem",
        "rejectItem",
        "addComment",
        "patchComment",
        "uploadAttachment",
        "addLink",
        "markNotificationsRead",
        "deleteItem",
    ] {
        assert!(operations.contains(&needed), "openapi.json has no {needed}");
    }
}

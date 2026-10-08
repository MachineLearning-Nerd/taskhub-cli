//! Hand-written API v1 types, checked against TaskHub's fixtures in tests/contract.rs.
//!
//! Responses tolerate added fields (serde ignores them) and unknown enum values (kept in `Unknown`).
//! Requests reject unknown fields, and only ever carry enum values parsed from known names.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// A string enum that keeps values this CLI does not know, so a newer server never breaks a read.
/// `parse_known` is the only way user input becomes a value, so `Unknown` is never sent in a write.
macro_rules! string_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident = $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        pub enum $name {
            $($variant,)+
            Unknown(String),
        }

        impl $name {
            pub const KNOWN: &'static [&'static str] = &[$($text),+];

            pub fn as_str(&self) -> &str {
                match self {
                    $($name::$variant => $text,)+
                    $name::Unknown(value) => value,
                }
            }

            pub fn parse_known(value: &str) -> Option<Self> {
                match value {
                    $($text => Some($name::$variant),)+
                    _ => None,
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = String::deserialize(deserializer)?;
                Ok($name::parse_known(&value).unwrap_or($name::Unknown(value)))
            }
        }
    };
}

string_enum!(Status { Todo = "todo", InProgress = "in_progress", DevDone = "dev_done", Done = "done" });
string_enum!(Priority { High = "high", Medium = "medium", Low = "low" });
string_enum!(ItemType { Task = "task", Bug = "bug" });
string_enum!(LinkKind { Pr = "pr", Link = "link" });
string_enum!(Role { Admin = "admin", Developer = "developer", Tester = "tester" });
string_enum!(Profile { Read = "read", Write = "write" });
string_enum!(NextReason { SentBack = "sent_back", Assigned = "assigned", Unassigned = "unassigned" });
string_enum!(NotificationKind { Mention = "mention", Assigned = "assigned", Rejected = "rejected", Review = "review" });

/// A person as the API shows them. `agent` is true when a token did the work: shown as "username (agent)".
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Person {
    pub username: String,
    pub agent: bool,
}

impl fmt::Display for Person {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.agent { write!(f, "{} (agent)", self.username) } else { f.write_str(&self.username) }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemSummary {
    pub key: String,
    pub project: String,
    pub number: u64,
    #[serde(rename = "type")]
    pub item_type: ItemType,
    pub title: String,
    pub status: Status,
    pub priority: Priority,
    pub assignee: Option<Person>,
    pub labels: Vec<String>,
    pub version: u64,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    #[serde(flatten)]
    pub summary: ItemSummary,
    pub description: String,
    pub created_by: Person,
    pub created_at: String,
    pub links: Vec<Link>,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextItem {
    #[serde(flatten)]
    pub summary: ItemSummary,
    pub reason: NextReason,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub id: String,
    pub kind: LinkKind,
    pub url: String,
    pub author: Person,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub size: u64,
    pub uploader: Person,
    pub comment_id: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: String,
    pub author: Person,
    pub body: String,
    pub created_at: String,
    pub edited_at: Option<String>,
    pub attachments: Vec<Attachment>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub at: String,
    pub actor: Person,
    /// Kept as text: new event names appear as TaskHub grows and are shown as they come.
    pub event: String,
    pub fields: Vec<String>,
    pub from: Option<Status>,
    pub to: Option<Status>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationItem {
    pub key: String,
    pub title: String,
    pub status: Status,
    pub assignee: Option<Person>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub id: String,
    pub kind: NotificationKind,
    pub item: NotificationItem,
    pub actor: Person,
    pub created_at: String,
    pub read: bool,
}

/// Keyed by status name, as the API sends it (`in_progress`, not camelCase).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemCounts {
    pub todo: u64,
    pub in_progress: u64,
    pub dev_done: u64,
    pub done: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub key: String,
    pub name: String,
    pub description: String,
    pub archived: bool,
    pub item_counts: ItemCounts,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Label {
    pub name: String,
    pub color: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Member {
    pub username: String,
    pub role: Role,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Enums {
    pub types: Vec<ItemType>,
    pub statuses: Vec<Status>,
    pub priorities: Vec<Priority>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    pub title: usize,
    pub description: usize,
    pub comment: usize,
    pub link_url: usize,
    pub attachment_bytes: u64,
    pub json_body_bytes: u64,
    pub multipart_body_bytes: u64,
    pub response_bytes: u64,
    pub receipt_bytes: u64,
    pub default_page_size: u32,
    pub max_page_size: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDetail {
    #[serde(flatten)]
    pub project: Project,
    pub labels: Vec<Label>,
    pub members: Vec<Member>,
    pub enums: Enums,
    pub limits: Limits,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Me {
    pub owner: Person,
    pub role: Role,
    pub profile: Profile,
    pub projects: Vec<String>,
    pub expires_at: Option<String>,
    pub api_version: u32,
    pub min_client_version: Option<String>,
    pub latest_client_version: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paged<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContextProject {
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    pub item: Item,
    pub project: ContextProject,
    pub allowed_transitions: Vec<Status>,
    pub comments: Paged<Comment>,
    pub attachments: Paged<Attachment>,
    pub links: Paged<Link>,
}

/// What every item write returns. Optional parts appear only for the writes that produce them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    pub key: String,
    pub version: u64,
    pub status: Status,
    /// For edits: the fields that actually changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_fields: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mentioned: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unmatched_mentions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_ids: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkedRead {
    pub marked_read: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServerOperation {
    pub id: String,
    pub replayed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub api_version: u32,
    pub request_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<Page>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<ServerOperation>,
}

/// The success envelope: `data` is `null` for `/items/next` when there is nothing to do.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub data: T,
    pub meta: Meta,
}

/// RFC 9457 Problem Details as TaskHub sends them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    #[serde(rename = "type")]
    pub problem_type: String,
    pub title: String,
    pub status: u16,
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

// ---- Requests -----------------------------------------------------------------------------------------

/// A field in a partial update: left out, cleared with `null`, or set.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Patch<T> {
    #[default]
    Missing,
    Null,
    Value(T),
}

impl<T> Patch<T> {
    pub fn is_missing(&self) -> bool {
        matches!(self, Patch::Missing)
    }
}

impl<T: Serialize> Serialize for Patch<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            // Never reached: fields are skipped with `skip_serializing_if = "Patch::is_missing"`.
            Patch::Missing | Patch::Null => serializer.serialize_none(),
            Patch::Value(value) => value.serialize(serializer),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Patch<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Only called when the field is present; `#[serde(default)]` supplies `Missing`.
        Ok(match Option::<T>::deserialize(deserializer)? {
            Some(value) => Patch::Value(value),
            None => Patch::Null,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateItemBody {
    #[serde(rename = "type")]
    pub item_type: ItemType,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<Priority>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemChanges {
    #[serde(rename = "type", default, skip_serializing_if = "Patch::is_missing")]
    pub item_type: Patch<ItemType>,
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub title: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub description: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub priority: Patch<Priority>,
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub assignee: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub labels: Patch<Vec<String>>,
}

impl ItemChanges {
    pub fn is_empty(&self) -> bool {
        self.item_type.is_missing()
            && self.title.is_missing()
            && self.description.is_missing()
            && self.priority.is_missing()
            && self.assignee.is_missing()
            && self.labels.is_missing()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchItemBody {
    pub expected_version: u64,
    pub changes: ItemChanges,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransitionBody {
    pub from: Status,
    pub to: Status,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimBody {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddLinkBody {
    pub kind: LinkKind,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubmissionBody {
    pub from: Status,
    pub summary: String,
    pub testing: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limitations: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<AddLinkBody>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment_ids: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RejectionBody {
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment_ids: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddCommentBody {
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment_ids: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatchCommentBody {
    pub body: String,
}

/// Either specific entries or everything: `{"ids": [...]}` or `{"all": true}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum MarkReadBody {
    Ids { ids: Vec<String> },
    All { all: AlwaysTrue },
}

/// Serializes as `true` and accepts only `true`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AlwaysTrue;

impl Serialize for AlwaysTrue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(true)
    }
}

impl<'de> Deserialize<'de> for AlwaysTrue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if bool::deserialize(deserializer)? {
            Ok(AlwaysTrue)
        } else {
            Err(serde::de::Error::custom("expected true"))
        }
    }
}

/// TaskHub counts field limits in UTF-16 code units, as JavaScript's `String.length` does.
pub fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unknown_enum_values_survive_reads() {
        let status: Status = serde_json::from_value(json!("blocked")).unwrap();
        assert_eq!(status, Status::Unknown("blocked".into()));
        assert_eq!(status.as_str(), "blocked");
        assert_eq!(Status::parse_known("blocked"), None);
        assert_eq!(Status::parse_known("dev_done"), Some(Status::DevDone));
    }

    #[test]
    fn patch_fields_distinguish_missing_null_and_value() {
        let changes =
            ItemChanges { assignee: Patch::Null, title: Patch::Value("New".into()), ..Default::default() };
        assert_eq!(serde_json::to_value(&changes).unwrap(), json!({"assignee": null, "title": "New"}));
        let back: ItemChanges = serde_json::from_value(json!({"assignee": null, "title": "New"})).unwrap();
        assert_eq!(back, changes);
        assert!(back.priority.is_missing());
        assert!(serde_json::from_value::<ItemChanges>(json!({"color": "red"})).is_err());
    }

    #[test]
    fn utf16_length_matches_javascript() {
        assert_eq!(utf16_len("abc"), 3);
        assert_eq!(utf16_len("é"), 1);
        assert_eq!(utf16_len("😀"), 2);
        assert_eq!(utf16_len("👩‍💻"), 5);
    }

    #[test]
    fn people_marked_as_agents_read_as_username_agent() {
        assert_eq!(Person { username: "ravi".into(), agent: true }.to_string(), "ravi (agent)");
        assert_eq!(Person { username: "neha".into(), agent: false }.to_string(), "neha");
    }
}

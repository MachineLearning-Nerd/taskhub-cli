//! The one error type. Every failure, local or from the server, becomes a `CliError` with a stable code,
//! an exit code and a hint, and is rendered through the same envelope.

use serde::Serialize;
use serde_json::{Map, Value};
use std::fmt;

macro_rules! codes {
    ($($variant:ident = $name:literal => $exit:expr),+ $(,)?) => {
        /// Stable error codes: the API's own plus the CLI's local ones. Agents match on these, never on messages.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum Code {
            $($variant,)+
            /// A code this CLI version does not know yet. Shown as received.
            Other(String),
        }

        impl Code {
            pub fn as_str(&self) -> &str {
                match self {
                    $(Code::$variant => $name,)+
                    Code::Other(code) => code,
                }
            }

            pub fn parse(code: &str) -> Code {
                match code {
                    $($name => Code::$variant,)+
                    other => Code::Other(other.to_owned()),
                }
            }

            /// The process exit code, as defined in PLAN.md "Exit codes".
            pub fn exit_code(&self) -> u8 {
                match self {
                    $(Code::$variant => $exit,)+
                    Code::Other(_) => 1,
                }
            }
        }
    };
}

codes! {
    // Server codes.
    BadRequest = "BAD_REQUEST" => 2,
    InvalidCursor = "INVALID_CURSOR" => 2,
    PayloadTooLarge = "PAYLOAD_TOO_LARGE" => 2,
    UnsupportedFileType = "UNSUPPORTED_FILE_TYPE" => 2,
    ValidationFailed = "VALIDATION_FAILED" => 2,
    UnknownUser = "UNKNOWN_USER" => 2,
    UnknownLabel = "UNKNOWN_LABEL" => 2,
    InvalidTransition = "INVALID_TRANSITION" => 2,
    AttachmentNotAvailable = "ATTACHMENT_NOT_AVAILABLE" => 2,
    ResponseTooLarge = "RESPONSE_TOO_LARGE" => 2,
    IdempotencyKeyRequired = "IDEMPOTENCY_KEY_REQUIRED" => 1,
    Unauthenticated = "UNAUTHENTICATED" => 3,
    ReadOnlyToken = "READ_ONLY_TOKEN" => 4,
    SignOffRequiresPerson = "SIGN_OFF_REQUIRES_PERSON" => 4,
    DeleteRequiresPerson = "DELETE_REQUIRES_PERSON" => 4,
    NotAgentComment = "NOT_AGENT_COMMENT" => 4,
    ProjectArchived = "PROJECT_ARCHIVED" => 4,
    NotFound = "NOT_FOUND" => 5,
    VersionConflict = "VERSION_CONFLICT" => 6,
    StatusConflict = "STATUS_CONFLICT" => 6,
    AlreadyClaimed = "ALREADY_CLAIMED" => 6,
    IdempotencyConflict = "IDEMPOTENCY_CONFLICT" => 6,
    IdempotencyInProgress = "IDEMPOTENCY_IN_PROGRESS" => 6,
    RateLimited = "RATE_LIMITED" => 7,
    TemporarilyUnavailable = "TEMPORARILY_UNAVAILABLE" => 8,
    ClientUpgradeRequired = "CLIENT_UPGRADE_REQUIRED" => 9,
    // CLI-local codes (httpStatus: null).
    InvalidInput = "INVALID_INPUT" => 2,
    RefRequired = "REF_REQUIRED" => 2,
    PrNotFound = "PR_NOT_FOUND" => 2,
    OutputExists = "OUTPUT_EXISTS" => 2,
    OriginNotAllowed = "ORIGIN_NOT_ALLOWED" => 2,
    CredentialsMissing = "CREDENTIALS_MISSING" => 3,
    CredentialsInsecure = "CREDENTIALS_INSECURE" => 3,
    OperationOwnerMismatch = "OPERATION_OWNER_MISMATCH" => 4,
    OperationNotFound = "OPERATION_NOT_FOUND" => 5,
    ReplayWindowExpired = "REPLAY_WINDOW_EXPIRED" => 6,
    UploadChanged = "UPLOAD_CHANGED" => 6,
    OutcomeUnknown = "OUTCOME_UNKNOWN" => 8,
    TransportFailed = "TRANSPORT_FAILED" => 8,
    ProtocolError = "PROTOCOL_ERROR" => 9,
    Internal = "INTERNAL" => 1,
}

impl Serialize for Code {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What a write's journal knows about its outcome, carried on errors from writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Committed,
    Rejected,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
pub struct OperationMeta {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replayed: Option<bool>,
    pub outcome: Outcome,
}

/// An error is boxed so `Result<T, CliError>` stays small; fields are reached through `Deref`.
#[derive(Clone, Debug)]
pub struct CliError(Box<ErrorData>);

#[derive(Clone, Debug)]
pub struct ErrorData {
    pub code: Code,
    pub message: String,
    pub http_status: Option<u16>,
    pub details: Map<String, Value>,
    pub hint: Option<String>,
    pub operation: Option<OperationMeta>,
    /// Seconds the server asked us to wait, from `Retry-After` or `details.retryAfterSeconds`.
    pub retry_after: Option<u64>,
}

impl std::ops::Deref for CliError {
    type Target = ErrorData;
    fn deref(&self) -> &ErrorData {
        &self.0
    }
}

impl std::ops::DerefMut for CliError {
    fn deref_mut(&mut self) -> &mut ErrorData {
        &mut self.0
    }
}

impl CliError {
    pub fn new(code: Code, message: impl Into<String>) -> Self {
        CliError(Box::new(ErrorData {
            code,
            message: message.into(),
            http_status: None,
            details: Map::new(),
            hint: None,
            operation: None,
            retry_after: None,
        }))
    }

    pub fn input(message: impl Into<String>) -> Self {
        CliError::new(Code::InvalidInput, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        CliError::new(Code::Internal, message)
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn with_detail(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.details.insert(key.to_owned(), value.into());
        self
    }

    pub fn with_operation(mut self, operation: OperationMeta) -> Self {
        self.operation = Some(operation);
        self
    }

    pub fn exit_code(&self) -> u8 {
        self.code.exit_code()
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for CliError {}

pub type Result<T> = std::result::Result<T, CliError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip_and_map_to_the_planned_exit_codes() {
        let table = [
            ("VALIDATION_FAILED", 2),
            ("IDEMPOTENCY_KEY_REQUIRED", 1),
            ("UNAUTHENTICATED", 3),
            ("SIGN_OFF_REQUIRES_PERSON", 4),
            ("DELETE_REQUIRES_PERSON", 4),
            ("NOT_FOUND", 5),
            ("IDEMPOTENCY_IN_PROGRESS", 6),
            ("RATE_LIMITED", 7),
            ("TEMPORARILY_UNAVAILABLE", 8),
            ("CLIENT_UPGRADE_REQUIRED", 9),
            ("REF_REQUIRED", 2),
            ("CREDENTIALS_INSECURE", 3),
            ("OPERATION_OWNER_MISMATCH", 4),
            ("OPERATION_NOT_FOUND", 5),
            ("REPLAY_WINDOW_EXPIRED", 6),
            ("OUTCOME_UNKNOWN", 8),
            ("PROTOCOL_ERROR", 9),
            ("INTERNAL", 1),
        ];
        for (name, exit) in table {
            let code = Code::parse(name);
            assert_eq!(code.as_str(), name);
            assert_eq!(code.exit_code(), exit, "{name}");
        }
        assert_eq!(Code::parse("SOMETHING_NEW"), Code::Other("SOMETHING_NEW".into()));
        assert_eq!(Code::parse("SOMETHING_NEW").exit_code(), 1);
    }
}

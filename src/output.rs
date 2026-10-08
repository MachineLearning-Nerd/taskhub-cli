//! All output goes through here: the JSON envelope when piped (or `--json`), readable text on a terminal.

use crate::error::{CliError, OperationMeta};
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::io::{IsTerminal, Write};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Json,
    Human,
}

impl Mode {
    pub fn choose(json: bool, human: bool) -> Mode {
        if json {
            return Mode::Json;
        }
        if human {
            return Mode::Human;
        }
        if std::env::var("TASKHUB_OUTPUT").is_ok_and(|value| value == "json") {
            return Mode::Json;
        }
        if std::io::stdout().is_terminal() { Mode::Human } else { Mode::Json }
    }
}

/// Where an item key came from, reported as `meta.ref`.
#[derive(Clone, Debug, Serialize)]
pub struct RefMeta {
    pub key: String,
    /// `argument` or `branch`.
    pub source: &'static str,
}

/// A command's successful result: the data for the envelope, extra meta, and the text for people.
#[derive(Debug)]
pub struct Success {
    pub data: Value,
    pub meta: Map<String, Value>,
    pub human: String,
}

impl Success {
    pub fn new(data: Value, human: impl Into<String>) -> Self {
        Success { data, meta: Map::new(), human: human.into() }
    }

    /// Copies the server's `meta` (apiVersion, requestId, page) into ours.
    pub fn with_server_meta(mut self, meta: &Value) -> Self {
        if let Value::Object(map) = meta {
            for (key, value) in map {
                if key != "operation" {
                    self.meta.insert(key.clone(), value.clone());
                }
            }
        }
        self
    }

    pub fn with_operation(mut self, operation: &OperationMeta) -> Self {
        self.meta.insert("operation".into(), json!(operation));
        self
    }

    pub fn with_ref(mut self, reference: Option<&RefMeta>) -> Self {
        if let Some(reference) = reference {
            self.meta.insert("ref".into(), json!(reference));
        }
        self
    }
}

pub fn success_envelope(success: &Success) -> Value {
    json!({ "ok": true, "data": success.data, "meta": success.meta })
}

pub fn error_envelope(error: &CliError) -> Value {
    let mut meta = Map::new();
    if let Some(operation) = &error.operation {
        meta.insert("operation".into(), json!(operation));
    }
    json!({
        "ok": false,
        "error": {
            "code": error.code,
            "message": error.message,
            "httpStatus": error.http_status,
            "details": error.details,
            "hint": error.hint,
        },
        "meta": meta,
    })
}

pub fn print_success(mode: Mode, success: &Success) {
    match mode {
        Mode::Json => print_json(&success_envelope(success)),
        Mode::Human => {
            let text = success.human.trim_end();
            if !text.is_empty() {
                print_line(text);
            }
        }
    }
}

pub fn print_error(mode: Mode, error: &CliError) {
    match mode {
        Mode::Json => print_json(&error_envelope(error)),
        Mode::Human => {
            let mut text = format!("Error: {} ({})", error.message, error.code);
            if let Some(hint) = &error.hint {
                text.push_str(&format!("\nNext: {hint}"));
            }
            if let Some(operation) = &error.operation {
                text.push_str(&format!("\nOperation: {}", operation.id));
            }
            let _ = writeln!(std::io::stderr(), "{text}");
        }
    }
}

fn print_json(value: &Value) {
    print_line(&value.to_string());
}

fn print_line(text: &str) {
    let mut stdout = std::io::stdout().lock();
    // A closed pipe (`taskhub ... | head`) is not an error worth reporting.
    let _ = writeln!(stdout, "{text}");
    let _ = stdout.flush();
}

/// A note for people on stderr, such as "Using WEB-12 from branch ...". Never part of the envelope.
pub fn note(text: &str) {
    let _ = writeln!(std::io::stderr(), "{text}");
}

/// A plain text table: columns padded to the widest cell, no colors or box drawing.
pub fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut widths: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if let Some(width) = widths.get_mut(i) {
                *width = (*width).max(cell.chars().count());
            }
        }
    }
    let line = |cells: &[String]| {
        let mut out = String::new();
        for (i, cell) in cells.iter().enumerate() {
            if i + 1 == cells.len() {
                out.push_str(cell);
            } else {
                let pad = widths[i].saturating_sub(cell.chars().count());
                out.push_str(cell);
                out.push_str(&" ".repeat(pad + 2));
            }
        }
        out
    };
    let mut out = line(&headers.iter().map(|h| h.to_string()).collect::<Vec<_>>());
    for row in rows {
        out.push('\n');
        out.push_str(&line(row));
    }
    out
}

/// Shortens text to `max` characters with an ellipsis, for table cells.
pub fn clip(text: &str, max: usize) -> String {
    let single_line = text.replace(['\n', '\r', '\t'], " ");
    if single_line.chars().count() <= max {
        return single_line;
    }
    let mut out: String = single_line.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{CliError, Code};

    #[test]
    fn error_envelope_has_the_planned_shape() {
        let error = CliError::new(Code::RefRequired, "No item key given.").with_hint("Pass a key.");
        assert_eq!(
            error_envelope(&error),
            json!({"ok": false, "error": {"code": "REF_REQUIRED", "message": "No item key given.",
                "httpStatus": null, "details": {}, "hint": "Pass a key."}, "meta": {}})
        );
    }

    #[test]
    fn tables_pad_columns() {
        let text = table(
            &["KEY", "TITLE"],
            &[vec!["WEB-1".into(), "One".into()], vec!["WEB-12".into(), "Two".into()]],
        );
        assert_eq!(text, "KEY     TITLE\nWEB-1   One\nWEB-12  Two");
    }

    #[test]
    fn clip_shortens_long_text() {
        assert_eq!(clip("short", 10), "short");
        assert_eq!(clip("a much longer title", 8), "a much …");
    }
}

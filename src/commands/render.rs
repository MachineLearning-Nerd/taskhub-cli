//! Text for people. Every function takes typed API data and returns a string; JSON mode never uses these.
//! Item titles, descriptions and comments are other people's text: shown as-is, never interpreted.

use crate::clock::relative;
use crate::output::{clip, table};
use crate::types::*;

pub fn status_label(status: &Status) -> &str {
    match status {
        Status::Todo => "To Do",
        Status::InProgress => "In Progress",
        Status::DevDone => "Dev Done",
        Status::Done => "Done",
        Status::Unknown(value) => value,
    }
}

fn person(person: Option<&Person>) -> String {
    person.map_or_else(|| "unassigned".to_owned(), Person::to_string)
}

pub fn item_rows(items: &[ItemSummary]) -> String {
    if items.is_empty() {
        return "No items.".to_owned();
    }
    let rows: Vec<Vec<String>> = items
        .iter()
        .map(|item| {
            vec![
                item.key.clone(),
                status_label(&item.status).to_owned(),
                item.priority.to_string(),
                item.item_type.to_string(),
                person(item.assignee.as_ref()),
                clip(&item.title, 60),
            ]
        })
        .collect();
    table(&["KEY", "STATUS", "PRIORITY", "TYPE", "ASSIGNEE", "TITLE"], &rows)
}

pub fn more(next_cursor: Option<&str>, command: &str) -> String {
    match next_cursor {
        Some(cursor) => format!("\nMore: {command} --cursor {cursor}"),
        None => String::new(),
    }
}

fn header(item: &ItemSummary) -> String {
    let labels =
        if item.labels.is_empty() { String::new() } else { format!(" · labels {}", item.labels.join(", ")) };
    format!(
        "{}  {}\n{} · {} priority · {} · {}{labels} · version {}",
        item.key,
        item.title,
        item.item_type,
        item.priority,
        status_label(&item.status),
        person(item.assignee.as_ref()),
        item.version
    )
}

pub fn item(item: &Item) -> String {
    let mut out = header(&item.summary);
    out.push_str(&format!("\nCreated by {} {} · {}", item.created_by, relative(&item.created_at), item.url));
    if !item.description.trim().is_empty() {
        out.push_str(&format!("\n\n{}", item.description.trim_end()));
    }
    if !item.links.is_empty() {
        out.push_str(&format!("\n\nLinks\n{}", links(&item.links)));
    }
    out
}

fn links(links: &[Link]) -> String {
    links
        .iter()
        .map(|link| format!("  {:<4} {}  ({})", link.kind.as_str().to_uppercase(), link.url, link.author))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn attachments(files: &[Attachment]) -> String {
    if files.is_empty() {
        return "No files.".to_owned();
    }
    let rows: Vec<Vec<String>> = files
        .iter()
        .map(|file| {
            vec![
                file.id.clone(),
                clip(&file.filename, 40),
                size(file.size),
                file.uploader.to_string(),
                relative(&file.created_at),
            ]
        })
        .collect();
    table(&["ID", "FILE", "SIZE", "BY", "ADDED"], &rows)
}

pub fn size(bytes: u64) -> String {
    match bytes {
        0..1024 => format!("{bytes} B"),
        1024..1_048_576 => format!("{:.1} KB", bytes as f64 / 1024.0),
        _ => format!("{:.1} MB", bytes as f64 / 1_048_576.0),
    }
}

pub fn comments(list: &[Comment]) -> String {
    if list.is_empty() {
        return "No comments.".to_owned();
    }
    list.iter()
        .map(|comment| {
            let edited = if comment.edited_at.is_some() { " (edited)" } else { "" };
            let files = if comment.attachments.is_empty() {
                String::new()
            } else {
                let names: Vec<&str> = comment.attachments.iter().map(|f| f.filename.as_str()).collect();
                format!("\n    Files: {}", names.join(", "))
            };
            let body = comment
                .body
                .trim_end()
                .lines()
                .map(|line| format!("    {line}"))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "  {}, {}{edited} · {}\n{body}{files}",
                comment.author,
                relative(&comment.created_at),
                comment.id
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn context(context: &Context) -> String {
    let mut out = item(&context.item);
    out.push_str(&format!("\n\nProject: {}", context.project.name));
    let moves: Vec<&str> = context.allowed_transitions.iter().map(status_label).collect();
    out.push_str(&format!(
        "\nAllowed moves: {}",
        if moves.is_empty() { "none".to_owned() } else { moves.join(", ") }
    ));
    if !context.attachments.data.is_empty() {
        out.push_str(&format!("\n\nFiles\n{}", attachments(&context.attachments.data)));
    }
    out.push_str(&format!("\n\nLatest comments\n{}", comments(&context.comments.data)));
    out.push_str(&more(
        context.comments.next_cursor.as_deref(),
        &format!("taskhub comments list {}", context.item.summary.key),
    ));
    out
}

pub fn activity(events: &[Activity]) -> String {
    if events.is_empty() {
        return "No activity.".to_owned();
    }
    events
        .iter()
        .map(|event| {
            let movement = match (&event.from, &event.to) {
                (Some(from), Some(to)) => format!(" {} → {}", status_label(from), status_label(to)),
                _ => String::new(),
            };
            let fields = if event.fields.is_empty() {
                String::new()
            } else {
                format!(" ({})", event.fields.join(", "))
            };
            format!("{:<16} {} {}{movement}{fields}", relative(&event.at), event.actor, event.event)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn notifications(list: &[Notification]) -> String {
    if list.is_empty() {
        return "Inbox is empty.".to_owned();
    }
    let rows: Vec<Vec<String>> = list
        .iter()
        .map(|entry| {
            vec![
                if entry.read { " ".into() } else { "•".into() },
                entry.kind.to_string(),
                entry.item.key.clone(),
                status_label(&entry.item.status).to_owned(),
                entry.actor.to_string(),
                relative(&entry.created_at),
                clip(&entry.item.title, 40),
                entry.id.clone(),
            ]
        })
        .collect();
    table(&["", "KIND", "ITEM", "STATUS", "BY", "WHEN", "TITLE", "ID"], &rows)
}

pub fn projects(list: &[Project]) -> String {
    if list.is_empty() {
        return "No projects are granted to this token.".to_owned();
    }
    let rows: Vec<Vec<String>> = list
        .iter()
        .map(|project| {
            let c = &project.item_counts;
            vec![
                project.key.clone(),
                clip(&project.name, 30),
                format!("{} / {} / {} / {}", c.todo, c.in_progress, c.dev_done, c.done),
                if project.archived { "archived".into() } else { String::new() },
            ]
        })
        .collect();
    table(&["KEY", "NAME", "TODO / IN PROGRESS / DEV DONE / DONE", ""], &rows)
}

pub fn project(detail: &ProjectDetail) -> String {
    let p = &detail.project;
    let mut out = format!("{}  {}{}", p.key, p.name, if p.archived { " (archived)" } else { "" });
    if !p.description.trim().is_empty() {
        out.push_str(&format!("\n{}", p.description.trim_end()));
    }
    let labels: Vec<&str> = detail.labels.iter().map(|label| label.name.as_str()).collect();
    let members: Vec<String> =
        detail.members.iter().map(|m| format!("{} ({})", m.username, m.role)).collect();
    out.push_str(&format!("\nLabels: {}", if labels.is_empty() { "none".into() } else { labels.join(", ") }));
    out.push_str(&format!("\nMembers: {}", members.join(", ")));
    let l = &detail.limits;
    out.push_str(&format!(
        "\nLimits: title {} · description {} · comment {} characters · files {}",
        l.title,
        l.description,
        l.comment,
        size(l.attachment_bytes)
    ));
    out
}

#[allow(dead_code)] // Used by the write commands (C2).
pub fn receipt(action: &str, receipt: &Receipt) -> String {
    let mut out = format!(
        "{action} {} (now {}, version {}).",
        receipt.key,
        status_label(&receipt.status),
        receipt.version
    );
    if let Some(mentioned) = receipt.mentioned.as_ref().filter(|list| !list.is_empty()) {
        out.push_str(&format!("\nNotified: {}", mentioned.join(", ")));
    }
    if let Some(unmatched) = receipt.unmatched_mentions.as_ref().filter(|list| !list.is_empty()) {
        out.push_str(&format!("\nNo such user, nobody notified: {}", unmatched.join(", ")));
    }
    out
}

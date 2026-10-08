//! Time helpers: RFC 3339 timestamps and "3 hours ago" for people.

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

pub fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

pub fn now_rfc3339() -> String {
    now().format(&Rfc3339).unwrap_or_default()
}

pub fn parse(text: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(text, &Rfc3339).ok()
}

/// "just now", "5 minutes ago", "3 days ago", or "in 4 days" for future times.
pub fn relative(text: &str) -> String {
    let Some(at) = parse(text) else { return text.to_owned() };
    relative_to(at, now())
}

pub fn relative_to(at: OffsetDateTime, now: OffsetDateTime) -> String {
    let seconds = (now - at).whole_seconds();
    let (amount, future) = (seconds.unsigned_abs(), seconds < 0);
    let (value, unit) = match amount {
        0..60 => return "just now".to_owned(),
        60..3600 => (amount / 60, "minute"),
        3600..86_400 => (amount / 3600, "hour"),
        86_400..2_592_000 => (amount / 86_400, "day"),
        2_592_000..31_536_000 => (amount / 2_592_000, "month"),
        _ => (amount / 31_536_000, "year"),
    };
    let plural = if value == 1 { "" } else { "s" };
    if future { format!("in {value} {unit}{plural}") } else { format!("{value} {unit}{plural} ago") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    #[test]
    fn relative_times_read_naturally() {
        let now = parse("2026-10-08T12:00:00Z").unwrap();
        assert_eq!(relative_to(now - Duration::seconds(30), now), "just now");
        assert_eq!(relative_to(now - Duration::minutes(1), now), "1 minute ago");
        assert_eq!(relative_to(now - Duration::hours(5), now), "5 hours ago");
        assert_eq!(relative_to(now - Duration::days(3), now), "3 days ago");
        assert_eq!(relative_to(now + Duration::days(4), now), "in 4 days");
    }
}

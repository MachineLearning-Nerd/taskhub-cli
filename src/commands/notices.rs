//! At most once a day, on a terminal: warn when the token expires within seven days, and mention a newer CLI.
//! Failures here are silent; they must never affect the command that just succeeded.

use super::Session;
use crate::config;
use crate::output::note;
use std::time::{Duration, SystemTime};

const STAMP: &str = "daily-check";
const INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

pub fn daily() {
    let Ok(dir) = config::state_dir() else { return };
    let stamp = dir.join(STAMP);
    let fresh = std::fs::metadata(&stamp)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|at| SystemTime::now().duration_since(at).ok())
        .is_some_and(|age| age < INTERVAL);
    if fresh || config::write_private(&stamp, b"").is_err() {
        return;
    }
    let Ok(session) = Session::open(Duration::from_secs(5)) else { return };
    let Ok(me) = super::auth::fetch_me(&session.client) else { return };
    if let Some(expires) = me.expires_at.as_deref().and_then(crate::clock::parse) {
        let left = expires - crate::clock::now();
        if left.is_positive() && left.whole_days() < 7 {
            note(&format!(
                "Your TaskHub token expires {}. Create a new one at {}",
                crate::clock::relative_to(expires, crate::clock::now()),
                session.credential.origin.url("/settings/tokens")
            ));
        }
    }
    if let Some(latest) = me.latest_client_version.as_deref() {
        if newer(latest, env!("CARGO_PKG_VERSION")) {
            note(&format!("taskhub {latest} is available (this is {}).", env!("CARGO_PKG_VERSION")));
        }
    }
}

/// Compares `major.minor.patch` numerically; anything unparsable is never "newer".
fn newer(candidate: &str, current: &str) -> bool {
    let parse = |v: &str| -> Option<(u64, u64, u64)> {
        let core = v.split(['-', '+']).next()?;
        let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
        Some((parts.next()??, parts.next()??, parts.next()??))
    };
    matches!((parse(candidate), parse(current)), (Some(a), Some(b)) if a > b)
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_comparison() {
        assert!(super::newer("0.2.0", "0.1.9"));
        assert!(super::newer("1.0.0", "0.9.9"));
        assert!(!super::newer("0.1.0", "0.1.0"));
        assert!(!super::newer("garbage", "0.1.0"));
    }
}

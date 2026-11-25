//! Task 120 — the one place every on-screen time is formatted. No other
//! view calls `strftime` on a `jiff::Timestamp` directly (enforced by
//! `time_format_guard.rs`); the copied log (`history.rs::export_text`) and
//! generated release notes (`knotra-vcs::model::changelog::to_markdown`)
//! are untouched by this module on purpose — both are text meant to leave
//! the application (pasted into an issue tracker, read by someone in a
//! different time zone), so they stay in UTC, labelled, regardless of
//! whichever zone this file resolves for the screen.
//!
//! **The zone is read once, not per frame.** `local_zone()` caches
//! `jiff::tz::TimeZone::try_system()`'s own result for the life of the
//! process: the system's configured zone does not meaningfully change
//! while knotra is running, and re-detecting it on every redraw would mean
//! a fresh system lookup (a file read on Linux/macOS, a registry read on
//! Windows, per `TimeZone::try_system`'s own platform-behaviour doc) many
//! times a second, for a value that in practice never changes within a
//! session. A `TimeZone` value, once obtained, already carries whichever
//! IANA rules (DST included) apply to any instant it is asked to convert
//! — caching the *zone itself* loses nothing a per-frame re-detection
//! would have gained, only the repeated cost of finding it.

use std::sync::OnceLock;

use jiff::Timestamp;
use jiff::tz::TimeZone;

static LOCAL_ZONE: OnceLock<Option<TimeZone>> = OnceLock::new();

/// The computer's own time zone, detected once and cached — `None` when it
/// could not be determined, so [`format`]'s own caller falls back to UTC,
/// labelled, rather than show a silently wrong local time.
pub fn local_zone() -> Option<&'static TimeZone> {
    LOCAL_ZONE
        .get_or_init(|| TimeZone::try_system().ok())
        .as_ref()
}

/// Formats `ts` with `pattern` (a `strftime` pattern, carrying no time zone
/// text of its own) in `zone` — or, when `zone` is `None`, in UTC with a
/// trailing `" UTC"` this function appends itself, so a reader never sees
/// an unlabelled time that might silently be the wrong zone.
///
/// The one place every view-layer time is formatted (Task 120 §1) — every
/// other call site passes `local_zone()` through; a fixed `Some`/`None` is
/// for tests, which must not depend on whatever zone the machine running
/// them happens to be in.
pub fn format(ts: Timestamp, zone: Option<&TimeZone>, pattern: &str) -> String {
    match zone {
        Some(tz) => ts.to_zoned(tz.clone()).strftime(pattern).to_string(),
        None => format!("{} UTC", ts.strftime(pattern)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2024-03-04T05:06:07Z — every field distinct, the same fixed instant
    /// `history.rs`'s own `export_text_timestamp_format_is_stable` and
    /// `knotra-vcs::model::changelog`'s own stability test already use.
    fn fixed_instant() -> Timestamp {
        Timestamp::from_second(1_709_528_767).unwrap()
    }

    /// Asia/Tokyo is UTC+9, no DST — the owner's own zone (handoff §1's own
    /// worked example): 05:06 UTC becomes 14:06 local, same calendar day.
    #[test]
    fn asia_tokyo_is_nine_hours_ahead() {
        let tz = TimeZone::get("Asia/Tokyo").expect("Asia/Tokyo must be in the tzdb");
        assert_eq!(
            format(fixed_instant(), Some(&tz), "%Y-%m-%d %H:%M"),
            "2024-03-04 14:06"
        );
        assert_eq!(
            format(fixed_instant(), Some(&tz), "%m/%d %H:%M"),
            "03/04 14:06"
        );
    }

    /// America/New_York observes daylight saving — one instant in January
    /// (EST, UTC-5) and one in July (EDT, UTC-4), so the same helper must
    /// show a different offset for each, not a fixed one baked in.
    #[test]
    fn america_new_york_shows_daylight_saving() {
        let tz = TimeZone::get("America/New_York").expect("America/New_York must be in the tzdb");

        let january = Timestamp::from_second(1_705_320_000).unwrap(); // 2024-01-15T12:00:00Z
        assert_eq!(
            format(january, Some(&tz), "%Y-%m-%d %H:%M"),
            "2024-01-15 07:00",
            "EST is UTC-5"
        );

        let july = Timestamp::from_second(1_720_785_600).unwrap(); // 2024-07-12T12:00:00Z
        assert_eq!(
            format(july, Some(&tz), "%Y-%m-%d %H:%M"),
            "2024-07-12 08:00",
            "EDT is UTC-4"
        );
    }

    /// With no zone (the system's own could not be determined): UTC,
    /// labelled — never a bare, unlabelled time a reader could mistake for
    /// local.
    #[test]
    fn no_zone_falls_back_to_labelled_utc() {
        assert_eq!(
            format(fixed_instant(), None, "%Y-%m-%d %H:%M"),
            "2024-03-04 05:06 UTC"
        );
    }
}

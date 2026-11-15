//! Ported from the tests of `tests/Sobreiro.Tests/CopilotStateTests.cs` about
//! `CopilotStateText.Line`; the `Pick` test is in `copilot_shown.rs`. The last test walks rows
//! that the C# of 1.x wrote out itself.
use chrono::{DateTime, TimeDelta, Utc};
use sobreiro_core::copilot::copilot_state_text::line;
use sobreiro_core::copilot::{CopilotReading, CopilotSource};
use sobreiro_core::display::Zone;
use sobreiro_core::polling::ProviderState;

use crate::{at_ticks, en_gb, oracle_clock, oracle_zone, state_of, time};

fn now() -> DateTime<Utc> {
    time("2026-11-10T09:45:00Z")
}

fn seen() -> DateTime<Utc> {
    time("2026-11-10T09:25:00Z")
}

fn reading(observed: DateTime<Utc>, source: CopilotSource) -> CopilotReading {
    CopilotReading {
        budget: 15000.0,
        used: 2644.2,
        resets_at: None,
        observed_at: observed,
        source,
    }
}

fn say(state: ProviderState, reading: &CopilotReading) -> Option<String> {
    line(state, reading, now(), &Zone::utc(), &en_gb())
}

#[test]
fn fresh_live_ok_has_no_line() {
    let r = reading(now() - TimeDelta::minutes(15), CopilotSource::Live);
    assert_eq!(say(ProviderState::Ok, &r), None);
}

#[test]
fn old_live_ok_says_as_of() {
    let r = reading(seen(), CopilotSource::Live);
    assert_eq!(say(ProviderState::Ok, &r).as_deref(), Some("As of 09:25."));
}

#[test]
fn cache_says_from_the_copilot_cli() {
    let r = reading(seen(), CopilotSource::Cache);
    assert_eq!(
        say(ProviderState::Ok, &r).as_deref(),
        Some("From the Copilot CLI, as of 09:25.")
    );
}

#[test]
fn problem_comes_before_source() {
    let r = reading(seen(), CopilotSource::Cache);
    assert_eq!(
        say(ProviderState::Offline, &r).as_deref(),
        Some(
            "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes. From the Copilot CLI, as of 09:25."
        )
    );
}

#[test]
fn each_problem_has_its_sentence() {
    for (state, sentence) in [
        (
            ProviderState::NoSignIn,
            "No GitHub sign-in Sobreiro can use. Sign in with the Copilot CLI or gh.",
        ),
        (
            ProviderState::Refused,
            "GitHub didn't accept the sign-in. Sign in again with the Copilot CLI or gh.",
        ),
        (
            ProviderState::RateLimited,
            "GitHub asked Sobreiro to wait. Nothing to do: it asks again by itself.",
        ),
        (
            ProviderState::Offline,
            "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes.",
        ),
        (
            ProviderState::BadAnswer,
            "GitHub's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro.",
        ),
    ] {
        let r = reading(now(), CopilotSource::Live);
        assert_eq!(say(state, &r).as_deref(), Some(sentence), "{state:?}");
    }
}

/// The state, whether the reading is the cache's, when it was observed and the moment (ticks
/// from noon on 2026-11-09 UTC), the zone, the culture and the sentence.
type Row = (
    &'static str,
    bool,
    i64,
    i64,
    &'static str,
    &'static str,
    Option<&'static str>,
);

const ROWS: &[Row] = &[
    (
        "Off",
        false,
        783000000000,
        783000000000,
        "+09:00",
        "en-GB",
        None,
    ),
    (
        "Off",
        false,
        784200000001,
        793200000000,
        "+05:45",
        "wide",
        None,
    ),
    (
        "Off",
        false,
        773999999999,
        783000000000,
        "America/Santiago",
        "en-US",
        Some("As of 6:29 AM."),
    ),
    (
        "Off",
        false,
        711000000000,
        783000000000,
        "Australia/Lord_Howe",
        "pt-PT",
        Some("As of 18:45."),
    ),
    (
        "Off",
        false,
        -6129000000000,
        783000000000,
        "+09:00",
        "astral",
        Some("As of Mon 2 Nov 🌇🌇🌇🌇🌇🌇 6:45."),
    ),
    (
        "Off",
        true,
        784800000000,
        793200000000,
        "Europe/Lisbon",
        "pt-PT",
        Some("From the Copilot CLI, as of 09:48."),
    ),
    (
        "Off",
        true,
        794400000000,
        803400000000,
        "America/Havana",
        "astral",
        Some("From the Copilot CLI, as of 🌅🌅🌅🌅🌅🌅 6:04."),
    ),
    (
        "Off",
        true,
        773400000000,
        783000000000,
        "UTC",
        "en-GB",
        Some("From the Copilot CLI, as of 09:29."),
    ),
    (
        "Off",
        true,
        -286800000000,
        793200000000,
        "-05:00",
        "wide",
        Some("From the Copilot CLI, as of Sun 11:02 post meridiem do dia 🌇."),
    ),
    (
        "Off",
        true,
        784800000000,
        783000000000,
        "Europe/Lisbon",
        "en-US",
        Some("From the Copilot CLI, as of 9:48 AM."),
    ),
    (
        "Ok",
        false,
        803400000000,
        803400000000,
        "America/Santiago",
        "en-GB",
        None,
    ),
    (
        "Ok",
        false,
        784800000000,
        793200000000,
        "America/Havana",
        "pt-PT",
        None,
    ),
    (
        "Ok",
        false,
        784200000001,
        793200000000,
        "Australia/Lord_Howe",
        "wide",
        None,
    ),
    (
        "Ok",
        false,
        774000000000,
        783000000000,
        "UTC",
        "astral",
        None,
    ),
    (
        "Ok",
        false,
        773999999999,
        783000000000,
        "+09:00",
        "en-US",
        Some("As of 6:29 PM."),
    ),
    (
        "Ok",
        false,
        783600000000,
        793200000000,
        "-05:00",
        "en-GB",
        Some("As of 04:46."),
    ),
    (
        "Ok",
        false,
        721200000000,
        793200000000,
        "+05:45",
        "pt-PT",
        Some("As of 13:47."),
    ),
    (
        "Ok",
        false,
        -276600000000,
        803400000000,
        "Europe/Lisbon",
        "wide",
        Some("As of Mon 4:19 ÀÉÎÕÜ ante meridiem do dia."),
    ),
    (
        "Ok",
        false,
        -6129000000000,
        783000000000,
        "America/Santiago",
        "astral",
        Some("As of Mon 2 Nov 🌅🌅🌅🌅🌅🌅 6:45."),
    ),
    (
        "Ok",
        false,
        784800000000,
        783000000000,
        "America/Havana",
        "en-US",
        None,
    ),
    (
        "Ok",
        true,
        783000000000,
        783000000000,
        "Australia/Lord_Howe",
        "en-GB",
        Some("From the Copilot CLI, as of 20:45."),
    ),
    (
        "Ok",
        true,
        784800000000,
        793200000000,
        "UTC",
        "pt-PT",
        Some("From the Copilot CLI, as of 09:48."),
    ),
    (
        "Ok",
        true,
        794400000001,
        803400000000,
        "+09:00",
        "wide",
        Some("From the Copilot CLI, as of 7:04 post meridiem do dia 🌇."),
    ),
    (
        "Ok",
        true,
        774000000000,
        783000000000,
        "-05:00",
        "astral",
        Some("From the Copilot CLI, as of 🌅🌅🌅🌅🌅🌅 4:30."),
    ),
    (
        "Ok",
        true,
        773999999999,
        783000000000,
        "+05:45",
        "en-US",
        Some("From the Copilot CLI, as of 3:14 PM."),
    ),
    (
        "Ok",
        true,
        793800000000,
        803400000000,
        "Europe/Lisbon",
        "en-GB",
        Some("From the Copilot CLI, as of 10:03."),
    ),
    (
        "Ok",
        true,
        731400000000,
        803400000000,
        "America/Santiago",
        "pt-PT",
        Some("From the Copilot CLI, as of 05:19."),
    ),
    (
        "Ok",
        true,
        -276600000000,
        803400000000,
        "America/Havana",
        "wide",
        Some("From the Copilot CLI, as of Mon 12:19 ÀÉÎÕÜ ante meridiem do dia."),
    ),
    (
        "Ok",
        true,
        -6118800000000,
        793200000000,
        "Australia/Lord_Howe",
        "astral",
        Some("From the Copilot CLI, as of Mon 2 Nov 🌇🌇🌇🌇🌇🌇 9:02."),
    ),
    (
        "Ok",
        true,
        784800000000,
        783000000000,
        "UTC",
        "en-US",
        Some("From the Copilot CLI, as of 9:48 AM."),
    ),
    (
        "Expired",
        false,
        793200000000,
        793200000000,
        "+09:00",
        "en-GB",
        None,
    ),
    (
        "Expired",
        false,
        794400000001,
        803400000000,
        "+05:45",
        "wide",
        None,
    ),
    (
        "Expired",
        false,
        794399999999,
        803400000000,
        "America/Santiago",
        "en-US",
        Some("As of 7:03 AM."),
    ),
    (
        "Expired",
        false,
        731400000000,
        803400000000,
        "Australia/Lord_Howe",
        "pt-PT",
        Some("As of 19:19."),
    ),
    (
        "Expired",
        false,
        -6118800000000,
        793200000000,
        "+09:00",
        "astral",
        Some("As of Mon 2 Nov 🌇🌇🌇🌇🌇🌇 7:02."),
    ),
    (
        "Expired",
        true,
        774600000000,
        783000000000,
        "Europe/Lisbon",
        "pt-PT",
        Some("From the Copilot CLI, as of 09:31."),
    ),
    (
        "Expired",
        true,
        774000000000,
        783000000000,
        "America/Havana",
        "astral",
        Some("From the Copilot CLI, as of 🌅🌅🌅🌅🌅🌅 5:30."),
    ),
    (
        "Expired",
        true,
        783600000000,
        793200000000,
        "UTC",
        "en-GB",
        Some("From the Copilot CLI, as of 09:46."),
    ),
    (
        "Expired",
        true,
        -297000000000,
        783000000000,
        "-05:00",
        "wide",
        Some("From the Copilot CLI, as of Sun 10:45 post meridiem do dia 🌇."),
    ),
    (
        "Expired",
        true,
        805200000000,
        803400000000,
        "Europe/Lisbon",
        "en-US",
        Some("From the Copilot CLI, as of 10:22 AM."),
    ),
    (
        "NoSignIn",
        false,
        793200000000,
        793200000000,
        "America/Santiago",
        "en-GB",
        Some("No GitHub sign-in Sobreiro can use. Sign in with the Copilot CLI or gh."),
    ),
    (
        "NoSignIn",
        false,
        794400000001,
        803400000000,
        "Australia/Lord_Howe",
        "wide",
        Some("No GitHub sign-in Sobreiro can use. Sign in with the Copilot CLI or gh."),
    ),
    (
        "NoSignIn",
        false,
        794399999999,
        803400000000,
        "+09:00",
        "en-US",
        Some(
            "No GitHub sign-in Sobreiro can use. Sign in with the Copilot CLI or gh. As of 7:03 PM.",
        ),
    ),
    (
        "NoSignIn",
        false,
        721200000000,
        793200000000,
        "+05:45",
        "pt-PT",
        Some(
            "No GitHub sign-in Sobreiro can use. Sign in with the Copilot CLI or gh. As of 13:47.",
        ),
    ),
    (
        "NoSignIn",
        false,
        -6129000000000,
        783000000000,
        "America/Santiago",
        "astral",
        Some(
            "No GitHub sign-in Sobreiro can use. Sign in with the Copilot CLI or gh. As of Mon 2 Nov 🌅🌅🌅🌅🌅🌅 6:45.",
        ),
    ),
    (
        "NoSignIn",
        true,
        774600000000,
        783000000000,
        "UTC",
        "pt-PT",
        Some(
            "No GitHub sign-in Sobreiro can use. Sign in with the Copilot CLI or gh. From the Copilot CLI, as of 09:31.",
        ),
    ),
    (
        "NoSignIn",
        true,
        784200000000,
        793200000000,
        "-05:00",
        "astral",
        Some(
            "No GitHub sign-in Sobreiro can use. Sign in with the Copilot CLI or gh. From the Copilot CLI, as of 🌅🌅🌅🌅🌅🌅 4:47.",
        ),
    ),
    (
        "NoSignIn",
        true,
        783600000000,
        793200000000,
        "Europe/Lisbon",
        "en-GB",
        Some(
            "No GitHub sign-in Sobreiro can use. Sign in with the Copilot CLI or gh. From the Copilot CLI, as of 09:46.",
        ),
    ),
    (
        "NoSignIn",
        true,
        -276600000000,
        803400000000,
        "America/Havana",
        "wide",
        Some(
            "No GitHub sign-in Sobreiro can use. Sign in with the Copilot CLI or gh. From the Copilot CLI, as of Mon 12:19 ÀÉÎÕÜ ante meridiem do dia.",
        ),
    ),
    (
        "NoSignIn",
        true,
        805200000000,
        803400000000,
        "UTC",
        "en-US",
        Some(
            "No GitHub sign-in Sobreiro can use. Sign in with the Copilot CLI or gh. From the Copilot CLI, as of 10:22 AM.",
        ),
    ),
    (
        "Refused",
        false,
        793200000000,
        793200000000,
        "+09:00",
        "en-GB",
        Some("GitHub didn't accept the sign-in. Sign in again with the Copilot CLI or gh."),
    ),
    (
        "Refused",
        false,
        784200000001,
        793200000000,
        "+05:45",
        "wide",
        Some("GitHub didn't accept the sign-in. Sign in again with the Copilot CLI or gh."),
    ),
    (
        "Refused",
        false,
        794399999999,
        803400000000,
        "America/Santiago",
        "en-US",
        Some(
            "GitHub didn't accept the sign-in. Sign in again with the Copilot CLI or gh. As of 7:03 AM.",
        ),
    ),
    (
        "Refused",
        false,
        711000000000,
        783000000000,
        "Australia/Lord_Howe",
        "pt-PT",
        Some(
            "GitHub didn't accept the sign-in. Sign in again with the Copilot CLI or gh. As of 18:45.",
        ),
    ),
    (
        "Refused",
        false,
        -6129000000000,
        783000000000,
        "+09:00",
        "astral",
        Some(
            "GitHub didn't accept the sign-in. Sign in again with the Copilot CLI or gh. As of Mon 2 Nov 🌇🌇🌇🌇🌇🌇 6:45.",
        ),
    ),
    (
        "Refused",
        true,
        784800000000,
        793200000000,
        "Europe/Lisbon",
        "pt-PT",
        Some(
            "GitHub didn't accept the sign-in. Sign in again with the Copilot CLI or gh. From the Copilot CLI, as of 09:48.",
        ),
    ),
    (
        "Refused",
        true,
        794400000000,
        803400000000,
        "America/Havana",
        "astral",
        Some(
            "GitHub didn't accept the sign-in. Sign in again with the Copilot CLI or gh. From the Copilot CLI, as of 🌅🌅🌅🌅🌅🌅 6:04.",
        ),
    ),
    (
        "Refused",
        true,
        793800000000,
        803400000000,
        "UTC",
        "en-GB",
        Some(
            "GitHub didn't accept the sign-in. Sign in again with the Copilot CLI or gh. From the Copilot CLI, as of 10:03.",
        ),
    ),
    (
        "Refused",
        true,
        -276600000000,
        803400000000,
        "-05:00",
        "wide",
        Some(
            "GitHub didn't accept the sign-in. Sign in again with the Copilot CLI or gh. From the Copilot CLI, as of Sun 11:19 post meridiem do dia 🌇.",
        ),
    ),
    (
        "Refused",
        true,
        784800000000,
        783000000000,
        "Europe/Lisbon",
        "en-US",
        Some(
            "GitHub didn't accept the sign-in. Sign in again with the Copilot CLI or gh. From the Copilot CLI, as of 9:48 AM.",
        ),
    ),
    (
        "RateLimited",
        false,
        803400000000,
        803400000000,
        "America/Santiago",
        "en-GB",
        Some("GitHub asked Sobreiro to wait. Nothing to do: it asks again by itself."),
    ),
    (
        "RateLimited",
        false,
        774000000001,
        783000000000,
        "Australia/Lord_Howe",
        "wide",
        Some("GitHub asked Sobreiro to wait. Nothing to do: it asks again by itself."),
    ),
    (
        "RateLimited",
        false,
        784199999999,
        793200000000,
        "+09:00",
        "en-US",
        Some(
            "GitHub asked Sobreiro to wait. Nothing to do: it asks again by itself. As of 6:46 PM.",
        ),
    ),
    (
        "RateLimited",
        false,
        721200000000,
        793200000000,
        "+05:45",
        "pt-PT",
        Some("GitHub asked Sobreiro to wait. Nothing to do: it asks again by itself. As of 13:47."),
    ),
    (
        "RateLimited",
        false,
        -6118800000000,
        793200000000,
        "America/Santiago",
        "astral",
        Some(
            "GitHub asked Sobreiro to wait. Nothing to do: it asks again by itself. As of Mon 2 Nov 🌅🌅🌅🌅🌅🌅 7:02.",
        ),
    ),
    (
        "RateLimited",
        true,
        774600000000,
        783000000000,
        "UTC",
        "pt-PT",
        Some(
            "GitHub asked Sobreiro to wait. Nothing to do: it asks again by itself. From the Copilot CLI, as of 09:31.",
        ),
    ),
    (
        "RateLimited",
        true,
        794400000000,
        803400000000,
        "-05:00",
        "astral",
        Some(
            "GitHub asked Sobreiro to wait. Nothing to do: it asks again by itself. From the Copilot CLI, as of 🌅🌅🌅🌅🌅🌅 5:04.",
        ),
    ),
    (
        "RateLimited",
        true,
        783600000000,
        793200000000,
        "Europe/Lisbon",
        "en-GB",
        Some(
            "GitHub asked Sobreiro to wait. Nothing to do: it asks again by itself. From the Copilot CLI, as of 09:46.",
        ),
    ),
    (
        "RateLimited",
        true,
        -286800000000,
        793200000000,
        "America/Havana",
        "wide",
        Some(
            "GitHub asked Sobreiro to wait. Nothing to do: it asks again by itself. From the Copilot CLI, as of Mon 12:02 ÀÉÎÕÜ ante meridiem do dia.",
        ),
    ),
    (
        "RateLimited",
        true,
        784800000000,
        783000000000,
        "UTC",
        "en-US",
        Some(
            "GitHub asked Sobreiro to wait. Nothing to do: it asks again by itself. From the Copilot CLI, as of 9:48 AM.",
        ),
    ),
    (
        "Offline",
        false,
        793200000000,
        793200000000,
        "+09:00",
        "en-GB",
        Some("Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes."),
    ),
    (
        "Offline",
        false,
        774600000000,
        783000000000,
        "-05:00",
        "pt-PT",
        Some("Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes."),
    ),
    (
        "Offline",
        false,
        794400000001,
        803400000000,
        "+05:45",
        "wide",
        Some("Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes."),
    ),
    (
        "Offline",
        false,
        784200000000,
        793200000000,
        "Europe/Lisbon",
        "astral",
        Some("Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes."),
    ),
    (
        "Offline",
        false,
        784199999999,
        793200000000,
        "America/Santiago",
        "en-US",
        Some(
            "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes. As of 6:46 AM.",
        ),
    ),
    (
        "Offline",
        false,
        773400000000,
        783000000000,
        "America/Havana",
        "en-GB",
        Some(
            "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes. As of 05:29.",
        ),
    ),
    (
        "Offline",
        false,
        721200000000,
        793200000000,
        "Australia/Lord_Howe",
        "pt-PT",
        Some(
            "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes. As of 19:02.",
        ),
    ),
    (
        "Offline",
        false,
        -286800000000,
        793200000000,
        "UTC",
        "wide",
        Some(
            "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes. As of Mon 4:02 ÀÉÎÕÜ ante meridiem do dia.",
        ),
    ),
    (
        "Offline",
        false,
        -6129000000000,
        783000000000,
        "+09:00",
        "astral",
        Some(
            "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes. As of Mon 2 Nov 🌇🌇🌇🌇🌇🌇 6:45.",
        ),
    ),
    (
        "Offline",
        false,
        784800000000,
        783000000000,
        "-05:00",
        "en-US",
        Some("Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes."),
    ),
    (
        "Offline",
        true,
        793200000000,
        793200000000,
        "+05:45",
        "en-GB",
        Some(
            "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes. From the Copilot CLI, as of 15:47.",
        ),
    ),
    (
        "Offline",
        true,
        774600000000,
        783000000000,
        "Europe/Lisbon",
        "pt-PT",
        Some(
            "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes. From the Copilot CLI, as of 09:31.",
        ),
    ),
    (
        "Offline",
        true,
        794400000001,
        803400000000,
        "America/Santiago",
        "wide",
        Some(
            "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes. From the Copilot CLI, as of 7:04 ÀÉÎÕÜ ante meridiem do dia.",
        ),
    ),
    (
        "Offline",
        true,
        794400000000,
        803400000000,
        "America/Havana",
        "astral",
        Some(
            "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes. From the Copilot CLI, as of 🌅🌅🌅🌅🌅🌅 6:04.",
        ),
    ),
    (
        "Offline",
        true,
        784199999999,
        793200000000,
        "Australia/Lord_Howe",
        "en-US",
        Some(
            "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes. From the Copilot CLI, as of 8:46 PM.",
        ),
    ),
    (
        "Offline",
        true,
        773400000000,
        783000000000,
        "UTC",
        "en-GB",
        Some(
            "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes. From the Copilot CLI, as of 09:29.",
        ),
    ),
    (
        "Offline",
        true,
        711000000000,
        783000000000,
        "+09:00",
        "pt-PT",
        Some(
            "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes. From the Copilot CLI, as of 16:45.",
        ),
    ),
    (
        "Offline",
        true,
        -276600000000,
        803400000000,
        "-05:00",
        "wide",
        Some(
            "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes. From the Copilot CLI, as of Sun 11:19 post meridiem do dia 🌇.",
        ),
    ),
    (
        "Offline",
        true,
        -6108600000000,
        803400000000,
        "+05:45",
        "astral",
        Some(
            "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes. From the Copilot CLI, as of Mon 2 Nov 🌇🌇🌇🌇🌇🌇 4:04.",
        ),
    ),
    (
        "Offline",
        true,
        805200000000,
        803400000000,
        "Europe/Lisbon",
        "en-US",
        Some(
            "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes. From the Copilot CLI, as of 10:22 AM.",
        ),
    ),
    (
        "BadAnswer",
        false,
        793200000000,
        793200000000,
        "America/Santiago",
        "en-GB",
        Some(
            "GitHub's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro.",
        ),
    ),
    (
        "BadAnswer",
        false,
        774000000001,
        783000000000,
        "Australia/Lord_Howe",
        "wide",
        Some(
            "GitHub's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro.",
        ),
    ),
    (
        "BadAnswer",
        false,
        784199999999,
        793200000000,
        "+09:00",
        "en-US",
        Some(
            "GitHub's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro. As of 6:46 PM.",
        ),
    ),
    (
        "BadAnswer",
        false,
        731400000000,
        803400000000,
        "+05:45",
        "pt-PT",
        Some(
            "GitHub's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro. As of 14:04.",
        ),
    ),
    (
        "BadAnswer",
        false,
        -6108600000000,
        803400000000,
        "America/Santiago",
        "astral",
        Some(
            "GitHub's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro. As of Mon 2 Nov 🌅🌅🌅🌅🌅🌅 7:19.",
        ),
    ),
    (
        "BadAnswer",
        true,
        784800000000,
        793200000000,
        "UTC",
        "pt-PT",
        Some(
            "GitHub's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro. From the Copilot CLI, as of 09:48.",
        ),
    ),
    (
        "BadAnswer",
        true,
        774000000000,
        783000000000,
        "-05:00",
        "astral",
        Some(
            "GitHub's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro. From the Copilot CLI, as of 🌅🌅🌅🌅🌅🌅 4:30.",
        ),
    ),
    (
        "BadAnswer",
        true,
        783600000000,
        793200000000,
        "Europe/Lisbon",
        "en-GB",
        Some(
            "GitHub's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro. From the Copilot CLI, as of 09:46.",
        ),
    ),
    (
        "BadAnswer",
        true,
        -276600000000,
        803400000000,
        "America/Havana",
        "wide",
        Some(
            "GitHub's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro. From the Copilot CLI, as of Mon 12:19 ÀÉÎÕÜ ante meridiem do dia.",
        ),
    ),
    (
        "BadAnswer",
        true,
        805200000000,
        803400000000,
        "UTC",
        "en-US",
        Some(
            "GitHub's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro. From the Copilot CLI, as of 10:22 AM.",
        ),
    ),
];

#[test]
fn copilot_state_text_answers_as_the_csharp_did() {
    for &(state, cache, observed, now, zone, clock, expected) in ROWS {
        let reading = reading(
            at_ticks(observed),
            if cache {
                CopilotSource::Cache
            } else {
                CopilotSource::Live
            },
        );
        let text = line(
            state_of(state),
            &reading,
            at_ticks(now),
            &oracle_zone(zone),
            &oracle_clock(clock),
        );
        assert_eq!(
            text.as_deref(),
            expected,
            "{state} cache {cache} observed {observed} now {now} {zone} {clock}"
        );
    }
}

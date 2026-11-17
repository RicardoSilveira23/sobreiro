//! Measures a Sobreiro release build from outside: time to the first pill, memory and processor
//! time when idle, the size of both exes and the bridge's run time. Prints one table row for
//! docs/baseline.md and, with `--table`, writes it there.

mod measure;
mod native;

use measure::Measure;
use sobreiro_baseline::{BaselineRun, DIVIDER, HEADER, IdleFigures, add, median, row};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

const USAGE: &str = "\
sobreiro-baseline [--exe <Sobreiro.exe>] [--bridge <sobreiro-bridge.exe>] [--label <build>]
                  [--tray run|size] [--idle-minutes 10] [--starts 5] [--bridge-runs 20]
                  [--table <baseline.md> --section <heading>]

Starts its own copy of the tray on a scratch data folder and measures it: time to the first
pill, memory and processor time when idle, the size of both exes and the bridge's run time.
With --tray size, or with no --exe, the tray is not started: only sizes and the bridge's run
time are taken. Prints one table row; with --table, writes it into that file too.";

fn say(line: &str) {
    let (_, hour, minute, second) = native::local_now();
    println!("{hour:02}:{minute:02}:{second:02} {line}");
}

fn full_path(path: &str) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| PathBuf::from(path))
}

/// The `--name value` pairs, or None when an argument has no name or no value.
fn options(args: &[String]) -> Option<HashMap<String, String>> {
    let mut found = HashMap::new();
    for pair in args.chunks(2) {
        let name = pair.first()?.strip_prefix("--")?;
        found.insert(name.to_string(), pair.get(1)?.clone());
    }
    Some(found)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(options) = options(&args) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let exe = options.get("exe").map(|e| full_path(e));
    let bridge_exe = options.get("bridge").map(|b| full_path(b));
    let tray_mode = options.get("tray").map_or("run", String::as_str);
    let Some(labelled) = exe.clone().or_else(|| bridge_exe.clone()) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    if !matches!(tray_mode, "run" | "size")
        || options.contains_key("table") != options.contains_key("section")
    {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let number = |name: &str, fallback: usize, least: usize| {
        options
            .get(name)
            .and_then(|text| text.parse::<usize>().ok())
            .map_or(fallback, |n| n.max(least))
    };
    let run_tray = exe.is_some() && tray_mode == "run";
    let idle_minutes = number("idle-minutes", 10, 2);
    let starts = number("starts", 5, 2);
    let bridge_runs = number("bridge-runs", 20, 1);
    let label = options
        .get("label")
        .cloned()
        .or_else(|| native::product_version(&labelled))
        .unwrap_or_else(|| "unknown".to_string());

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let scratch = std::env::temp_dir().join(format!(
        "sobreiro-baseline-{:08x}",
        nanos ^ std::process::id().rotate_left(16)
    ));
    let outcome = std::fs::create_dir_all(&scratch)
        .map_err(|e| e.to_string())
        .and_then(|()| {
            run(&Plan {
                exe: exe.as_deref(),
                bridge_exe: bridge_exe.as_deref(),
                run_tray,
                idle_minutes,
                starts,
                bridge_runs,
                label: &label,
                labelled: &labelled,
                table: options
                    .get("table")
                    .map(|t| (t.as_str(), options["section"].as_str())),
                scratch: &scratch,
            })
        });
    let code = match outcome {
        Ok(code) => code,
        Err(message) => {
            eprintln!("Baseline failed: {message}");
            1
        }
    };
    if std::fs::remove_dir_all(&scratch).is_err() {
        eprintln!("Could not remove {}.", scratch.display());
    }
    ExitCode::from(code)
}

struct Plan<'a> {
    exe: Option<&'a Path>,
    bridge_exe: Option<&'a Path>,
    run_tray: bool,
    idle_minutes: usize,
    starts: usize,
    bridge_runs: usize,
    label: &'a str,
    labelled: &'a Path,
    table: Option<(&'a str, &'a str)>,
    scratch: &'a Path,
}

fn run(plan: &Plan) -> Result<u8, String> {
    if plan.run_tray
        && let Some(other) = native::pill_of_another_app()
    {
        eprintln!(
            "A pill is already in the taskbar (process {other}, {}). A second one finds no room beside it and \
             stays hidden, so quit that app first and start it again afterwards.",
            native::name_of(other)
        );
        return Ok(1);
    }
    let _timer = native::FineTimer::start();
    let measure = Measure {
        scratch: plan.scratch,
        say: &say,
    };
    say(&format!(
        "build {}: {}",
        plan.label,
        plan.labelled.display()
    ));
    let mut pills: Vec<Option<Duration>> = vec![None];
    let mut idle: Option<IdleFigures> = None;
    if let (true, Some(exe)) = (plan.run_tray, plan.exe) {
        say(&format!("{} starts, signed out", plan.starts));
        pills = measure.starts(exe, plan.starts)?;
        say(&format!(
            "idle for {} minutes, with this PC's sign-ins",
            plan.idle_minutes
        ));
        idle = Some(measure.idle(exe, plan.idle_minutes as u64)?);
    }
    let mut bridge_run = None;
    if let Some(bridge) = plan.bridge_exe {
        say(&format!("{} bridge runs", plan.bridge_runs));
        bridge_run = measure.bridge(bridge, plan.bridge_runs)?;
    }

    let size = |path: Option<&Path>| -> Result<Option<u64>, String> {
        path.map(|p| {
            std::fs::metadata(p)
                .map(|m| m.len())
                .map_err(|e| format!("{}: {e}", p.display()))
        })
        .transpose()
    };
    let later: Vec<Duration> = pills.iter().skip(1).flatten().copied().collect();
    let run = BaselineRun {
        build: plan.label.to_string(),
        date: native::local_now().0,
        idle,
        first_pill_first_start: pills[0],
        first_pill_later_starts: median(&later),
        tray_exe_bytes: size(plan.exe)?,
        bridge_exe_bytes: size(plan.bridge_exe)?,
        bridge_run,
    };
    println!("{HEADER}");
    println!("{DIVIDER}");
    println!("{}", row(&run));

    if let Some((table, section)) = plan.table {
        let text = std::fs::read_to_string(table).map_err(|e| format!("{table}: {e}"))?;
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
        let Some(updated) = add(text, section, &run) else {
            eprintln!("No table under \"## {section}\" in {table}: nothing written.");
            return Ok(1);
        };
        std::fs::write(table, updated).map_err(|e| format!("{table}: {e}"))?;
        say(&format!("row written to {table}, under {section}"));
    }
    Ok(0)
}

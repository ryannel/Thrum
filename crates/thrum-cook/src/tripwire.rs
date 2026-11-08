//! Asserted tripwires: bench measurements checked against their own stored history, so a
//! regression fails visibly instead of only being logged.
//!
//! A history is a small text file of runs, one per line: tab-separated `key=value` fields with
//! the time, revision, load average, logical CPU count, load class and each metric (lower is
//! better). A new run is compared only with earlier runs of the same load class, because a
//! shared machine's timings depend on how busy it is. Each metric's limit is
//! `max(median × factor, floor)` over at least [`MIN_HISTORY`] same-class runs; with fewer, the
//! metric is still being seeded and only recorded. The check never edits the history; a caller
//! appends a run that passed when asked to record.

use std::fmt::Write as _;
use std::path::Path;

/// Same-class runs a metric needs before it is checked.
pub const MIN_HISTORY: usize = 3;

/// How busy the machine was: the one-minute load average per logical CPU.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadClass {
    /// Below one runnable thread per CPU.
    Quiet,
    /// One to four per CPU.
    Busy,
    /// More than four per CPU.
    Saturated,
}

impl LoadClass {
    /// The class of `load` on `cpus` logical CPUs.
    pub fn of(load: f64, cpus: usize) -> Self {
        let per_cpu = load / cpus.max(1) as f64;
        if per_cpu < 1.0 {
            Self::Quiet
        } else if per_cpu <= 4.0 {
            Self::Busy
        } else {
            Self::Saturated
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Quiet => "quiet",
            Self::Busy => "busy",
            Self::Saturated => "saturated",
        }
    }

    fn parse(name: &str) -> Option<Self> {
        [Self::Quiet, Self::Busy, Self::Saturated]
            .into_iter()
            .find(|class| class.name() == name)
    }
}

/// One bench run.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    /// Seconds since the Unix epoch.
    pub unix: u64,
    /// Short revision the bench was built from.
    pub revision: String,
    /// One-minute load average when the run finished.
    pub load: f64,
    /// Logical CPUs.
    pub cpus: usize,
    /// Metric name and value, in report order.
    pub metrics: Vec<(String, f64)>,
}

impl Run {
    /// This machine's load class for the run.
    pub fn class(&self) -> LoadClass {
        LoadClass::of(self.load, self.cpus)
    }

    fn metric(&self, name: &str) -> Option<f64> {
        self.metrics
            .iter()
            .find(|(metric, _)| metric == name)
            .map(|&(_, value)| value)
    }

    /// The history line for this run.
    pub fn line(&self) -> String {
        let mut line = format!(
            "unix={}\trev={}\tload={:.2}\tcpus={}\tclass={}",
            self.unix,
            self.revision,
            self.load,
            self.cpus,
            self.class().name()
        );
        for (name, value) in &self.metrics {
            write!(line, "\t{name}={value:.4}").ok();
        }
        line
    }

    /// Parse a history line; `None` for a malformed one.
    pub fn parse(line: &str) -> Option<Self> {
        let mut run = Self {
            unix: 0,
            revision: String::new(),
            load: f64::NAN,
            cpus: 0,
            metrics: Vec::new(),
        };
        let mut class = None;
        for field in line.split('\t') {
            let (key, value) = field.split_once('=')?;
            match key {
                "unix" => run.unix = value.parse().ok()?,
                "rev" => value.clone_into(&mut run.revision),
                "load" => run.load = value.parse().ok()?,
                "cpus" => run.cpus = value.parse().ok()?,
                "class" => class = LoadClass::parse(value),
                _ => run.metrics.push((key.to_owned(), value.parse().ok()?)),
            }
        }
        (class == Some(run.class()) && run.cpus > 0).then_some(run)
    }
}

/// How one metric is judged.
#[derive(Clone, Copy, Debug)]
pub struct Rule {
    /// Metric name as the bench records it.
    pub metric: &'static str,
    /// Allowed multiple of the same-class history median.
    pub factor: f64,
    /// Limit never below this (in the metric's unit), so tiny timings do not flap.
    pub floor: f64,
}

/// The outcome of checking a run.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Verdict {
    /// Metrics over their limit, each with its value, limit and history size.
    pub regressions: Vec<String>,
    /// Metrics within their limit.
    pub passed: Vec<String>,
    /// Metrics without enough same-class history yet.
    pub seeding: Vec<String>,
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        f64::midpoint(values[middle - 1], values[middle])
    } else {
        values[middle]
    }
}

/// Check `run` against same-class runs of `history` under `rules`.
pub fn check(history: &[Run], run: &Run, rules: &[Rule]) -> Verdict {
    let class = run.class();
    let mut verdict = Verdict::default();
    for rule in rules {
        let Some(value) = run.metric(rule.metric) else {
            verdict
                .regressions
                .push(format!("{}: missing from this run", rule.metric));
            continue;
        };
        let mut past: Vec<f64> = history
            .iter()
            .filter(|past| past.class() == class)
            .filter_map(|past| past.metric(rule.metric))
            .collect();
        if past.len() < MIN_HISTORY {
            verdict.seeding.push(format!(
                "{}: {value:.3} ({} of {MIN_HISTORY} {} runs)",
                rule.metric,
                past.len(),
                class.name()
            ));
            continue;
        }
        let count = past.len();
        let limit = (median(&mut past) * rule.factor).max(rule.floor);
        let line = format!(
            "{}: {value:.3} against a limit of {limit:.3} ({count} {} runs)",
            rule.metric,
            class.name()
        );
        if value > limit {
            verdict.regressions.push(line);
        } else {
            verdict.passed.push(line);
        }
    }
    verdict
}

/// Read a history file; a missing file is an empty history and malformed lines are skipped.
pub fn load_history(path: &Path) -> Vec<Run> {
    std::fs::read_to_string(path)
        .map(|text| text.lines().filter_map(Run::parse).collect())
        .unwrap_or_default()
}

/// Append `run` to a history file.
///
/// # Errors
/// I/O failure.
pub fn append(path: &Path, run: &Run) -> std::io::Result<()> {
    use std::io::Write as _;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{}", run.line())
}

/// The one-minute load average, or `NaN` where it cannot be read.
pub fn load_average() -> f64 {
    let read = |program: &str, args: &[&str]| {
        std::process::Command::new(program)
            .args(args)
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
    };
    // macOS `vm.loadavg` is "{ 1.23 4.56 7.89 }"; Linux /proc/loadavg starts with the 1-minute value.
    let text = std::fs::read_to_string("/proc/loadavg")
        .ok()
        .or_else(|| read("sysctl", &["-n", "vm.loadavg"]))
        .unwrap_or_default();
    text.split_whitespace()
        .find_map(|field| field.parse().ok())
        .unwrap_or(f64::NAN)
}

/// A run of `metrics` measured now on this machine at this checkout's revision.
pub fn this_run(metrics: Vec<(String, f64)>) -> Run {
    let revision = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map_or_else(|| "unknown".to_owned(), |text| text.trim().to_owned());
    Run {
        unix: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs()),
        revision,
        load: load_average(),
        cpus: std::thread::available_parallelism().map_or(1, std::num::NonZero::get),
        metrics,
    }
}

/// Check `run` against the history at `path`, append it when `record` is set and it passed,
/// and describe the outcome.
///
/// # Errors
/// The run regressed (the message names every metric over its limit), or the history could
/// not be appended.
pub fn gate(path: &Path, run: &Run, rules: &[Rule], record: bool) -> Result<String, String> {
    let verdict = check(&load_history(path), run, rules);
    let mut report = format!(
        "tripwires ({}, load {:.1} on {} CPUs, {}):\n",
        path.display(),
        run.load,
        run.cpus,
        run.class().name()
    );
    for (label, lines) in [
        ("REGRESSED", &verdict.regressions),
        ("ok", &verdict.passed),
        ("seeding", &verdict.seeding),
    ] {
        for line in lines {
            writeln!(report, "  {label} {line}").ok();
        }
    }
    if !verdict.regressions.is_empty() {
        return Err(report);
    }
    if record {
        append(path, run).map_err(|error| format!("{report}history not appended: {error}"))?;
        report.push_str("  recorded\n");
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(load: f64, value: f64) -> Run {
        Run {
            unix: 1,
            revision: "abc".into(),
            load,
            cpus: 16,
            metrics: vec![("scan_ms".into(), value)],
        }
    }

    const RULE: Rule = Rule {
        metric: "scan_ms",
        factor: 2.0,
        floor: 1.0,
    };

    #[test]
    fn a_metric_is_seeded_then_checked_against_its_own_load_class() {
        // Load 4 on 16 CPUs is quiet; 40 is busy.
        let mut history = vec![run(4.0, 10.0), run(4.0, 12.0)];
        assert_eq!(check(&history, &run(4.0, 100.0), &[RULE]).seeding.len(), 1);
        history.push(run(4.0, 11.0));
        assert_eq!(
            check(&history, &run(4.0, 100.0), &[RULE]).regressions.len(),
            1
        );
        // Quiet history does not judge a busy run.
        let busy = check(&history, &run(40.0, 100.0), &[RULE]);
        assert!(busy.seeding.len() == 1 && busy.regressions.is_empty());
        let history: Vec<Run> = [10.0, 12.0, 11.0]
            .into_iter()
            .map(|value| run(40.0, value))
            .collect();
        assert_eq!(check(&history, &run(40.0, 22.0), &[RULE]).passed.len(), 1);
        let regressed = check(&history, &run(40.0, 22.1), &[RULE]);
        assert_eq!(regressed.regressions.len(), 1, "{regressed:?}");
        assert!(regressed.regressions[0].starts_with("scan_ms: 22.100 against a limit of 22.000"));
    }

    #[test]
    fn the_floor_holds_for_tiny_timings_and_a_missing_metric_fails() {
        let history: Vec<Run> = (0..3).map(|_| run(40.0, 0.01)).collect();
        assert_eq!(
            check(&history, &run(40.0, 0.9), &[RULE]).regressions,
            Vec::<String>::new()
        );
        let mut missing = run(40.0, 1.0);
        missing.metrics.clear();
        assert_eq!(check(&history, &missing, &[RULE]).regressions.len(), 1);
    }

    #[test]
    fn history_lines_round_trip_and_malformed_lines_are_skipped() {
        let original = run(40.5, 3.25);
        assert_eq!(Run::parse(&original.line()), Some(original.clone()));
        assert_eq!(
            Run::parse("unix=1\trev=x\tload=1\tcpus=16\tclass=saturated"),
            None
        );
        assert_eq!(Run::parse("garbage"), None);
        let path = std::env::temp_dir().join(format!("thrum-tripwire-{}", std::process::id()));
        std::fs::remove_file(&path).ok();
        append(&path, &original).expect("append");
        std::fs::write(
            &path,
            format!(
                "{}\nnot a run\n",
                std::fs::read_to_string(&path).expect("read")
            ),
        )
        .expect("write");
        assert_eq!(load_history(&path), vec![original]);
        std::fs::remove_file(&path).expect("remove");
    }
}

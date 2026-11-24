//! Reading cargo-nextest's JUnit report for `./dev check`, and running once more, alone, each test
//! the limit on one test stopped side by side (GATE-TIME mechanism 13).

use std::{error::Error, path::Path, process::Command};

use crate::{NEXTEST_STORE, TaskResult};

/// One test's result from cargo-nextest's JUnit report.
pub(crate) struct TestTime {
    /// `<package>::<binary> <test>`, as cargo-nextest names it.
    pub(crate) name: String,
    pub(crate) seconds: f64,
    pub(crate) timed_out: bool,
    /// Failed in any way, a stop by the limit included.
    pub(crate) failed: bool,
    /// Run in the draw-probe pass, a second build of one binary, rather than the workspace pass.
    pub(crate) probe: bool,
}

/// The value of `attribute="..."` in one XML start tag, unescaped.
fn xml_attribute(tag: &str, attribute: &str) -> Option<String> {
    let start = tag.find(&format!(" {attribute}=\""))? + attribute.len() + 3;
    let value = &tag[start..start + tag[start..].find('"')?];
    let mut text = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(at) = rest.find('&') {
        text.push_str(&rest[..at]);
        rest = &rest[at..];
        let entity = [
            ("&lt;", '<'),
            ("&gt;", '>'),
            ("&quot;", '"'),
            ("&apos;", '\''),
            ("&amp;", '&'),
        ]
        .into_iter()
        .find(|(entity, _)| rest.starts_with(entity));
        let (skip, character) = entity.map_or((1, '&'), |(entity, c)| (entity.len(), c));
        text.push(character);
        rest = &rest[skip..];
    }
    text.push_str(rest);
    Some(text)
}

/// Every test case of a cargo-nextest JUnit report. A case whose failure says it timed out was
/// stopped by the slow-test limit of `.config/nextest.toml`.
pub(crate) fn junit_times(xml: &str) -> Vec<TestTime> {
    let mut times = Vec::new();
    for case in xml.split("<testcase ").skip(1) {
        // The leading space lets the first attribute match like the others.
        let tag = format!(" {}", &case[..case.find('>').unwrap_or(case.len())]);
        let body = &case[..case.find("</testcase>").unwrap_or(case.len())];
        let (Some(name), Some(class)) = (
            xml_attribute(&tag, "name"),
            xml_attribute(&tag, "classname"),
        ) else {
            continue;
        };
        times.push(TestTime {
            name: format!("{class} {name}"),
            seconds: xml_attribute(&tag, "time")
                .and_then(|time| time.parse().ok())
                .unwrap_or(f64::NAN),
            timed_out: body.contains("<failure type=\"test timeout\""),
            failed: body.contains("<failure")
                || body.contains("<error")
                || body.contains("<skipped"),
            probe: false,
        });
    }
    times
}

/// Above this a test is reported as slow; ten seconds is the merge tier's bound for one test, and
/// `.config/nextest.toml` stops a test that is not exempt at the same time.
pub(crate) const SLOW_TEST_SECONDS: f64 = 10.0;

/// The serial pass's policy, chosen by the command independently of package selection.
#[derive(Clone, Copy, Default)]
pub(crate) enum RerunLimit {
    #[default]
    Merge,
    Workspace,
}

impl RerunLimit {
    pub(crate) fn profile(self) -> &'static str {
        match self {
            Self::Merge => "default",
            Self::Workspace => "workspace-rerun",
        }
    }

    pub(crate) fn seconds(self) -> f64 {
        match self {
            Self::Merge => SLOW_TEST_SECONDS,
            Self::Workspace => 30.0,
        }
    }
}

/// A test the limit stopped side by side, and its second run alone (GATE-TIME mechanism 13).
pub(crate) struct Rerun {
    pub(crate) name: String,
    pub(crate) side_by_side: f64,
    /// Its time alone, or None when the second run reported nothing for it.
    pub(crate) alone: Option<f64>,
    pub(crate) passed: bool,
    pub(crate) probe: bool,
}

/// The filter that selects exactly the tests in `stopped`, each by its binary and its whole name.
pub(crate) fn rerun_filter(stopped: &[&TestTime]) -> String {
    stopped
        .iter()
        .map(|test| {
            let (binary, name) = test.name.split_once(' ').unwrap_or((&test.name, ""));
            format!("(binary_id(={binary}) & test(={name}))")
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

/// Each stopped test's second run, from the second run's report: it passes only when the report
/// has it passed within the limit.
pub(crate) fn reruns(stopped: &[&TestTime], alone: &[TestTime], limit: RerunLimit) -> Vec<Rerun> {
    stopped
        .iter()
        .map(|test| {
            let again = alone.iter().find(|other| other.name == test.name);
            Rerun {
                name: test.name.clone(),
                side_by_side: test.seconds,
                alone: again.map(|again| again.seconds),
                passed: again.is_some_and(|again| {
                    !again.failed && (0.0..=limit.seconds()).contains(&again.seconds)
                }),
                probe: test.probe,
            }
        })
        .collect()
}

/// Whether a pass of cargo-nextest that exited with a failure passes after its second runs: only
/// when every test that failed was stopped by the limit and every one of them passed alone. A test
/// that fails any other way, or a failure with no failed test such as a build error, still fails.
pub(crate) fn passes_after_reruns(
    times: &[TestTime],
    reruns: &[Rerun],
    retry_succeeded: bool,
) -> bool {
    let stopped = times.iter().filter(|test| test.timed_out).count();
    retry_succeeded
        && stopped > 0
        && stopped == times.iter().filter(|test| test.failed).count()
        && reruns.len() == stopped
        && reruns.iter().all(|rerun| rerun.passed)
}

/// `args` without any `-E <filter>`, for a second run that selects its own tests.
pub(crate) fn strip_filters<'a>(args: &[&'a str]) -> Vec<&'a str> {
    let mut kept = Vec::new();
    let mut rest = args.iter();
    while let Some(&arg) = rest.next() {
        if arg == "-E" {
            rest.next();
        } else if arg != "--no-tests=pass" {
            kept.push(arg);
        }
    }
    kept
}

/// Runs nextest and reads its profile's JUnit report. Only exit 100 (failed tests) permits
/// recovery from the report. Build, configuration and runner errors propagate directly.
pub(crate) fn run_nextest(
    store: &Path,
    profile: RerunLimit,
    args: &[&str],
) -> Result<(TaskResult, String), Box<dyn Error>> {
    let junit = store.join(profile.profile()).join("junit.xml");
    // A report left by an earlier run must not be read as this one's.
    match std::fs::remove_file(&junit) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error.into()),
        _ => {}
    }
    let mut full = vec![
        "nextest",
        "run",
        "--locked",
        "--offline",
        "--profile",
        profile.profile(),
        "--retries",
        "0",
        "--no-fail-fast",
    ];
    full.extend_from_slice(args);
    let status = Command::new(env!("CARGO")).args(&full).status()?;
    let ran = if status.success() {
        Ok(())
    } else {
        let error = format!("cargo {} failed ({status})", full.join(" "));
        if status.code() != Some(100) {
            return Err(error.into());
        }
        Err(error.into())
    };
    let xml = std::fs::read_to_string(&junit).map_err(|error| {
        format!(
            "cargo-nextest wrote no readable JUnit report at {} ({error}); \
             .config/nextest.toml's [store] dir must be {NEXTEST_STORE}",
            junit.display()
        )
    })?;
    Ok((ran, xml))
}

/// The closing lines on the tests the limit stopped side by side: each with its time there and its
/// time alone on its second run. A test that failed any other way was not run again.
pub(crate) fn rerun_lines(reruns: &[Rerun], limit: RerunLimit) -> Vec<String> {
    if reruns.is_empty() {
        return vec![format!(
            "no test was stopped at {SLOW_TEST_SECONDS} s side by side, so none was run again alone"
        )];
    }
    let mut lines = vec![format!(
        "{} test(s) stopped at {SLOW_TEST_SECONDS} s side by side and run again alone, after the others; \
         each fails the check unless it passed alone within {} s:",
        reruns.len(),
        limit.seconds()
    )];
    for rerun in reruns {
        let alone = rerun.alone.map_or_else(
            || "no result".to_owned(),
            |seconds| format!("{seconds:.3} s"),
        );
        let verdict = if rerun.passed { "passed" } else { "FAILED" };
        let pass = if rerun.probe {
            " (draw-probe pass)"
        } else {
            ""
        };
        lines.push(format!(
            "  {:>6.3} s side by side, {alone} alone, {verdict}  {}{pass}",
            rerun.side_by_side, rerun.name
        ));
        if rerun.passed
            && rerun
                .alone
                .is_some_and(|seconds| seconds > SLOW_TEST_SECONDS)
        {
            lines.push(format!(
                "    recovery above the {SLOW_TEST_SECONDS} s fast-test target: {}{pass}",
                rerun.name
            ));
        }
    }
    lines
}

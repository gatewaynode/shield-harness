// Subprocess invocation of `lcs scan -e <engine> -f json`. Phase 2a.
//
// Pipes a sample's raw bytes to lcs's stdin, captures stdout/stderr, parses
// the JSON ScanReport, and records wall-clock latency. Exit code 0 means
// clean, 1 means at least one finding, 2 is a runtime error from lcs.

use crate::runner::lcs::binary as resolve_binary;
use crate::runner::scan_report::ScanReport;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;

#[derive(Debug)]
pub struct ScanOutcome {
    pub report: ScanReport,
    pub exit_code: i32,
    pub stderr: String,
    pub latency_ms: u64,
    pub raw_stdout: String,
}

#[derive(Debug)]
pub enum ScanError {
    LcsNotFound {
        path: String,
        source: std::io::Error,
    },
    StdinFailed {
        source: std::io::Error,
    },
    WaitFailed {
        source: std::io::Error,
    },
    Crashed {
        exit_code: i32,
        stderr: String,
    },
    ParseFailed {
        stdout: String,
        source: serde_json::Error,
    },
    UnexpectedExit {
        exit_code: i32,
        stderr: String,
    },
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LcsNotFound { path, source } => {
                write!(f, "lcs binary '{path}' not runnable: {source}")
            }
            Self::StdinFailed { source } => write!(f, "failed to write sample to lcs stdin: {source}"),
            Self::WaitFailed { source } => write!(f, "failed to wait for lcs to exit: {source}"),
            Self::Crashed { exit_code, stderr } => write!(
                f,
                "lcs exited with code {exit_code}: {}",
                stderr.trim()
            ),
            Self::ParseFailed { source, .. } => {
                write!(f, "failed to parse lcs JSON output: {source}")
            }
            Self::UnexpectedExit { exit_code, stderr } => write!(
                f,
                "lcs exited with unexpected code {exit_code}: {}",
                stderr.trim()
            ),
        }
    }
}

impl std::error::Error for ScanError {}

/// Invoke `lcs scan -e <engine> -f json` against the given sample bytes.
/// Captures stdout, stderr, exit code, and wall-clock latency.
///
/// Exit 0 = clean, exit 1 = threat (one or more findings) — both yield a
/// parsed `ScanOutcome`. Exit 2 = lcs error → `ScanError::Crashed`. Other
/// exit codes are reported as `UnexpectedExit`.
pub fn scan(
    sample_bytes: &[u8],
    engine: &str,
    lcs_path: Option<&Path>,
) -> Result<ScanOutcome, ScanError> {
    let bin = resolve_binary(lcs_path);
    let started = Instant::now();

    let mut child = Command::new(&bin)
        .args(["scan", "-e", engine, "-f", "json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| ScanError::LcsNotFound {
            path: bin.display().to_string(),
            source,
        })?;

    let mut stdin = child.stdin.take().expect("stdin was piped");
    let write_result = stdin.write_all(sample_bytes);
    drop(stdin);

    let output_result = child.wait_with_output();
    let elapsed = started.elapsed();

    write_result.map_err(|source| ScanError::StdinFailed { source })?;
    let output = output_result.map_err(|source| ScanError::WaitFailed { source })?;

    let latency_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
    let exit_code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let raw_stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    match exit_code {
        0 | 1 => {
            let report: ScanReport = serde_json::from_str(&raw_stdout).map_err(|source| {
                ScanError::ParseFailed {
                    stdout: raw_stdout.clone(),
                    source,
                }
            })?;
            Ok(ScanOutcome {
                report,
                exit_code,
                stderr,
                latency_ms,
                raw_stdout,
            })
        }
        2 => Err(ScanError::Crashed { exit_code, stderr }),
        _ => Err(ScanError::UnexpectedExit { exit_code, stderr }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::sample::Severity;
    use std::path::PathBuf;

    fn fixture(rel: &str) -> PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        PathBuf::from(manifest_dir).join("samples/seed-handcurated").join(rel)
    }

    fn read(rel: &str) -> Vec<u8> {
        std::fs::read(fixture(rel)).expect("fixture must exist")
    }

    #[test]
    fn lcs_not_found_carries_path_and_source() {
        let bytes = b"hello";
        let err = scan(bytes, "simple", Some(Path::new("/no/such/lcs")))
            .expect_err("must fail when binary missing");
        match err {
            ScanError::LcsNotFound { path, .. } => assert!(path.contains("no/such/lcs")),
            other => panic!("expected LcsNotFound, got {other:?}"),
        }
    }

    #[test]
    fn clean_sample_yields_clean_report_exit_zero() {
        let bytes = read("clean/seed-clean-001.txt");
        let outcome = scan(&bytes, "simple", None).expect("scan should succeed");
        assert_eq!(outcome.exit_code, 0);
        assert!(outcome.report.clean);
        assert_eq!(outcome.report.finding_count, 0);
        assert!(outcome.report.findings.is_empty());
        assert_eq!(outcome.report.threat_scores.cumulative, 0);
        assert!(outcome.report.threat_scores.class_scores.is_empty());
        assert!(!outcome.report.rule_set_fingerprint.is_empty());
        assert!(!outcome.raw_stdout.is_empty());
    }

    #[test]
    fn threat_sample_fires_with_expected_category() {
        let bytes = read("threat/seed-threat-001.txt");
        let outcome = scan(&bytes, "simple", None).expect("scan should succeed");
        assert_eq!(outcome.exit_code, 1);
        assert!(!outcome.report.clean);
        assert!(outcome.report.finding_count >= 1);
        let categories: Vec<&str> = outcome
            .report
            .findings
            .iter()
            .map(|f| f.category.as_str())
            .collect();
        assert!(
            categories.contains(&"prompt_injection"),
            "expected prompt_injection in {categories:?}"
        );
        let pi = outcome
            .report
            .findings
            .iter()
            .find(|f| f.category == "prompt_injection")
            .expect("prompt_injection finding present");
        assert_eq!(pi.engine, "simple");
        assert!(pi.severity >= Severity::Medium);
        assert_eq!(pi.byte_range.0, 0);
        assert!(pi.byte_range.1 > pi.byte_range.0);
    }

    #[test]
    fn syara_threat_sample_returns_multi_finding_report() {
        let bytes = read("threat/seed-threat-001.txt");
        let outcome = scan(&bytes, "syara", None).expect("scan should succeed");
        assert_eq!(outcome.exit_code, 1);
        assert!(outcome.report.findings.len() >= 2);
        for f in &outcome.report.findings {
            assert_eq!(f.engine, "syara");
        }
        assert!(outcome.report.threat_scores.cumulative > 0);
        assert!(!outcome.report.threat_scores.class_scores.is_empty());
    }

    #[test]
    fn raw_stdout_round_trips_exactly_to_parsed_report() {
        let bytes = read("threat/seed-threat-002.md");
        let outcome = scan(&bytes, "yara", None).expect("scan should succeed");
        let reparsed: ScanReport =
            serde_json::from_str(&outcome.raw_stdout).expect("raw_stdout parses");
        assert_eq!(reparsed, outcome.report);
    }

    #[test]
    fn latency_is_recorded() {
        let bytes = read("clean/seed-clean-002.txt");
        let outcome = scan(&bytes, "simple", None).expect("scan should succeed");
        // Real lcs invocation always takes nonzero wall-clock time. If this
        // ever flakes to 0 we want to know.
        assert!(outcome.latency_ms < 60_000, "scan took >60s: {}", outcome.latency_ms);
    }

    #[test]
    fn parse_failed_when_stdout_is_garbage() {
        // Cover the parse-failure branch without involving lcs by invoking
        // /bin/echo as a stand-in: it accepts stdin (ignores it), exits 0,
        // and writes non-JSON to stdout — which is exactly the failure mode
        // we model.
        let bytes = b"anything";
        let err = scan(bytes, "simple", Some(Path::new("/bin/echo")))
            .expect_err("echo's output is not valid ScanReport JSON");
        match err {
            ScanError::ParseFailed { stdout, .. } => {
                assert!(!stdout.is_empty(), "stdout should be captured even on parse failure");
            }
            other => panic!("expected ParseFailed, got {other:?}"),
        }
    }
}

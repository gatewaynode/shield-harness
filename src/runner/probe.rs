// Engine availability probe via `lcs scan -e <engine> -f quiet`. Phase 2b.
//
// Implements the ARCH §5 state machine: each requested engine is exercised
// with a tiny constant input. Exit 0 (clean) or 1 (threats) means the engine
// works. Exit 2 (lcs error) means skip; stderr is classified into a
// `SkipKind` and preserved verbatim in `skip_reason` for run.json.

use crate::runner::lcs::binary as resolve_binary;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const PROBE_INPUT: &[u8] = b"hi";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineStatus {
    pub name: String,
    pub state: Availability,
    pub skip_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Available,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipKind {
    FeatureMissing,
    OnnxRuntimeMissing,
    LmstudioUnreachable,
    Other,
}

impl std::fmt::Display for SkipKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::FeatureMissing => "feature_missing",
            Self::OnnxRuntimeMissing => "onnx_runtime_missing",
            Self::LmstudioUnreachable => "lmstudio_unreachable",
            Self::Other => "other",
        };
        f.write_str(label)
    }
}

#[derive(Debug)]
pub enum ProbeError {
    LcsNotFound {
        path: String,
        source: std::io::Error,
    },
    WaitFailed {
        engine: String,
        source: std::io::Error,
    },
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LcsNotFound { path, source } => {
                write!(f, "lcs binary '{path}' not runnable: {source}")
            }
            Self::WaitFailed { engine, source } => {
                write!(f, "failed to wait for lcs probe of engine '{engine}': {source}")
            }
        }
    }
}

impl std::error::Error for ProbeError {}

/// Probe each requested engine in order. Returns one `EngineStatus` per
/// requested engine, preserving order. Errors only when lcs cannot be spawned
/// at all; per-engine failures (exit 2, unknown engine, etc.) become
/// `EngineStatus { state: Skipped, skip_reason: Some(...) }`.
pub fn probe_engines(
    requested: &[&str],
    lcs_path: Option<&Path>,
) -> Result<Vec<EngineStatus>, ProbeError> {
    let bin = resolve_binary(lcs_path);
    let mut out = Vec::with_capacity(requested.len());
    for engine in requested {
        out.push(probe_one(&bin, engine)?);
    }
    Ok(out)
}

fn probe_one(bin: &Path, engine: &str) -> Result<EngineStatus, ProbeError> {
    let mut child = Command::new(bin)
        .args(["scan", "-e", engine, "-f", "quiet"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| ProbeError::LcsNotFound {
            path: bin.display().to_string(),
            source,
        })?;

    let mut stdin = child.stdin.take().expect("stdin was piped");
    // Best-effort: an engine that errors before reading stdin will close the
    // pipe, and write_all will fail. The exit code + stderr is what tells us
    // the engine state — we don't elevate stdin errors.
    let _ = stdin.write_all(PROBE_INPUT);
    drop(stdin);

    let output = child
        .wait_with_output()
        .map_err(|source| ProbeError::WaitFailed {
            engine: engine.to_string(),
            source,
        })?;

    let exit_code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    match exit_code {
        0 | 1 => Ok(EngineStatus {
            name: engine.to_string(),
            state: Availability::Available,
            skip_reason: None,
        }),
        _ => Ok(EngineStatus {
            name: engine.to_string(),
            state: Availability::Skipped,
            skip_reason: Some(format_skip_reason(&stderr)),
        }),
    }
}

/// Classify lcs stderr into a structured skip kind, then format it as a
/// `<kind>: <first non-empty stderr line>` string for run.json.
fn format_skip_reason(stderr: &str) -> String {
    let kind = classify_stderr(stderr);
    let head = first_nonempty_line(stderr);
    if head.is_empty() {
        format!("{kind}: lcs exited with error code; stderr was empty")
    } else {
        format!("{kind}: {head}")
    }
}

fn first_nonempty_line(s: &str) -> &str {
    s.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("")
}

/// Classify lcs stderr into one of the documented skip categories.
/// Substring-matched against lowercased stderr; falls back to `Other` so
/// classification is never lossy — the verbatim stderr is preserved in
/// `EngineStatus.skip_reason`.
pub fn classify_stderr(stderr: &str) -> SkipKind {
    let lower = stderr.to_lowercase();
    if lower.contains("onnx") {
        SkipKind::OnnxRuntimeMissing
    } else if lower.contains("lmstudio")
        || lower.contains("localhost:1234")
        || lower.contains("connection refused")
    {
        SkipKind::LmstudioUnreachable
    } else if lower.contains("not compiled")
        || lower.contains("compiled without")
        || (lower.contains("feature")
            && (lower.contains("not enabled")
                || lower.contains("disabled")
                || lower.contains("requires")))
    {
        SkipKind::FeatureMissing
    } else {
        SkipKind::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- pure unit tests for the stderr classifier -----------------------

    #[test]
    fn classify_onnx_runtime_missing() {
        let stderr = "Error: ONNX runtime not found on this system";
        assert_eq!(classify_stderr(stderr), SkipKind::OnnxRuntimeMissing);
        let stderr = "failed to load onnxruntime.dylib";
        assert_eq!(classify_stderr(stderr), SkipKind::OnnxRuntimeMissing);
    }

    #[test]
    fn classify_lmstudio_unreachable() {
        let stderr = "could not reach LMStudio at http://localhost:1234/v1";
        assert_eq!(classify_stderr(stderr), SkipKind::LmstudioUnreachable);
        let stderr = "Error: connection refused (os error 61)";
        assert_eq!(classify_stderr(stderr), SkipKind::LmstudioUnreachable);
    }

    #[test]
    fn classify_feature_missing() {
        let stderr = "syara feature not enabled in this build";
        assert_eq!(classify_stderr(stderr), SkipKind::FeatureMissing);
        let stderr = "engine 'yara' was not compiled with this binary";
        assert_eq!(classify_stderr(stderr), SkipKind::FeatureMissing);
        let stderr = "this engine requires the 'syara-llm' feature";
        assert_eq!(classify_stderr(stderr), SkipKind::FeatureMissing);
    }

    #[test]
    fn classify_other_for_unrecognized_stderr() {
        assert_eq!(classify_stderr(""), SkipKind::Other);
        assert_eq!(classify_stderr("unknown engine 'bogus'"), SkipKind::Other);
        assert_eq!(classify_stderr("permission denied"), SkipKind::Other);
    }

    #[test]
    fn classify_priority_onnx_beats_feature() {
        // "feature ... not enabled" + "ONNX" both match — ONNX wins because
        // the more-specific cause (missing runtime) is more actionable than
        // the generic feature-flag framing.
        let stderr = "syara-sbert feature not enabled (ONNX runtime missing)";
        assert_eq!(classify_stderr(stderr), SkipKind::OnnxRuntimeMissing);
    }

    #[test]
    fn format_skip_reason_includes_kind_and_first_line() {
        let stderr = "Error: ONNX runtime not found\n  at /opt/lcs/lib\n";
        let reason = format_skip_reason(stderr);
        assert!(reason.starts_with("onnx_runtime_missing: "));
        assert!(reason.contains("ONNX runtime not found"));
        assert!(!reason.contains("at /opt/lcs/lib"), "only first line surfaced");
    }

    #[test]
    fn format_skip_reason_handles_empty_stderr() {
        let reason = format_skip_reason("");
        assert_eq!(reason, "other: lcs exited with error code; stderr was empty");
    }

    #[test]
    fn format_skip_reason_skips_blank_leading_lines() {
        let stderr = "\n   \nactual error here\n";
        let reason = format_skip_reason(stderr);
        assert!(reason.ends_with("actual error here"));
    }

    // --- live integration tests against real lcs 0.5.3 -------------------

    #[test]
    fn lcs_not_found_carries_path_and_source() {
        let err = probe_engines(&["simple"], Some(Path::new("/no/such/lcs")))
            .expect_err("must fail when binary missing");
        match err {
            ProbeError::LcsNotFound { path, .. } => {
                assert!(path.contains("no/such/lcs"));
            }
            other => panic!("expected LcsNotFound, got {other:?}"),
        }
    }

    #[test]
    fn simple_engine_is_available_against_real_lcs() {
        let statuses = probe_engines(&["simple"], None).expect("probe must succeed");
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].name, "simple");
        assert_eq!(statuses[0].state, Availability::Available);
        assert!(statuses[0].skip_reason.is_none());
    }

    #[test]
    fn yara_engine_is_available_against_real_lcs() {
        let statuses = probe_engines(&["yara"], None).expect("probe must succeed");
        assert_eq!(statuses[0].state, Availability::Available);
    }

    #[test]
    fn syara_engine_is_available_against_real_lcs() {
        let statuses = probe_engines(&["syara"], None).expect("probe must succeed");
        assert_eq!(statuses[0].state, Availability::Available);
    }

    #[test]
    fn three_engines_probed_in_order() {
        let statuses = probe_engines(&["simple", "yara", "syara"], None)
            .expect("probe must succeed");
        assert_eq!(statuses.len(), 3);
        assert_eq!(statuses[0].name, "simple");
        assert_eq!(statuses[1].name, "yara");
        assert_eq!(statuses[2].name, "syara");
        for s in &statuses {
            assert_eq!(s.state, Availability::Available, "engine {} skipped: {:?}", s.name, s.skip_reason);
        }
    }

    #[test]
    fn unknown_engine_yields_skipped_with_reason() {
        // lcs rejects unknown engine names with a non-zero exit; we model
        // that as a Skipped status carrying the stderr explanation. This
        // pins the contract: an unknown engine never aborts a probe pass.
        let statuses = probe_engines(&["definitely-not-an-engine"], None)
            .expect("probe must not error on bad engine name");
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].state, Availability::Skipped);
        let reason = statuses[0]
            .skip_reason
            .as_ref()
            .expect("skipped status must carry reason");
        assert!(!reason.is_empty(), "skip_reason must not be empty");
    }

    #[test]
    fn empty_request_returns_empty_status_vec() {
        let statuses = probe_engines(&[], None).expect("empty probe is a no-op");
        assert!(statuses.is_empty());
    }
}

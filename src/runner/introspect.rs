// Lcs introspection helpers. Wraps the `lcs rules` subcommand introduced in lcs 0.5.2.

use std::path::Path;
use std::process::Command;

#[derive(Debug)]
pub enum ProbeError {
    LcsNotFound {
        path: String,
        source: std::io::Error,
    },
    EngineUnavailable {
        engine: String,
        stderr: String,
    },
    ParseFailed {
        engine: String,
        output: String,
    },
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LcsNotFound { path, source } => {
                write!(f, "lcs binary '{path}' not runnable: {source}")
            }
            Self::EngineUnavailable { engine, stderr } => {
                write!(
                    f,
                    "lcs engine '{engine}' unavailable: {}",
                    stderr.trim()
                )
            }
            Self::ParseFailed { engine, output } => write!(
                f,
                "lcs rules --categories -e {engine} produced no categories; output was: {output:?}"
            ),
        }
    }
}

impl std::error::Error for ProbeError {}

fn binary(override_path: Option<&Path>) -> String {
    override_path
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "lcs".to_string())
}

/// Probe the category vocabulary the given engine can emit.
/// Wraps `lcs rules --categories -e <engine>` (lcs >= 0.5.2).
pub fn probe_categories(
    lcs_path: Option<&Path>,
    engine: &str,
) -> Result<Vec<String>, ProbeError> {
    let bin = binary(lcs_path);
    let output = Command::new(&bin)
        .args(["rules", "--categories", "-e", engine])
        .output()
        .map_err(|source| ProbeError::LcsNotFound {
            path: bin.clone(),
            source,
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(ProbeError::EngineUnavailable {
            engine: engine.to_string(),
            stderr,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let categories = parse_category_lines(&stdout);
    if categories.is_empty() {
        return Err(ProbeError::ParseFailed {
            engine: engine.to_string(),
            output: stdout.into_owned(),
        });
    }
    Ok(categories)
}

fn parse_category_lines(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_one_category_per_line() {
        let raw = "prompt_injection\nhidden_content\ndata_exfiltration\n";
        assert_eq!(
            parse_category_lines(raw),
            vec!["prompt_injection", "hidden_content", "data_exfiltration"]
        );
    }

    #[test]
    fn ignores_blank_lines_and_trims_whitespace() {
        let raw = "\nprompt_injection\n   \n  hidden_content  \n\n";
        assert_eq!(
            parse_category_lines(raw),
            vec!["prompt_injection", "hidden_content"]
        );
    }

    #[test]
    fn empty_output_parses_to_empty_vec() {
        assert!(parse_category_lines("").is_empty());
        assert!(parse_category_lines("   \n\n  \n").is_empty());
    }

    #[test]
    fn lcs_not_found_error_carries_path_and_source() {
        let err = probe_categories(Some(Path::new("/definitely/no/such/lcs")), "simple")
            .expect_err("must fail when binary missing");
        match err {
            ProbeError::LcsNotFound { path, .. } => assert!(path.contains("definitely")),
            other => panic!("expected LcsNotFound, got {other:?}"),
        }
    }
}

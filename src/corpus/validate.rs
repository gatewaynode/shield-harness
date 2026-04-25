use crate::cli::{Common, ValidateArgs};
use crate::corpus::loader::load_corpus;
use crate::corpus::sample::{Sample, Verdict};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::process::ExitCode;

const LICENSE_ALLOWLIST: &[&str] = &[
    "MIT",
    "Apache-2.0",
    "BSD-*",
    "CC-BY-*",
    "CC0",
    "internal",
    "synthetic",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueKind {
    CohortDirMismatch {
        dir_name: String,
        sidecar_value: String,
    },
    VerdictDirMismatch {
        dir_name: String,
        sidecar_value: String,
    },
    TextFileMissing {
        path: PathBuf,
    },
    DuplicateId {
        first_seen_at: PathBuf,
    },
    ThreatMissingCategories,
    LicenseEmpty,
    LicenseDisallowed {
        value: String,
    },
    /// Emitted when --check-lcs-categories is passed; the real check requires the
    /// Phase 2 engine probe and lcs `list -e <eng>` invocation. Non-blocking notice.
    LcsCategoryCheckPending,
}

#[derive(Debug, Clone)]
pub struct Issue {
    pub sample_id: String,
    pub sidecar_path: PathBuf,
    pub kind: IssueKind,
}

impl Issue {
    pub fn is_blocking(&self) -> bool {
        !matches!(self.kind, IssueKind::LcsCategoryCheckPending)
    }
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.sidecar_path.as_os_str().is_empty() {
            write!(f, "{}: ", self.sidecar_path.display())?;
        }
        if !self.sample_id.is_empty() {
            write!(f, "[{}] ", self.sample_id)?;
        }
        match &self.kind {
            IssueKind::CohortDirMismatch {
                dir_name,
                sidecar_value,
            } => write!(
                f,
                "cohort directory '{dir_name}' does not match sidecar cohort '{sidecar_value}'"
            ),
            IssueKind::VerdictDirMismatch {
                dir_name,
                sidecar_value,
            } => write!(
                f,
                "verdict directory '{dir_name}' does not match sidecar verdict '{sidecar_value}'"
            ),
            IssueKind::TextFileMissing { path } => {
                write!(f, "text file missing: {}", path.display())
            }
            IssueKind::DuplicateId { first_seen_at } => write!(
                f,
                "duplicate id; first seen at {}",
                first_seen_at.display()
            ),
            IssueKind::ThreatMissingCategories => {
                write!(f, "threat sample has no expected_categories")
            }
            IssueKind::LicenseEmpty => write!(f, "license field is empty"),
            IssueKind::LicenseDisallowed { value } => {
                write!(f, "license '{value}' is not in the allow-list")
            }
            IssueKind::LcsCategoryCheckPending => write!(
                f,
                "notice: --check-lcs-categories is acknowledged but the lcs vocabulary probe lands in Phase 2"
            ),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub check_lcs_categories: bool,
}

pub fn validate(samples: &[Sample], opts: &Options) -> Vec<Issue> {
    let mut issues: Vec<Issue> = Vec::new();

    if opts.check_lcs_categories {
        issues.push(Issue {
            sample_id: String::new(),
            sidecar_path: PathBuf::new(),
            kind: IssueKind::LcsCategoryCheckPending,
        });
    }

    let mut first_seen: HashMap<&str, &Sample> = HashMap::new();
    for sample in samples {
        if let Some(prior) = first_seen.get(sample.sidecar.id.as_str()) {
            issues.push(Issue {
                sample_id: sample.sidecar.id.clone(),
                sidecar_path: sample.sidecar_path.clone(),
                kind: IssueKind::DuplicateId {
                    first_seen_at: prior.sidecar_path.clone(),
                },
            });
        } else {
            first_seen.insert(sample.sidecar.id.as_str(), sample);
        }
    }

    for sample in samples {
        let sid = &sample.sidecar;
        let sp = &sample.sidecar_path;
        let mut push = |kind: IssueKind| {
            issues.push(Issue {
                sample_id: sid.id.clone(),
                sidecar_path: sp.clone(),
                kind,
            });
        };

        let verdict_dir = sp
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str());
        let cohort_dir = sp
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str());

        let expected_verdict_str = match sid.verdict {
            Verdict::Clean => "clean",
            Verdict::Threat => "threat",
        };
        if let Some(vd) = verdict_dir {
            if vd != expected_verdict_str {
                push(IssueKind::VerdictDirMismatch {
                    dir_name: vd.to_string(),
                    sidecar_value: expected_verdict_str.to_string(),
                });
            }
        }
        if let Some(cd) = cohort_dir {
            if cd != sid.cohort {
                push(IssueKind::CohortDirMismatch {
                    dir_name: cd.to_string(),
                    sidecar_value: sid.cohort.clone(),
                });
            }
        }

        if !sample.text_full_path.is_file() {
            push(IssueKind::TextFileMissing {
                path: sample.text_full_path.clone(),
            });
        }

        if sid.verdict == Verdict::Threat && sid.expected_categories.is_empty() {
            push(IssueKind::ThreatMissingCategories);
        }

        if sid.license.is_empty() {
            push(IssueKind::LicenseEmpty);
        } else if !license_allowed(&sid.license) {
            push(IssueKind::LicenseDisallowed {
                value: sid.license.clone(),
            });
        }
    }

    issues
}

fn license_allowed(value: &str) -> bool {
    LICENSE_ALLOWLIST.iter().any(|pattern| {
        if let Some(prefix) = pattern.strip_suffix('*') {
            value.starts_with(prefix)
        } else {
            *pattern == value
        }
    })
}

pub fn run(common: Common, args: ValidateArgs) -> ExitCode {
    let samples = match load_corpus(&common.samples_dir) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("validate: load failed: {e}");
            return ExitCode::from(2);
        }
    };

    let opts = Options {
        check_lcs_categories: args.check_lcs_categories,
    };
    let issues = validate(&samples, &opts);

    let blocking_count = issues.iter().filter(|i| i.is_blocking()).count();
    let notice_count = issues.len() - blocking_count;

    for issue in &issues {
        if issue.is_blocking() {
            eprintln!("{issue}");
        } else {
            println!("{issue}");
        }
    }

    if blocking_count == 0 {
        println!(
            "validate: ok — {} sample(s){}",
            samples.len(),
            if notice_count > 0 {
                format!(", {notice_count} notice(s)")
            } else {
                String::new()
            }
        );
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "validate: {blocking_count} issue(s) across {} sample(s)",
            samples.len()
        );
        ExitCode::from(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/validate-cases")
            .join(name)
    }

    fn load(name: &str) -> Vec<Sample> {
        load_corpus(&fixture(name)).expect("load fixture")
    }

    #[test]
    fn happy_path_yields_no_issues() {
        let issues = validate(&load("happy"), &Options::default());
        assert!(issues.is_empty(), "got: {issues:?}");
    }

    #[test]
    fn cohort_dir_mismatch_is_caught() {
        let issues = validate(&load("cohort-dir-mismatch"), &Options::default());
        assert_eq!(issues.len(), 1, "issues: {issues:?}");
        match &issues[0].kind {
            IssueKind::CohortDirMismatch {
                dir_name,
                sidecar_value,
            } => {
                assert_eq!(dir_name, "wrong-name");
                assert_eq!(sidecar_value, "the-real-cohort-name");
            }
            other => panic!("unexpected kind: {other:?}"),
        }
        assert!(issues[0].is_blocking());
    }

    #[test]
    fn verdict_dir_mismatch_is_caught() {
        let issues = validate(&load("verdict-dir-mismatch"), &Options::default());
        assert_eq!(issues.len(), 1, "issues: {issues:?}");
        match &issues[0].kind {
            IssueKind::VerdictDirMismatch {
                dir_name,
                sidecar_value,
            } => {
                assert_eq!(dir_name, "clean");
                assert_eq!(sidecar_value, "threat");
            }
            other => panic!("unexpected kind: {other:?}"),
        }
    }

    #[test]
    fn missing_text_file_is_caught() {
        let issues = validate(&load("missing-text-file"), &Options::default());
        assert_eq!(issues.len(), 1, "issues: {issues:?}");
        match &issues[0].kind {
            IssueKind::TextFileMissing { path } => {
                assert!(
                    path.ends_with("0001.txt"),
                    "unexpected path: {}",
                    path.display()
                );
            }
            other => panic!("unexpected kind: {other:?}"),
        }
    }

    #[test]
    fn duplicate_id_across_cohorts_is_caught() {
        let issues = validate(&load("duplicate-id"), &Options::default());
        assert_eq!(issues.len(), 1, "issues: {issues:?}");
        let issue = &issues[0];
        assert_eq!(issue.sample_id, "vDUP");
        match &issue.kind {
            IssueKind::DuplicateId { first_seen_at } => {
                // Sorted by (cohort, id), so cohort-x is the first occurrence and
                // cohort-y is the duplicate that gets flagged.
                assert!(
                    first_seen_at
                        .to_string_lossy()
                        .contains("cohort-x/clean/0001.toml"),
                    "first_seen_at: {}",
                    first_seen_at.display()
                );
                assert!(
                    issue
                        .sidecar_path
                        .to_string_lossy()
                        .contains("cohort-y/clean/0001.toml"),
                    "duplicate at: {}",
                    issue.sidecar_path.display()
                );
            }
            other => panic!("unexpected kind: {other:?}"),
        }
    }

    #[test]
    fn threat_without_categories_is_caught() {
        let issues = validate(&load("threat-without-categories"), &Options::default());
        assert_eq!(issues.len(), 1, "issues: {issues:?}");
        assert!(matches!(issues[0].kind, IssueKind::ThreatMissingCategories));
    }

    #[test]
    fn empty_license_is_caught() {
        let issues = validate(&load("license-empty"), &Options::default());
        assert_eq!(issues.len(), 1, "issues: {issues:?}");
        assert!(matches!(issues[0].kind, IssueKind::LicenseEmpty));
    }

    #[test]
    fn disallowed_license_is_caught() {
        let issues = validate(&load("license-disallowed"), &Options::default());
        assert_eq!(issues.len(), 1, "issues: {issues:?}");
        match &issues[0].kind {
            IssueKind::LicenseDisallowed { value } => assert_eq!(value, "GPL-3.0-only"),
            other => panic!("unexpected kind: {other:?}"),
        }
    }

    #[test]
    fn glob_pattern_licenses_are_accepted() {
        let issues = validate(&load("license-glob-ok"), &Options::default());
        assert!(issues.is_empty(), "got: {issues:?}");
    }

    #[test]
    fn license_allowlist_matches() {
        // Exact matches.
        for ok in ["MIT", "Apache-2.0", "CC0", "internal", "synthetic"] {
            assert!(license_allowed(ok), "{ok} should be allowed");
        }
        // Glob matches.
        for ok in ["BSD-2-Clause", "BSD-3-Clause", "CC-BY-4.0", "CC-BY-SA-4.0"] {
            assert!(license_allowed(ok), "{ok} should be allowed (glob)");
        }
        // Rejections.
        for bad in ["GPL-3.0-only", "AGPL-3.0", "Proprietary", "", "bsd-3-clause"] {
            assert!(!license_allowed(bad), "{bad} should be rejected");
        }
    }

    #[test]
    fn lcs_category_flag_emits_non_blocking_notice() {
        let opts = Options {
            check_lcs_categories: true,
        };
        let issues = validate(&load("happy"), &opts);
        assert_eq!(issues.len(), 1, "issues: {issues:?}");
        let issue = &issues[0];
        assert!(matches!(issue.kind, IssueKind::LcsCategoryCheckPending));
        assert!(!issue.is_blocking());
    }

    #[test]
    fn issue_display_includes_path_and_id() {
        let issue = Issue {
            sample_id: "v0042".to_string(),
            sidecar_path: PathBuf::from("/tmp/foo/clean/0042.toml"),
            kind: IssueKind::ThreatMissingCategories,
        };
        let s = issue.to_string();
        assert!(s.contains("/tmp/foo/clean/0042.toml"), "{s}");
        assert!(s.contains("v0042"), "{s}");
        assert!(s.contains("expected_categories"), "{s}");
    }

    #[test]
    fn pending_notice_display_omits_empty_fields() {
        let issue = Issue {
            sample_id: String::new(),
            sidecar_path: PathBuf::new(),
            kind: IssueKind::LcsCategoryCheckPending,
        };
        let s = issue.to_string();
        assert!(!s.starts_with(": "), "{s}");
        assert!(!s.contains("[]"), "{s}");
        assert!(s.contains("Phase 2"), "{s}");
    }

    #[test]
    fn rejects_unknown_corpus_root() {
        // load_corpus errors propagate cleanly through run(); validate() itself doesn't
        // touch the filesystem beyond text file existence, so this guards the wiring.
        assert!(load_corpus(Path::new("/nope/nope/nope")).is_err());
    }
}

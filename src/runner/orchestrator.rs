// Run orchestrator. Phase 2c.
//
// `execute(common, args)` is the entry point: load the corpus, apply cohort
// filters, decide on engines, probe them, build the work matrix, dispatch
// scans in parallel via rayon, and return an in-memory RunRecord. Phase 2d
// will persist that record to disk; for now the CLI handler just prints a
// summary.

use crate::cli::{Common, InspectArgs, RunArgs};
use crate::corpus::loader::load_corpus;
use crate::corpus::sample::Sample;
use crate::runner::invoke::{ScanError, ScanOutcome, scan};
use crate::runner::probe::{Availability, EngineStatus, ProbeError, probe_engines};
use chrono::{DateTime, Utc};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rayon::ThreadPoolBuilder;
use std::path::PathBuf;
use std::process::ExitCode;

/// Canonical engines lcs exposes via `-e`. Matches validator's PROBED_ENGINES.
const DEFAULT_ENGINES: &[&str] = &["simple", "yara", "syara"];

#[derive(Debug)]
pub struct RunRecord {
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub requested_engines: Vec<String>,
    pub engine_statuses: Vec<EngineStatus>,
    pub work_results: Vec<WorkResult>,
    pub sample_count: usize,
}

#[derive(Debug)]
pub struct WorkResult {
    pub cohort: String,
    pub sample_id: String,
    pub engine: String,
    pub outcome: Result<ScanOutcome, ScanError>,
}

#[derive(Debug)]
pub enum RunError {
    LoadFailed(String),
    ReadSampleFailed { sample_id: String, source: std::io::Error },
    ProbeFailed(ProbeError),
    ThreadPoolBuildFailed(String),
    NoSamples,
    NoAvailableEngines { statuses: Vec<EngineStatus> },
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LoadFailed(e) => write!(f, "corpus load failed: {e}"),
            Self::ReadSampleFailed { sample_id, source } => {
                write!(f, "read sample '{sample_id}': {source}")
            }
            Self::ProbeFailed(e) => write!(f, "engine probe failed: {e}"),
            Self::ThreadPoolBuildFailed(e) => write!(f, "rayon thread pool build failed: {e}"),
            Self::NoSamples => write!(
                f,
                "no samples to run (corpus filter excluded everything or corpus is empty)"
            ),
            Self::NoAvailableEngines { statuses } => {
                let skipped: Vec<String> = statuses
                    .iter()
                    .filter(|s| s.state == Availability::Skipped)
                    .map(|s| {
                        format!(
                            "{}: {}",
                            s.name,
                            s.skip_reason.as_deref().unwrap_or("(no reason)")
                        )
                    })
                    .collect();
                write!(f, "no engines are available; skipped: [{}]", skipped.join("; "))
            }
        }
    }
}

impl std::error::Error for RunError {}

/// Run the full pipeline: corpus → cohort filter → engine probe →
/// work-unit dispatch → sorted RunRecord.
pub fn execute(common: &Common, args: &RunArgs) -> Result<RunRecord, RunError> {
    let started_at = Utc::now();

    let all_samples =
        load_corpus(&common.samples_dir).map_err(RunError::LoadFailed)?;
    let samples = filter_samples(all_samples, &args.cohort, &args.exclude_cohort);
    if samples.is_empty() {
        return Err(RunError::NoSamples);
    }

    let requested_engines = resolve_engine_list(&args.engines);
    let probe_request: Vec<&str> = requested_engines.iter().map(String::as_str).collect();
    let engine_statuses = probe_engines(&probe_request, common.lcs_path.as_deref())
        .map_err(RunError::ProbeFailed)?;

    let available_engines: Vec<String> = engine_statuses
        .iter()
        .filter(|s| s.state == Availability::Available)
        .map(|s| s.name.clone())
        .collect();
    if available_engines.is_empty() {
        return Err(RunError::NoAvailableEngines {
            statuses: engine_statuses,
        });
    }

    let work_units = build_work_units(&samples, &available_engines)?;
    let lcs_path = common.lcs_path.clone();
    let work_results = dispatch(work_units, args.jobs, lcs_path)?;

    let finished_at = Utc::now();
    Ok(RunRecord {
        started_at,
        finished_at,
        requested_engines,
        engine_statuses,
        sample_count: samples.len(),
        work_results,
    })
}

fn resolve_engine_list(requested: &[String]) -> Vec<String> {
    if requested.is_empty() {
        DEFAULT_ENGINES.iter().map(|s| (*s).to_string()).collect()
    } else {
        requested.to_vec()
    }
}

fn filter_samples(
    samples: Vec<Sample>,
    include: &[String],
    exclude: &[String],
) -> Vec<Sample> {
    samples
        .into_iter()
        .filter(|s| cohort_passes_filters(&s.sidecar.cohort, include, exclude))
        .collect()
}

fn cohort_passes_filters(cohort: &str, include: &[String], exclude: &[String]) -> bool {
    if !include.is_empty() && !include.iter().any(|p| pattern_matches(p, cohort)) {
        return false;
    }
    if exclude.iter().any(|p| pattern_matches(p, cohort)) {
        return false;
    }
    true
}

/// Trailing-`*` prefix glob, mirroring `corpus::validate::license_allowed`.
/// Plain strings are exact-match.
fn pattern_matches(pattern: &str, value: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix('*') {
        value.starts_with(prefix)
    } else {
        pattern == value
    }
}

struct WorkUnit {
    cohort: String,
    sample_id: String,
    engine: String,
    bytes: Vec<u8>,
}

fn build_work_units(
    samples: &[Sample],
    engines: &[String],
) -> Result<Vec<WorkUnit>, RunError> {
    let mut units = Vec::with_capacity(samples.len() * engines.len());
    for sample in samples {
        let bytes = sample.read_bytes().map_err(|source| RunError::ReadSampleFailed {
            sample_id: sample.sidecar.id.clone(),
            source,
        })?;
        for engine in engines {
            units.push(WorkUnit {
                cohort: sample.sidecar.cohort.clone(),
                sample_id: sample.sidecar.id.clone(),
                engine: engine.clone(),
                bytes: bytes.clone(),
            });
        }
    }
    Ok(units)
}

fn dispatch(
    work: Vec<WorkUnit>,
    jobs: Option<usize>,
    lcs_path: Option<PathBuf>,
) -> Result<Vec<WorkResult>, RunError> {
    let lcs_ref = lcs_path.as_deref();
    let do_unit = |unit: WorkUnit| -> WorkResult {
        let outcome = scan(&unit.bytes, &unit.engine, lcs_ref);
        WorkResult {
            cohort: unit.cohort,
            sample_id: unit.sample_id,
            engine: unit.engine,
            outcome,
        }
    };

    let mut results: Vec<WorkResult> = match jobs {
        Some(n) if n > 0 => {
            let pool = ThreadPoolBuilder::new()
                .num_threads(n)
                .build()
                .map_err(|e| RunError::ThreadPoolBuildFailed(e.to_string()))?;
            pool.install(|| work.into_par_iter().map(do_unit).collect())
        }
        _ => work.into_par_iter().map(do_unit).collect(),
    };

    results.sort_by(|a, b| {
        (a.cohort.as_str(), a.sample_id.as_str(), a.engine.as_str()).cmp(&(
            b.cohort.as_str(),
            b.sample_id.as_str(),
            b.engine.as_str(),
        ))
    });
    Ok(results)
}

// --- CLI handlers ---------------------------------------------------------

pub fn run(common: Common, args: RunArgs) -> ExitCode {
    match execute(&common, &args) {
        Ok(record) => {
            print_summary(&record);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("run: {e}");
            ExitCode::from(2)
        }
    }
}

pub fn inspect(_common: Common, _args: InspectArgs) -> ExitCode {
    eprintln!("inspect: not yet implemented (Phase 7)");
    ExitCode::from(2)
}

fn print_summary(record: &RunRecord) {
    println!(
        "Run started {} → finished {} ({} samples, {} work results)",
        record.started_at.to_rfc3339(),
        record.finished_at.to_rfc3339(),
        record.sample_count,
        record.work_results.len()
    );
    println!("Engines:");
    for s in &record.engine_statuses {
        match s.state {
            Availability::Available => println!("  {} → available", s.name),
            Availability::Skipped => println!(
                "  {} → skipped ({})",
                s.name,
                s.skip_reason.as_deref().unwrap_or("no reason")
            ),
        }
    }

    let mut available: Vec<&str> = record
        .engine_statuses
        .iter()
        .filter(|s| s.state == Availability::Available)
        .map(|s| s.name.as_str())
        .collect();
    available.sort();

    println!("Per-engine outcomes:");
    for engine in available {
        let mut clean = 0u32;
        let mut threat = 0u32;
        let mut errors = 0u32;
        let mut total_latency: u64 = 0;
        let mut completed: u64 = 0;
        for w in &record.work_results {
            if w.engine != engine {
                continue;
            }
            match &w.outcome {
                Ok(o) => {
                    if o.report.clean {
                        clean += 1;
                    } else {
                        threat += 1;
                    }
                    total_latency = total_latency.saturating_add(o.latency_ms);
                    completed += 1;
                }
                Err(_) => errors += 1,
            }
        }
        let avg = if completed > 0 {
            total_latency / completed
        } else {
            0
        };
        println!(
            "  {engine:<8}  clean={clean:>4}  threat={threat:>4}  errors={errors:>4}  avg_latency_ms={avg}"
        );
    }
}

// --- tests ---------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::sample::{Format, Sidecar, Verdict};
    use std::path::PathBuf;

    fn sample(cohort: &str, id: &str) -> Sample {
        Sample {
            sidecar: Sidecar {
                id: id.into(),
                text_path: format!("{id}.txt"),
                cohort: cohort.into(),
                verdict: Verdict::Clean,
                format: Format::RawText,
                source: "test".into(),
                license: "internal".into(),
                expected_categories: vec![],
                expected_min_severity: None,
                seed_id: None,
                tags: vec![],
                notes: String::new(),
            },
            text_full_path: PathBuf::from(format!("{cohort}/{id}.txt")),
            sidecar_path: PathBuf::from(format!("{cohort}/{id}.toml")),
        }
    }

    #[test]
    fn pattern_exact_and_glob() {
        assert!(pattern_matches("seed-handcurated", "seed-handcurated"));
        assert!(!pattern_matches("seed-handcurated", "synthetic-llama"));
        assert!(pattern_matches("synthetic-*", "synthetic-llama-paraphrase"));
        assert!(!pattern_matches("synthetic-*", "seed-handcurated"));
        assert!(pattern_matches("*", "anything"));
    }

    #[test]
    fn no_filters_keeps_all_samples() {
        let s = vec![sample("a", "1"), sample("b", "2")];
        let kept = filter_samples(s, &[], &[]);
        assert_eq!(kept.len(), 2);
    }

    #[test]
    fn include_filter_restricts() {
        let s = vec![sample("a", "1"), sample("b", "2"), sample("a", "3")];
        let kept = filter_samples(s, &["a".into()], &[]);
        assert_eq!(kept.len(), 2);
        for k in &kept {
            assert_eq!(k.sidecar.cohort, "a");
        }
    }

    #[test]
    fn exclude_filter_drops_glob_matches() {
        let s = vec![
            sample("seed-handcurated", "1"),
            sample("synthetic-llama", "2"),
            sample("synthetic-mixtral", "3"),
        ];
        let kept = filter_samples(s, &[], &["synthetic-*".into()]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].sidecar.cohort, "seed-handcurated");
    }

    #[test]
    fn include_then_exclude_compose() {
        let s = vec![
            sample("seed-handcurated", "1"),
            sample("synthetic-llama", "2"),
            sample("synthetic-mixtral", "3"),
        ];
        // Include everything, then exclude synthetic-mixtral specifically.
        let kept = filter_samples(s, &[], &["synthetic-mixtral".into()]);
        assert_eq!(kept.len(), 2);
    }

    #[test]
    fn resolve_engines_defaults_when_empty() {
        let resolved = resolve_engine_list(&[]);
        assert_eq!(resolved, vec!["simple", "yara", "syara"]);
    }

    #[test]
    fn resolve_engines_uses_explicit_list() {
        let resolved = resolve_engine_list(&["simple".into(), "yara".into()]);
        assert_eq!(resolved, vec!["simple", "yara"]);
    }

    // --- live integration test against real lcs 0.5.3 + seed corpus ------

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn execute_against_seed_cohort_runs_full_matrix() {
        let common = Common {
            lcs_path: None,
            samples_dir: manifest_dir().join("samples"),
        };
        let args = RunArgs {
            cohort: vec!["seed-handcurated".into()],
            exclude_cohort: vec![],
            engines: vec![],
            jobs: Some(2),
            runs_dir: PathBuf::from("runs"),
        };
        let record = execute(&common, &args).expect("seed run must succeed");

        assert_eq!(record.sample_count, 12, "seed cohort has 12 samples");
        assert_eq!(record.engine_statuses.len(), 3);
        for s in &record.engine_statuses {
            assert_eq!(
                s.state,
                Availability::Available,
                "engine {} unexpectedly skipped: {:?}",
                s.name,
                s.skip_reason
            );
        }
        assert_eq!(
            record.work_results.len(),
            12 * 3,
            "12 samples × 3 engines = 36 work results"
        );

        // Sorted by (cohort, sample_id, engine)
        let mut prev: Option<(&str, &str, &str)> = None;
        for w in &record.work_results {
            let key = (w.cohort.as_str(), w.sample_id.as_str(), w.engine.as_str());
            if let Some(p) = prev {
                assert!(p <= key, "work_results not sorted: {:?} > {:?}", p, key);
            }
            prev = Some(key);
        }

        // Every clean sample should land Ok with report.clean == true on the
        // simple engine; every threat-001 (prompt_injection) should fire on
        // simple. This is the contract the seed cohort is expected to honour.
        let clean_ok_simple = record.work_results.iter().any(|w| {
            w.engine == "simple"
                && w.sample_id == "seed-clean-001"
                && w.outcome.as_ref().is_ok_and(|o| o.report.clean)
        });
        assert!(clean_ok_simple, "seed-clean-001 should be clean on simple");

        let threat_001_fires_simple = record.work_results.iter().any(|w| {
            w.engine == "simple"
                && w.sample_id == "seed-threat-001"
                && w.outcome.as_ref().is_ok_and(|o| !o.report.clean)
        });
        assert!(
            threat_001_fires_simple,
            "seed-threat-001 should fire on simple"
        );
    }

    #[test]
    fn execute_with_engine_filter_narrows_matrix() {
        let common = Common {
            lcs_path: None,
            samples_dir: manifest_dir().join("samples"),
        };
        let args = RunArgs {
            cohort: vec!["seed-handcurated".into()],
            exclude_cohort: vec![],
            engines: vec!["simple".into()],
            jobs: Some(2),
            runs_dir: PathBuf::from("runs"),
        };
        let record = execute(&common, &args).expect("simple-only run must succeed");

        assert_eq!(record.requested_engines, vec!["simple"]);
        assert_eq!(record.engine_statuses.len(), 1);
        assert_eq!(record.work_results.len(), 12);
        for w in &record.work_results {
            assert_eq!(w.engine, "simple");
        }
    }

    #[test]
    fn execute_errors_when_no_samples_match_filter() {
        let common = Common {
            lcs_path: None,
            samples_dir: manifest_dir().join("samples"),
        };
        let args = RunArgs {
            cohort: vec!["nonexistent-cohort".into()],
            exclude_cohort: vec![],
            engines: vec![],
            jobs: None,
            runs_dir: PathBuf::from("runs"),
        };
        match execute(&common, &args) {
            Err(RunError::NoSamples) => {}
            other => panic!("expected NoSamples, got {other:?}"),
        }
    }

    #[test]
    fn execute_errors_when_no_engines_available() {
        let common = Common {
            lcs_path: None,
            samples_dir: manifest_dir().join("samples"),
        };
        let args = RunArgs {
            cohort: vec!["seed-handcurated".into()],
            exclude_cohort: vec![],
            engines: vec!["definitely-not-an-engine".into()],
            jobs: None,
            runs_dir: PathBuf::from("runs"),
        };
        match execute(&common, &args) {
            Err(RunError::NoAvailableEngines { statuses }) => {
                assert_eq!(statuses.len(), 1);
                assert_eq!(statuses[0].state, Availability::Skipped);
            }
            other => panic!("expected NoAvailableEngines, got {other:?}"),
        }
    }
}

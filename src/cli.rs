use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "shield-harness",
    version,
    about = "Benchmarking + regression harness for llm_context_shield (lcs)"
)]
pub struct Cli {
    /// Path to the lcs binary (default: discovered via PATH)
    #[arg(long, global = true)]
    pub lcs_path: Option<PathBuf>,

    /// Corpus root directory
    #[arg(long, global = true, default_value = "samples")]
    pub samples_dir: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Validate corpus integrity (sidecar schema, ids, paths, license, expected categories)
    Validate(ValidateArgs),
    /// Run lcs across the corpus and produce a run record
    Run(RunArgs),
    /// Diff two runs, or compare cohorts within one run; CI gate mode supported
    Diff(DiffArgs),
    /// Generate synthetic samples from seeds via a local LMStudio endpoint
    Synth(SynthArgs),
    /// Import samples from an external source into a named cohort
    Import(ImportArgs),
    /// Inspect a single sample across all available engines
    Inspect(InspectArgs),
}

#[derive(Args, Debug)]
pub struct ValidateArgs {}

#[derive(Args, Debug)]
pub struct RunArgs {
    /// Restrict to specific cohorts (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub cohort: Vec<String>,

    /// Exclude cohorts matching the given globs (comma-separated, e.g. 'synthetic-*')
    #[arg(long, value_delimiter = ',')]
    pub exclude_cohort: Vec<String>,

    /// Engines to test (comma-separated). Default: all probed available engines
    #[arg(long, value_delimiter = ',')]
    pub engines: Vec<String>,

    /// Worker thread count (default: num_cpus)
    #[arg(long)]
    pub jobs: Option<usize>,

    /// Enable per-rule attribution from lcs --log (forces --jobs 1)
    #[arg(long)]
    pub attribute_rules: bool,

    /// Run output directory
    #[arg(long, default_value = "runs")]
    pub runs_dir: PathBuf,
}

#[derive(Args, Debug)]
pub struct DiffArgs {
    /// Baseline run directory (default: baselines/current)
    #[arg(long)]
    pub baseline: Option<PathBuf>,

    /// Candidate run directory (default: latest under runs/)
    pub candidate: Option<PathBuf>,

    /// F1 delta threshold (negative = regression). Breach exits 1 with --ci-gate.
    #[arg(long, default_value_t = 0.02)]
    pub threshold_f1: f64,

    /// p95 latency increase threshold (fraction; 0.10 = 10%). Breach exits 1 with --ci-gate.
    #[arg(long, default_value_t = 0.10)]
    pub threshold_latency: f64,

    /// Exit non-zero on threshold breach (CI gate mode)
    #[arg(long)]
    pub ci_gate: bool,

    /// Compare cohorts within a single run instead of across runs
    #[arg(long)]
    pub within: Option<PathBuf>,

    /// In --within mode, compare cohorts side-by-side
    #[arg(long, requires = "within")]
    pub by_cohort: bool,

    /// Allow comparing runs with different lcs --version values
    #[arg(long)]
    pub allow_version_drift: bool,
}

#[derive(Args, Debug)]
pub struct SynthArgs {
    /// Seed sample id to vary
    #[arg(long)]
    pub seed: String,

    /// Generation strategy
    #[arg(long, default_value = "paraphrase")]
    pub strategy: String,

    /// Number of variants to generate
    #[arg(long, default_value_t = 1)]
    pub n: usize,

    /// LMStudio (or OpenAI-compatible) base URL
    #[arg(long, default_value = "http://localhost:1234/v1")]
    pub endpoint: String,

    /// Model name (default: auto-discover from /v1/models)
    #[arg(long)]
    pub model: Option<String>,
}

#[derive(Args, Debug)]
pub struct ImportArgs {
    /// Cohort name for the imported samples (required)
    #[arg(long)]
    pub cohort: String,

    #[command(subcommand)]
    pub source: ImportSource,
}

#[derive(Subcommand, Debug)]
pub enum ImportSource {
    /// Pull files from a GitHub repository
    Github {
        /// Repository slug, e.g. "owner/repo"
        repo: String,
        /// Git ref (branch, tag, or commit sha)
        #[arg(long)]
        git_ref: String,
        /// Glob of paths to fetch
        #[arg(long)]
        path_glob: String,
    },
    /// Pull from a HuggingFace dataset
    Huggingface {
        /// Dataset slug, e.g. "owner/dataset"
        dataset: String,
    },
    /// Import from a local directory + manifest
    Local {
        /// Directory holding the raw sample files
        dir: PathBuf,
        /// TOML manifest of metadata for each file
        manifest: PathBuf,
    },
}

#[derive(Args, Debug)]
pub struct InspectArgs {
    /// Sample id to inspect
    pub id: String,

    /// Restrict inspection to specific engines (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub engines: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Common {
    pub lcs_path: Option<PathBuf>,
    pub samples_dir: PathBuf,
}

pub fn run() -> ExitCode {
    let cli = Cli::parse();
    let common = Common {
        lcs_path: cli.lcs_path,
        samples_dir: cli.samples_dir,
    };
    match cli.command {
        Command::Validate(a) => crate::corpus::validate::run(common, a),
        Command::Run(a) => crate::runner::orchestrator::run(common, a),
        Command::Diff(a) => crate::diff::run(common, a),
        Command::Synth(a) => crate::synth::lmstudio::run(common, a),
        Command::Import(a) => crate::import::run(common, a),
        Command::Inspect(a) => crate::runner::orchestrator::inspect(common, a),
    }
}

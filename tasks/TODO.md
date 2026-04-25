# `shield-harness` — Plan of Work

**Source of record** for what's done, what's in flight, and what's next. This is the canonical breakdown referenced from [`PRD.md`](../PRD.md) §6 and [`ARCHITECTURE.md`](../ARCHITECTURE.md).

## Conventions

- Each phase has a discrete scope, a checklist, and a **Done when** acceptance line.
- After every phase a **⏸ Pause for review** marker. The implementing agent stops, summarises what changed, and waits. **The operator controls the commit/push cycle** — no commits, no pushes, no PR creation without explicit instruction.
- Sub-phases inside a phase get the same pause marker if the operator flags them as commit-worthy boundaries.
- Tests are mandatory per `CLAUDE.md` §4: real tests, no mocks-of-the-thing-under-test, no always-true assertions.
- `LESSONS.md` is updated after any user correction (`CLAUDE.md` §3).

## Status snapshot

| Phase | Topic | Status |
|---|---|---|
| 0 | Project skeleton — Cargo.toml, CLI shell, module scaffolding | ✅ Done |
| 1 | Corpus: schema, loader, validator, first cohort | 🚧 1a done; 1b next |
| 2 | Run pipeline: probe, invoke, orchestrator, run record | 📅 |
| 3 | Metrics: P/R/F1, latency, summary, CSV | 📅 |
| 4 | Diff + CI gate | 📅 |
| 5 | Importers (per-source adapters) | 📅 |
| 6 | Synth via LMStudio | 📅 |
| 7 | Per-rule attribution from `lcs --log` | 📅 |
| 8 | Confusion-matrix / false-positive explorer | 📅 Future |
| 9 | Adversarial mutation engine | 📅 Future |
| 10 | ReDB-backed corpus + read/query server | 📅 Future |
| 11 | Public corpus split (licence-filtered export) | 📅 Future |

---

## Phase 0 — Project skeleton

**Goal:** runnable binary with all subcommand stubs, blessed deps wired in, module tree in place. No real logic yet.

### Tasks

- [x] PRD.md and ARCHITECTURE.md authored.
- [x] CLAUDE.md initialised with project start date.
- [x] `Cargo.toml` populated with blessed deps, version-pinned (exact `=` pins), N-1 (or older) per the 30-day rule. `time` deferred until needed (blessed-but-dormant).
- [x] `src/main.rs` replaced with thin entry point that dispatches to `cli::run()`.
- [x] `src/cli.rs` defines clap subcommands: `validate`, `run`, `diff`, `synth`, `import`, `inspect` with all flags from ARCHITECTURE.
- [x] Module scaffold created with stubs returning `ExitCode::from(2)` and a "not yet implemented (Phase X)" message.
- [x] `.gitignore` updated for `/runs/` and `/baselines/`.
- [x] `tasks/LESSONS.md` and `tasks/BUGS.md` initialised.
- [x] `cargo build` succeeds (9 expected dead_code warnings on unused stubs — they resolve as later phases land).
- [x] `cargo run -- --help` prints the subcommand shell.
- [x] `cargo run -- run --help` prints flags from ARCHITECTURE.

**Done when:** binary builds, `--help` shows the planned CLI surface, every module file exists and compiles, no real corpus or run logic is implemented.

**⏸ Pause for review.**

---

## Phase 1 — Corpus

**Goal:** corpus schema is real and round-trips. A `validate` subcommand catches every common mistake. First cohort (`seed-handcurated`) holds at least a dozen deliberately-chosen samples.

### Sub-phase 1a — Schema & loader

- [x] `corpus::sample::Sidecar` struct with serde derive matching `ARCHITECTURE.md` §6.2. (Phase 0)
- [x] `corpus::sample::Sample` aggregating sidecar + lazy text bytes (added `Sample::read_bytes`).
- [x] `corpus::loader::load_corpus(root) -> Result<Vec<Sample>>`. Walks `samples/<cohort>/<verdict>/`. Sorted output by `(cohort, id)`.
- [x] Tests: load a fixture corpus directory, assert ordering, assert every field deserialises.

**Done — fixture at `tests/fixtures/loader-basic/` (3 samples, 2 cohorts, both verdicts, all 4 sidecar formats not yet — only `raw_text` + `markdown`; the remaining two formats land naturally in 1c when the seed cohort is built). 4 unit tests in `corpus::loader::tests` pass: ordering, full-field round-trip, text-path resolution via `read_bytes`, and root-not-a-dir error.**

**⏸ Pause for review (sub-phase boundary).**

### Sub-phase 1b — Validator

- [ ] `corpus::validate` performs:
  - Sidecar `cohort` field equals enclosing directory name.
  - Sidecar `verdict` field equals enclosing verdict directory name.
  - `text_path` resolves to an existing file.
  - Global `id` uniqueness across all cohorts.
  - `expected_categories` are recognised by the installed `lcs` (`lcs list -e <engine>` per available engine — requires Phase 2 probe code, so for 1b stub the lcs check behind a flag and produce a TODO marker).
  - For `verdict = "threat"`: `expected_categories` non-empty.
  - License field is non-empty and matches an allow-list (`MIT`, `Apache-2.0`, `BSD-*`, `CC-BY-*`, `CC0`, `internal`, `synthetic`).
- [ ] Tests for each failure mode (one fixture per failure). Tests for the happy path on the seed cohort.

### Sub-phase 1c — Seed cohort

- [ ] Create `samples/seed-handcurated/clean/` with at least 6 hand-written clean samples covering all four formats (`raw_text`, `markdown`, `html`, `chat_history`).
- [ ] Create `samples/seed-handcurated/threat/` with at least 6 hand-written threat samples covering at least 4 distinct `lcs` categories.
- [ ] Each sample has a complete sidecar.
- [ ] `cargo run -- validate` exits 0 on the seed cohort.

**Done when:** `validate` is a real working subcommand, the seed cohort exists and is fully labelled, every validator failure mode has a test, and the operator can manually break a sidecar and watch validate catch it.

**⏸ Pause for review.**

---

## Phase 2 — Run pipeline

**Goal:** `shield-harness run` actually invokes `lcs`, captures outcomes per (cohort, sample, engine), and writes a complete run record under `runs/`.

### Sub-phase 2a — `lcs` invocation primitive

- [ ] `runner::invoke::scan(sample, engine) -> ScanOutcome`. Pipes sample bytes to stdin of `lcs scan -e <eng> -f json`. Captures stdout, stderr, exit code, wall-clock latency.
- [ ] `lcs` binary discovery: `--lcs-path` flag, fall back to `which lcs`. Resolved path captured.
- [ ] JSON output parsed into `Finding` records using the schema documented in `llm_context_shield/README.md`.
- [ ] Tests against a real local `lcs` binary if available; integration tests gated by `cargo test --features integration` so CI without `lcs` still passes unit tests.

### Sub-phase 2b — Engine availability probe

- [ ] `runner::probe::probe_engines(requested) -> Vec<EngineStatus>`. Implements the state machine in ARCHITECTURE §5 against `lcs scan -e <eng> -f quiet` with a tiny constant input.
- [ ] Stderr parsed for the skip reason (feature missing, ONNX runtime missing, LMStudio unreachable).
- [ ] Tests for each known skip reason against fixture stderr strings; integration test against the real `lcs` for the available engines.

### Sub-phase 2c — Orchestrator

- [ ] `runner::orchestrator` builds the work-unit list `(cohort, sample_id, engine_name)`, dispatches via rayon `par_iter`, collects outcomes, sorts, returns `RunRecord`.
- [ ] `--jobs N` flag wired through.
- [ ] `--cohort` and `--exclude-cohort` filters applied at corpus load time.
- [ ] `--engines` filter applied to the engine matrix.

### Sub-phase 2d — Run record persistence

- [ ] `report::record::write_run(dir, record)` produces the directory layout in ARCHITECTURE §7: `meta.json`, `outputs/<engine>.jsonl`, `run.json`. (Metrics/summary land in Phase 3.)
- [ ] `meta.json` captures `lcs --version`, harness git SHA (from build-time env or runtime `git rev-parse`), corpus content hash (sha256 over sorted (sidecar_path, hash-of-content, text_path, hash-of-content) rows), started_at, finished_at, requested engines, host info.
- [ ] Run directory is `runs/<UTC-RFC3339-timestamp>-<short-sha>/`.
- [ ] Tests verifying the directory layout exists after a run, files contain expected top-level keys, and a second run produces a distinct directory.

**Done when:** `cargo run -- run` against the seed corpus produces a real run directory containing real JSON records of real `lcs` invocations across at least the `simple` engine. Skipped engines are recorded with reasons.

**⏸ Pause for review.**

---

## Phase 3 — Metrics & summary

**Goal:** every run produces meaningful per-cohort, per-category, per-engine numbers, both as `metrics.csv` and as a human-readable `summary.txt`.

### Sub-phase 3a — P/R/F1

- [ ] `metrics::prf::compute(outcomes, ground_truth) -> Vec<PrfRow>`. Rows keyed by `(cohort, category, engine)` plus `(*, *, engine)` rollups.
- [ ] Verdict mapping: a sample is a true positive for `category C` on `engine E` if the sidecar lists `C` in `expected_categories` and engine `E`'s outcome contains a finding with `category == C` at or above `expected_min_severity`. Other quadrants follow the standard confusion matrix.
- [ ] Tests with synthetic outcome fixtures covering tp/fp/tn/fn boundary cases per category and per cohort.

### Sub-phase 3b — Latency percentiles

- [ ] `metrics::latency::compute(outcomes) -> Vec<LatencyRow>`. Rows keyed by `(cohort, engine)` with p50/p95/p99 in ms.
- [ ] Use the standard "nearest-rank" percentile algorithm; document it in a comment so the determinism contract is auditable.
- [ ] Tests for percentile correctness on known input vectors.

### Sub-phase 3c — Reporting

- [ ] `report::record::write_metrics_csv(dir, prf_rows, latency_rows)` produces `metrics.csv` with stable column order.
- [ ] `report::summary::print(metrics, &mut io::Write)` prints a human-readable rollup (overall F1 per engine, biggest-mover categories, latency table). Same output goes to stdout and to `summary.txt`.
- [ ] Tests against fixture metrics asserting the formatted output matches an expected string.

**Done when:** a run end-to-end produces all five files in the run directory, the operator can read the summary and immediately know per-engine F1 + p95 latency.

**⏸ Pause for review.**

---

## Phase 4 — Diff + CI gate

**Goal:** `shield-harness diff` produces a focused regression report and exits non-zero on threshold breach when invoked as a CI gate.

### Tasks

- [ ] `diff::load_runs(baseline, candidate)`; baseline defaults to `baselines/current` symlink, candidate defaults to the latest run under `runs/`.
- [ ] Alignment by `(cohort, sample_id, engine_name)`; samples present in only one run are listed in an "added/removed" section.
- [ ] Verdict-flip enumeration with sample id and the categories that changed.
- [ ] Per-`(cohort, category)` F1 delta; per-`(cohort, engine)` p95 latency delta.
- [ ] `--threshold-f1 <delta>` and `--threshold-latency <pct>` flags; `--ci-gate` makes threshold breach exit 1.
- [ ] `--within <run> --by-cohort` mode for within-run cross-cohort comparison.
- [ ] `--allow-version-drift` flag so diff can compare runs across `lcs --version` boundaries.
- [ ] Tests against fixture run directories.

**Done when:** the operator can change one rule in `lcs`, run twice, and see a focused diff that names exactly the samples and categories that moved.

**⏸ Pause for review.**

---

## Phase 5 — Importers

**Goal:** the corpus can grow from external sources without hand-copying files. One adapter per source.

### Tasks

- [ ] `import::Adapter` trait: `fn list(...) -> Vec<RawSample>`; `fn fetch(raw) -> Bytes`; `fn metadata(raw) -> SidecarSeed`.
- [ ] Adapter: GitHub raw-file fetcher (given org/repo/ref/path-glob, downloads + creates sidecars with `source = "github:..."` and the correct license discovered from the repo's LICENSE file).
- [ ] Adapter: HuggingFace dataset fetcher (HTTP API, paginated, license carried from dataset metadata).
- [ ] Adapter: local-file batch importer (point at a directory of pre-downloaded files + a TOML manifest of metadata).
- [ ] Idempotency: importing the same source twice is a no-op (id collision is a soft skip with a log line, not an error).
- [ ] Each import goes into a single named cohort (`--cohort <name>` flag is required).
- [ ] Tests against recorded fixture HTTP responses; no live network in unit tests.

**Done when:** at least one external source has been imported into a real cohort, validate passes, and a run can scope to that cohort.

**⏸ Pause for review.**

---

## Phase 6 — Synth via LMStudio

**Goal:** `shield-harness synth` generates variants of seed samples through a local LMStudio endpoint and writes them as a fully-formed cohort.

### Tasks

- [ ] `synth::lmstudio::Client` over `ureq`. Endpoint default `http://localhost:1234/v1`. Auto-discover model via `/v1/models` if `--model` not passed.
- [ ] Generation strategies (v0.1):
  - `paraphrase` — preserve intent and verdict, vary surface.
  - `translate-roundtrip` — translate to a second language and back; preserves intent in noisier surface.
- [ ] Each generated sample lands at `samples/synthetic-<model-tag>-<strategy>/<verdict>/<id>.<ext>` with `seed_id` set, `cohort` set, and `tags` recording the model tag and strategy.
- [ ] Connection-failure handling: if the endpoint is unreachable, exit 2 with a clear error and instructions for starting LMStudio.
- [ ] Tests: HTTP client unit tests against fixture responses; integration test gated on `LMSTUDIO_ENDPOINT` env var.

**Done when:** the operator can run `shield-harness synth --seed 0101 --strategy paraphrase --n 5` and end up with 5 new samples in a synthetic cohort, all of which `validate` accepts.

**⏸ Pause for review.**

---

## Phase 7 — Per-rule attribution from `lcs --log`

**Goal:** `shield-harness inspect <id>` and (optionally) `run --attribute-rules` surface which rule(s) inside `lcs` fired on which sample.

### Tasks

- [ ] `runner::log_scrape::Tailer`: records `(file_path, byte_offset)` for every file under `$XDG_STATE_HOME/llm_context_shield/` at run start.
- [ ] After each scan completes, the tailer reads appended bytes since the recorded offset for that file and stores them as the scan's `log_lines`.
- [ ] Parser extracts rule names from the log lines (best-effort regex against the current format; if the format changes, log a warning and produce empty attribution rather than aborting).
- [ ] `--attribute-rules` flag on `run` forces `--jobs 1` and enables attribution.
- [ ] `inspect <id>` runs every available engine on a single sample with attribution on, dumps the raw `lcs` JSON + the captured log slice for each engine.
- [ ] Tests against fixture log files.

**Done when:** the operator can ask "why did sample 0101 false-positive on the `yara` engine?" and get an answer naming specific rule names.

**⏸ Pause for review.**

---

## Phase 8 — Confusion-matrix / false-positive explorer (FUTURE)

Out of scope for v0.1. Will be planned in detail when prior phases are stable. Expected surface: an `explore` subcommand that prints / serves a per-cohort confusion matrix and a ranked list of "samples most often misclassified across runs."

---

## Phase 9 — Adversarial mutation engine (FUTURE)

Out of scope for v0.1. Higher-risk because it can drift the corpus toward "things `lcs` happens not to detect." Will need a curation loop (operator approves each generated sample) before it can grow the official cohorts.

---

## Phase 10 — ReDB-backed corpus + read/query server (FUTURE)

Migrate corpus storage from file-per-sample to a ReDB-backed store. Add a `serve` subcommand that exposes a query API for downstream tooling. Out of scope for v0.1; unlikely before v0.3.

---

## Phase 11 — Public corpus split (FUTURE)

Mechanical export of all samples whose `license` field is in a public-allow-list. Sidecar `cohort` field becomes the unit of selection. Out of scope for v0.1; needed when there's a reason to share.

---

## Active checklist (current sprint)

Edit this section as work happens; this is the at-a-glance "what's next" view.

- **Now:** Phase 1a complete — awaiting review.
- **Next:** Phase 1b — validator (cohort/verdict-name agreement, text_path existence, id uniqueness, license allow-list, threat-needs-categories; lcs-vocabulary check stubbed behind a flag until Phase 2 probe lands).
- **Blocked / waiting:** none.

## Cross-cutting reminders

- Every bug fix gets a unit test (`CLAUDE.md` §6).
- Every user correction gets a `LESSONS.md` entry (`CLAUDE.md` §3).
- Pause and ask before pushing past 500 LOC in any single file (`CLAUDE.md` development guidelines).
- Validate dep choices against the blessed-set table in `ARCHITECTURE.md` §13.1 before adding anything new.
- Verify every decision against PRD use-cases — if a task can't be traced to UC-1..UC-6, raise it before doing the work.

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
| 1 | Corpus: schema, loader, validator, first cohort | ✅ Done (1a + 1b + 1b.5 + 1c) |
| 2 | Run pipeline: probe, invoke, orchestrator, run record | 🚧 2a done (against lcs 0.5.3); 2b/2c/2d remaining |
| 3 | Metrics: P/R/F1, latency, summary, CSV | 📅 (blocked on Phase 2) |
| 4 | Diff + CI gate (incl. `rule_set_fingerprint` drift detection) | 📅 |
| 5 | Importers (per-source adapters) | 📅 |
| 6 | Synth via LMStudio | 📅 |
| 7 | Confusion-matrix / false-positive explorer | 📅 Future |
| 8 | Adversarial mutation engine | 📅 Future |
| 9 | ReDB-backed corpus + read/query server | 📅 Future |
| 10 | Public corpus split (licence-filtered export) | 📅 Future |

> **Phase 7 deleted 2026-04-26.** The original Phase 7 (per-rule attribution from `lcs --log` byte-offset scrape) is obsoleted by lcs 0.5.2's `findings[].rule_name`. Phases 8–11 renumbered to 7–10.

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

- [x] `corpus::validate` performs:
  - [x] Sidecar `cohort` field equals enclosing directory name.
  - [x] Sidecar `verdict` field equals enclosing verdict directory name.
  - [x] `text_path` resolves to an existing file.
  - [x] Global `id` uniqueness across all cohorts.
  - [x] `expected_categories` are recognised by the installed `lcs` — **stubbed** behind `--check-lcs-categories`; emits a non-blocking `LcsCategoryCheckPending` notice. Real probe lands in Phase 2.
  - [x] For `verdict = "threat"`: `expected_categories` non-empty.
  - [x] License field is non-empty and matches an allow-list (`MIT`, `Apache-2.0`, `BSD-*`, `CC-BY-*`, `CC0`, `internal`, `synthetic`). Glob entries (suffix `*`) match by prefix; everything else is exact.
- [x] Tests for each failure mode (one fixture per failure). Tests for the happy path on the seed cohort.

**Done — `corpus::validate::validate(samples, opts) -> Vec<Issue>` plus a `run` handler wired into the `Validate` subcommand. Issues classify as blocking or notice (`Issue::is_blocking`); CLI prints blocking to stderr and notices to stdout, exits 1 only on blocking. Fixtures at `tests/fixtures/validate-cases/{happy, cohort-dir-mismatch, verdict-dir-mismatch, missing-text-file, duplicate-id, threat-without-categories, license-{empty,disallowed,glob-ok}}/`. 14 unit tests in `corpus::validate::tests` (8 failure-mode + happy + glob-ok + allow-list table + pending-notice + 2 Display checks + load-error guard); plus the 4 1a tests, total 18 pass. Smoke-tested against four fixtures with the real CLI.**

**Note: Phase 1b deliberately did not run the seed cohort happy-path test — that lives in 1c when the seed corpus exists. The fixture `happy/` plays that role for now.**

**⏸ Pause for review (sub-phase boundary).**

### Sub-phase 1b.5 — Wire lcs 0.5.2 introspection (post-11.5 landing)

- [x] `runner::introspect::probe_categories(lcs_path, engine)` wraps `lcs rules --categories -e <engine>`. Distinguishes `LcsNotFound` (binary missing) from `EngineUnavailable` (engine-level skip) from `ParseFailed`.
- [x] Validator `Options` switches from `check_lcs_categories: bool` to `category_vocabulary: Option<BTreeSet<String>>`. CLI handler probes `simple`/`yara`/`syara`, builds union, populates vocabulary.
- [x] New `IssueKind::UnknownCategory { name }` (blocking) for categories outside the union vocabulary.
- [x] New `IssueKind::LcsProbeFailed { engine, reason }` (non-blocking notice) for per-engine probe failures. `LcsCategoryCheckPending` removed.
- [x] Lcs binary entirely missing → blocking error, exit 2 (does not poison `validate` runs that don't pass `--check-lcs-categories`).
- [x] Phase 7 deletion: `runner::log_scrape` module removed; `--attribute-rules` flag removed from `RunArgs`; ARCH §12.1 byte-offset narrative deleted.
- [x] Fixture `tests/fixtures/validate-cases/unknown-category/` + 3 vocab tests + 2 display tests added; obsolete `LcsCategoryCheckPending` tests removed. 25 unit tests pass.
- [x] Smoke-tested against real lcs 0.5.2: `validate --check-lcs-categories` exits 0 on `happy/`, exits 1 on `unknown-category/`, exits 2 on `--lcs-path /nope/...`.

**Done — single-commit bundle resolves the lcs 11.5 dependency. PRD §3.4, ARCH §1/§3/§4/§6.2/§7/§12.1/§14, CONTINUITY.md, and the phase plan all updated in the same commit.**

**⏸ Pause for review (sub-phase boundary).**

### Sub-phase 1c — Seed cohort

- [x] Create `samples/seed-handcurated/clean/` with at least 6 hand-written clean samples covering all four formats (`raw_text`, `markdown`, `html`, `chat_history`).
- [x] Create `samples/seed-handcurated/threat/` with at least 6 hand-written threat samples covering at least 4 distinct `lcs` categories.
- [x] Each sample has a complete sidecar.
- [x] `cargo run -- validate` exits 0 on the seed cohort.
- [x] `cargo run -- validate --check-lcs-categories` exits 0 (vocab union check passes for all 6 declared categories).

**Done — drafted by Claude 2026-04-26 pending operator edit pass. 6 clean (2 raw_text, 2 markdown, 1 html, 1 chat_history) + 6 threat (prompt_injection, jailbreak, secret_probing, context_shift, data_exfiltration, obfuscation). 25 tests still green; validate exits 0 in both modes.**

**Out-of-band fire-rate probe against real lcs 0.5.2 (informational — not a Phase 1c gate):**

| Sample | Expected | simple | yara | syara |
|---|---|---|---|---|
| 001 | prompt_injection | ✓ | ✓ + refusal_suppression | ✓ + refusal_suppression |
| 002 | jailbreak | ✓ + prompt_injection | ✓ | ✓ |
| 003 | secret_probing | — | — | — |
| 004 | context_shift | — | — | — |
| 005 | data_exfiltration | — | — | — |
| 006 | obfuscation | hidden_content | hidden_content + refusal_suppression | refusal_suppression |

Three samples (003, 004, 005) fire on no engine; one (006) fires under different categories than declared. Two interpretations:

1. **Detection gap** — these patterns are real attacks lcs doesn't catch; preserving the samples gives the harness genuine signal about lcs coverage holes (this is the harness's reason for existing).
2. **Sample drift** — these draft samples are too oblique to trigger any rule; tighten them to fire as labelled so v0.1 metrics aren't dragged down by intentionally-undetectable threats.

Both readings are valid. The operator-edit pass will resolve which is which on a per-sample basis. Either way, validator passes and Phase 1c is done.

**⏸ Pause for review.**

---

## Phase 2 — Run pipeline

**Unblocked 2026-04-26.** The lcs 11.5 dependency landed in lcs 0.5.2; the design here targets that surface (top-level `rule_set_fingerprint` + `threat_scores` on every `ScanReport`; `findings[].rule_name` + `findings[].engine` on every finding; `lcs rules --json` for the per-engine rule manifest).

**Goal:** `shield-harness run` actually invokes `lcs`, captures outcomes per (cohort, sample, engine), and writes a complete run record under `runs/`.

### Sub-phase 2a — `lcs` invocation primitive

- [x] `runner::invoke::scan(sample_bytes, engine, lcs_path) -> Result<ScanOutcome, ScanError>`. Pipes sample bytes to stdin of `lcs scan -e <eng> -f json`. Captures stdout, stderr, exit code, wall-clock latency.
- [x] `lcs` binary discovery: shared `runner::lcs::binary(Option<&Path>) -> PathBuf` resolver. Explicit path used verbatim; otherwise falls back to `"lcs"` for PATH lookup at exec time. `runner::introspect` refactored onto the same helper. (Resolved-path capture into `meta.json` lands with Phase 2d alongside other run-record metadata.)
- [x] JSON output parsed into `ScanReport` (top-level: `clean`, `finding_count`, `findings[]`, `rule_set_fingerprint`, `threat_scores.{class_scores, cumulative}`); each `Finding` deserialises `category`, `severity`, `description`, `matched_text`, `byte_range`, `rule_name`, `engine`. All required fields, no `Option<>` — lcs ≥ 0.5.3 is the contract (post 0.5.3 fix; see changelog).
- [x] Capture full `ScanReport` verbatim — `ScanOutcome.raw_stdout` is the un-parsed JSON ready for Phase 2d's `outputs/<engine>.jsonl` write path.
- [x] Tests against a real local `lcs` binary — 7 live integration tests covering: clean exit-0, threat exit-1 with prompt_injection, multi-finding syara, raw-stdout round-trip equality, latency capture, ParseFailed via `/bin/echo` stand-in, LcsNotFound on missing binary. **No runtime skip and no Cargo feature** — operator preference (2026-04-27) is to test live against lcs as we go since there's no CI yet.

**Done — `runner::scan_report`, `runner::lcs`, `runner::invoke` all landed; 39 tests pass against lcs 0.5.3. New `dead_code` warnings on `ScanOutcome.stderr`, `ScanOutcome`, `ScanError`, `scan`, `ScanReport`, `Finding`, `ThreatScores` — pending consumers in 2c/2d. Per project convention, warnings are not suppressed; they are forced reminders of unfinished work.**

**⏸ Pause for review (sub-phase boundary).**

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
- [ ] `meta.json` captures `lcs --version`, harness git SHA (from build-time env or runtime `git rev-parse`), corpus content hash (sha256 over sorted (sidecar_path, hash-of-content, text_path, hash-of-content) rows), started_at, finished_at, requested engines, host info, **per-engine `rule_set_fingerprint`**, and the **full per-engine rule manifest** from `lcs rules --json -e <eng>` (so a stale run record carries the rule context it was scanned against).
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
- [ ] **Rule-set drift detection:** if baseline and candidate share `lcs --version` but differ in any per-engine `rule_set_fingerprint`, surface as "rule-set drift" (likely user-rules change). `--allow-rule-set-drift` flag mirrors `--allow-version-drift`.
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

## Phase 7 — Confusion-matrix / false-positive explorer (FUTURE)

Out of scope for v0.1. Will be planned in detail when prior phases are stable. Expected surface: an `explore` subcommand that prints / serves a per-cohort confusion matrix and a ranked list of "samples most often misclassified across runs." Per-rule attribution is now free (every `Finding` carries `rule_name`), so the explorer can group misclassifications by the specific rule that fired or failed to fire.

---

## Phase 8 — Adversarial mutation engine (FUTURE)

Out of scope for v0.1. Higher-risk because it can drift the corpus toward "things `lcs` happens not to detect." Will need a curation loop (operator approves each generated sample) before it can grow the official cohorts.

---

## Phase 9 — ReDB-backed corpus + read/query server (FUTURE)

Migrate corpus storage from file-per-sample to a ReDB-backed store. Add a `serve` subcommand that exposes a query API for downstream tooling. Out of scope for v0.1; unlikely before v0.3.

---

## Phase 10 — Public corpus split (FUTURE)

Mechanical export of all samples whose `license` field is in a public-allow-list. Sidecar `cohort` field becomes the unit of selection. Out of scope for v0.1; needed when there's a reason to share.

---

## Active checklist (current sprint)

Edit this section as work happens; this is the at-a-glance "what's next" view.

- **Now:** Phase 2a complete (`runner::invoke::scan` against lcs 0.5.3; types in `runner::scan_report`; shared `runner::lcs::binary`; 39 tests pass with 7 live integration tests). Pin bumped to lcs ≥ 0.5.3 after upstream `threat_scores` fix.
- **Next:** Phase 2b (`runner::probe::probe_engines` against `lcs scan -e <eng> -f quiet`). State machine in ARCH §5; stderr-derived skip reasons; no runtime skip on tests.
- **Blocked / waiting:** nothing.

## Cross-cutting reminders

- Every bug fix gets a unit test (`CLAUDE.md` §6).
- Every user correction gets a `LESSONS.md` entry (`CLAUDE.md` §3).
- Pause and ask before pushing past 500 LOC in any single file (`CLAUDE.md` development guidelines).
- Validate dep choices against the blessed-set table in `ARCHITECTURE.md` §13.1 before adding anything new.
- Verify every decision against PRD use-cases — if a task can't be traced to UC-1..UC-6, raise it before doing the work.

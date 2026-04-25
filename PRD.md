# Product Requirements Document — `shield-harness`

**Status:** Living document. Initial draft.
**Owner:** John ([@gatewaynode](https://github.com/gatewaynode))
**Last reviewed:** 2026-04-25

This document describes *what* `shield-harness` is, *who* it serves, and *which workflows* it must support. It is the source of truth for scope decisions. Phase-level *how* and *when* live in [`tasks/TODO.md`](tasks/TODO.md); architecture lives in [`ARCHITECTURE.md`](ARCHITECTURE.md). This PRD links to them and does not duplicate them.

The system under test is the sibling project [`llm_context_shield`](../llm_context_shield/) (binary: `lcs`). Familiarity with its [PRD](../llm_context_shield/PRD.md) and [README](../llm_context_shield/README.md) is assumed.

---

## 1. Vision

An external benchmarking and regression harness for `llm_context_shield`. The harness owns a labelled corpus of LLM-context inputs (clean and malicious, in multiple formats, drawn from multiple provenances), runs `lcs` over the corpus across its full engine matrix, and produces metrics and diffs that let the operator answer two questions on every change to `lcs`:

1. **Did detection quality regress?** (per-category precision / recall / F1, baselined.)
2. **Did detection cost regress?** (latency, throughput.)

The harness is strictly external. It treats `lcs` as a black-box subprocess. It never links the `llm_context_shield` library, never introspects internal data structures, and never edits `lcs` rules. Its only inside-the-box visibility is whatever `lcs --log` writes to disk.

## 2. Use Cases

The workflows that scope this project. Every roadmap item must trace back to one of these.

### UC-1 — One-shot benchmark run

Operator runs `shield-harness run` after a change to `lcs` rules. The harness scans every sample with every enabled engine, classifies each result against the sample's expected label, writes a per-run report (per-category P/R/F1, latency percentiles, per-engine comparison), and stores the run for future diffing.

- **Status:** v0.1 target

### UC-2 — Baselined regression diff

Operator compares the latest run against a stored baseline (or against the previous run) and gets a focused diff: which samples flipped verdict, which categories moved on F1, which engine cost more time. This is the primary feedback loop for rule tuning.

- **Status:** v0.1 target

### UC-3 — CI gate

A CI invocation runs the harness against the corpus and exits non-zero if F1 (overall, or for any single category) drops by more than a configured threshold versus a pinned baseline, or if p95 latency regresses beyond a configured threshold. The same binary is used as in UC-1; only the invocation flags differ.

- **Status:** v0.1 target

### UC-4 — Corpus ingestion

Operator imports samples from an external source — a GitHub repository, an academic dataset, a HuggingFace collection, a public conversation dump. Each imported sample lands as a labelled corpus entry with provenance (`source`, `license`, original URL or citation) preserved. Imports are idempotent and de-duplicating.

- **Status:** v0.1 target (importer scaffolding); per-source adapters added incrementally.

### UC-5 — Synthetic sample generation

Operator points the harness at a local LMStudio (OpenAI-compatible) endpoint and generates variants of existing samples — paraphrases of true-positive injection attempts, near-duplicates of true negatives — to grow the corpus and probe `lcs` robustness. Generation is local-only; nothing leaves the operator's network. Synthetic samples are tagged with `source = "synthetic"` and a reference to the seed sample's `id` so provenance is chainable.

- **Status:** v0.1 target — minimal generator; richer mutation strategies in roadmap.

### UC-6 — Single-sample inspection

Operator wants to know why a particular sample fired (or failed to fire). Harness re-runs that one sample under every engine, dumps the raw `lcs` JSON output, and surfaces the relevant lines from `lcs --log` for per-rule attribution. No library calls; everything comes from the subprocess and its logs.

- **Status:** v0.1 target

## 3. Functional Requirements

### 3.1 Corpus

- **Storage layout.** File-per-sample with sidecar TOML metadata. Conventional layout: `samples/<verdict>/<id>.<ext>` for the text and `samples/<verdict>/<id>.toml` for the sidecar. `<verdict>` is `clean` or `threat`.
- **Sidecar schema (v1).** Required fields: `id`, `text_path`, `verdict`, `format`, `source`, `license`. Threat samples additionally require `expected_categories` (list, drawn from the categories `lcs` itself reports — see §3.4) and `expected_min_severity`. Optional: `notes`, `seed_id` (for synthetic samples), `tags`.
- **Formats.** Day-one supported `format` values: `raw_text`, `markdown`, `html`, `chat_history`. The harness presents the sample's raw bytes to `lcs` over stdin; format affects labelling and reporting, not invocation.
- **Provenance.** Every sample carries enough metadata to reconstruct where it came from and under what licence it can be used. Imported samples preserve the original source's licence; synthetic samples record the local model that produced them and the seed `id`.
- **Validation.** A `shield-harness validate` subcommand verifies all sidecars parse, every sample's `expected_categories` are recognised by the currently installed `lcs` (via `lcs list`), no `id` collisions, and every text file referenced exists.

### 3.2 Runs

- **Engine matrix.** Default invocation runs all five `lcs` engine variants — `simple`, `yara`, `syara`, `syara-sbert`, `syara-llm` — over every sample. CLI flags allow restricting to a subset (`--engines simple,yara`), matching the option shape `lcs` itself uses.
- **Graceful degradation.** When an engine variant is unavailable (e.g. `syara-llm` because LMStudio is not reachable, or `syara-sbert` because ONNX runtime is missing), the harness records the variant as `skipped` with a reason and continues. A skipped engine never fails the run; it does mean its column is missing from the report.
- **Output.** Each run produces:
  1. A machine-readable run record (JSON), persisted under `runs/<timestamp>-<git-sha>/`.
  2. A human-readable summary printed to stdout.
  3. The per-sample raw `lcs` JSON outputs, captured for downstream diffing.
- **Reproducibility.** Run records pin: `lcs --version`, the git SHA of the harness, the git SHA of `llm_context_shield` if discoverable, and the corpus content hash.

### 3.3 Metrics

Priority order, lower-numbered items ship before higher-numbered:

1. Per-category precision / recall / F1, plus overall.
2. Wall-clock latency per sample (p50 / p95 / p99) and total throughput per engine.
3. Per-rule attribution — which rule names fired on which samples — derived from `lcs --log` output. Best-effort; degrades cleanly when log format changes.

Confusion matrices and false-positive exploration are roadmap (see §6).

### 3.4 Category vocabulary

The harness treats the set of detectable categories as **whatever the installed `lcs` reports**, not a hardcoded list. `validate` and `run` both consult `lcs list -e <engine>` to learn the current vocabulary and reject sidecars that reference unknown categories. This avoids drift when `lcs` adds or renames a scanner.

### 3.5 Diffing

- `shield-harness diff <baseline-run> [<other-run>]` produces a focused diff: samples whose verdict flipped, categories whose F1 moved by more than a threshold, engines whose p95 latency moved by more than a threshold. Defaults to comparing the latest two runs.
- The same machinery powers UC-3 (CI gate). The CI form of the command exits non-zero when configured thresholds are exceeded.

### 3.6 Synthetic generation

- A `shield-harness synth` subcommand connects to a configured OpenAI-compatible endpoint (default: `http://localhost:1234/v1`, LMStudio's default) and generates variants of selected seed samples.
- Generation strategies (v0.1): paraphrase a threat sample, paraphrase a clean sample. Richer strategies (compositional attacks, format transforms) are roadmap.
- Each generated sample is written as a normal corpus entry with `source = "synthetic"`, `seed_id = <originating id>`, and metadata identifying the local model used.

## 4. Non-Functional Requirements

### 4.1 External-only integration

`shield-harness` does not depend on the `llm_context_shield` Cargo crate. It invokes `lcs` as a subprocess and reads its stdout, stderr, exit code, and log files. This boundary is load-bearing: it forces the harness to measure what an integrator actually sees and prevents accidental coupling to internal types.

### 4.2 Determinism

For a fixed corpus content hash and a fixed `lcs --version`, repeated runs produce byte-identical metrics output (modulo wall-clock latencies, which are reported as percentiles). Sample iteration order is sorted by `id`. JSON keys are emitted in stable order.

### 4.3 Performance

The harness is not the bottleneck — `lcs` is. Per-sample overhead from harness bookkeeping is well under 10 ms. Samples may be scanned in parallel across engines and across samples; default parallelism is `num_cpus`, tunable via flag.

### 4.4 Privacy

The harness makes no network calls except (a) explicitly invoked by the operator (corpus importers fetching from named URLs), or (b) the LMStudio endpoint during `synth`. No corpus content, no run results, and no logs are sent anywhere by default.

### 4.5 Self-supporting

Per `CLAUDE.md`, dependencies are kept minimal. v0.1 starting set: `serde`, `serde_json`, `toml`, `clap`. Anything beyond that is discussed before adding. Dependencies follow the same N-1 / no-package-under-30-days rule as `llm_context_shield`.

### 4.6 Cross-platform

Runs on macOS and Linux. Windows support is best-effort (the harness itself is portable Rust, but `lcs --log` paths use XDG conventions that differ on Windows). Not a v0.1 requirement.

## 5. Out of Scope / Non-Goals

- **Modifying `lcs` rules from the harness.** Rule authoring lives in `llm_context_shield`. The harness reports; it does not write rules.
- **Hosting or distributing the corpus.** v0.1 corpus stays local. Public corpus split is a future possibility (see §6) — sample sidecars carry licence metadata so the split is mechanically feasible later.
- **Library / FFI integration with `lcs`.** Subprocess only. If introspection beyond logs is ever needed, the right move is to ask `lcs` to expose more in its JSON output, not to link the library.
- **A scoring algorithm of our own.** The harness compares `lcs`'s output to ground-truth labels. It does not invent its own threat scores or correlate findings.
- **Replacing `lcs`'s own test suite.** `llm_context_shield` has unit and integration tests of its own; this harness sits a layer above that, doing corpus-driven evaluation, not API-shape verification.
- **Real-time / streaming evaluation.** All runs are batch.
- **Adversarial-robustness research as a v0.1 deliverable.** Synthetic generation in v0.1 is a corpus-growth tool, not an adversarial mutation engine. The latter is roadmap.

## 6. Roadmap

The phase plan, sub-phase checklists, and review notes live in [`tasks/TODO.md`](tasks/TODO.md). This table is the high-level snapshot.

| Phase | Topic | Status |
|---|---|---|
| 0 | Project skeleton, PRD, ARCHITECTURE | 🚧 In progress |
| 1 | Corpus schema, validator, seed corpus from existing `lcs` `data/` and `tests/` fixtures | 📅 Next |
| 2 | Run pipeline: subprocess invocation, all-engines matrix, JSON record persistence | 📅 |
| 3 | Metrics: per-category P/R/F1, latency percentiles, human-readable summary | 📅 |
| 4 | Diff + CI gate (UC-2, UC-3) | 📅 |
| 5 | Importers: GitHub repo adapter, HuggingFace dataset adapter, academic-paper sample loader | 📅 |
| 6 | Synthetic generation via LMStudio (UC-5) | 📅 |
| 7 | Per-rule attribution from `lcs --log` parsing | 📅 |
| 8 | Confusion-matrix / false-positive explorer (D in REFINEMENT) | 📅 Future |
| 9 | Adversarial mutation engine (E in REFINEMENT) | 📅 Future |
| 10 | ReDB-backed corpus + read/query server subroutine | 📅 Future |
| 11 | Public corpus split (licence-filtered export) | 📅 Future |

## 7. Change log

- **2026-04-25** — Initial PRD created from `tasks/REFINEMENT.md` answers. Corpus is file-per-sample + sidecar TOML; integration is subprocess-only; default engine matrix is all five with graceful degradation; CI gate is v0.1.

# Continuity Notes

**Purpose:** working hand-off notes written before context compaction so a fresh session can pick up without losing fidelity. Update this file before any compact, and check it at the start of every new session.

This is **not** a planning document (that's `TODO.md`), **not** a vision document (`PRD.md`), and **not** an architecture document (`ARCHITECTURE.md`). It is a *briefing*: enough to bootstrap, no more.

---

## Last updated

2026-04-27 — end of Phase 2a. **`runner::invoke::scan` lands against lcs 0.5.3.** Types in `runner::scan_report` (`ScanReport`, `Finding`, `ThreatScores`) match the JSON exactly with all 11.5 fields required (no `Option<>`). Shared `runner::lcs::binary` resolver — `runner::introspect` refactored onto it. **39 tests pass** (was 25; +5 scan_report parse, +2 lcs resolver, +7 invoke including 7 live lcs 0.5.3 integration tests). Pin bumped to lcs ≥ 0.5.3 after Phase 2a probe surfaced + filed + fixed an upstream gap in one cycle. **Phase 2a bundle uncommitted in working tree.**

## Where we are

- **Phase 1 done; Phase 2a done.** Validator works end-to-end against the real seed cohort. `runner::invoke::scan` works end-to-end against lcs 0.5.3 — pipes stdin, captures stdout/stderr/exit/wall-clock latency, parses JSON into `ScanReport` for exit 0/1, returns `ScanError::Crashed` for exit 2.
- **Phase 2b (engine availability probe) is the next engineering work.** State machine in ARCH §5 against `lcs scan -e <eng> -f quiet` with constant input. Stderr-derived skip reasons. Live tests against the real lcs binary (no runtime skip — operator preference).
- **Operator-edit pass on seed samples is a separate, parallel track.** Doesn't block Phase 2b/2c/2d. See "Seed-cohort fire-rate" below.
- **Awaiting commit cycle.** The user controls commits and pushes. Do not commit, push, or open PRs without explicit instruction.

## What just landed (Phase 2a bundle, uncommitted)

**Upstream cycle:** Phase 2a probing of the real lcs 0.5.2 JSON surface revealed `threat_scores` was *missing* from `clean=true` responses across all three engines, contradicting the 11.5 spec's "always present" framing. Filed upstream; lcs 0.5.3 ships the fix (`threat_scores: {class_scores: {}, cumulative: 0}` on clean) with per-engine fingerprints unchanged. Harness pin moved 0.5.2 → 0.5.3. The "pause + file + resume" pattern from the memory note worked exactly as designed.

**Code:**
- `src/runner/scan_report.rs` — NEW. `ScanReport`, `Finding`, `ThreatScores` types matching lcs 0.5.3 JSON exactly. All 11.5 fields required (no `Option<>`). `serde::Deserialize`. Reuses `corpus::sample::Severity` (snake_case rename matches lcs lowercase strings). 5 parse tests covering clean response, single finding, multi finding, garbage input, missing-required-field rejection.
- `src/runner/lcs.rs` — NEW. `binary(Option<&Path>) -> PathBuf` shared resolver. Explicit path used verbatim; `None` falls back to `PathBuf::from("lcs")` for PATH lookup at exec time. 2 unit tests.
- `src/runner/invoke.rs` — `scan(sample_bytes, engine, lcs_path) -> Result<ScanOutcome, ScanError>`. Spawns `lcs scan -e <eng> -f json` with `Stdio::piped()` on all three streams. Writes sample bytes to stdin and drops it (closes EOF). Calls `wait_with_output()`. Exit 0 (clean) or 1 (threat) → parsed `ScanOutcome { report, exit_code, stderr, latency_ms, raw_stdout }`. Exit 2 → `ScanError::Crashed`. Other exit codes → `ScanError::UnexpectedExit`. Latency is wall-clock from `Instant::now()` at spawn through `wait_with_output()`. 7 integration tests (all live against lcs 0.5.3): LcsNotFound carries path; clean sample exit 0 + empty findings + cumulative=0; threat sample exit 1 + prompt_injection at `byte_range.0 == 0`; multi-finding syara with `cumulative > 0`; raw_stdout round-trips byte-for-byte through `serde_json::from_str` to equal `outcome.report`; latency under 60s; ParseFailed via `/bin/echo` stand-in (echo accepts stdin, exits 0, writes non-JSON — exact failure mode).
- `src/runner/introspect.rs` — refactored `binary()` helper out; now uses `crate::runner::lcs::binary`. Behaviour unchanged; 4 existing introspect tests still pass.
- `src/runner/mod.rs` — added `pub mod lcs;` and `pub mod scan_report;`.

No CLI surface changes. The `Run` subcommand stub is unchanged — Phase 2c/2d will wire `scan()` into the `run` flow.

**Doc updates:**
- ARCHITECTURE.md — §1 (two rows) and §12.1 pin line bumped to lcs ≥ 0.5.3; §14 changelog entry for 2026-04-27.
- PRD.md — §7 changelog entry for 2026-04-27 (pin bump + Phase 2a code drop).
- BACKLOG.md — "lcs version pinning ergonomics" updated for 0.5.3 reality.
- TODO.md — Phase 2 status row, Phase 2a checkboxes all `[x]` with details, active checklist (next = Phase 2b).
- CONTINUITY.md — this rewrite (Last updated, Where we are, this section, Current lcs surface, Active work, Files modified).

**Build state:** 39 tests pass. Eight `dead_code` warnings: 1 pre-existing (`Sample::read_bytes`), 7 new on the `runner::invoke`/`runner::scan_report` API surface (`ScanOutcome`, `ScanOutcome.stderr`, `ScanError`, `scan`, `ScanReport`, `Finding`, `ThreatScores`). Tests use them; `cargo build` doesn't compile tests, so the build sees them as unused. All seven resolve when Phase 2c/2d wires the orchestrator + run-record paths. Per project convention (memory: "Never suppress warnings"), they stay as forced reminders.

The earlier Phase 1b.5 and Phase 1c bundles are now both in git history.

## Seed-cohort fire-rate (informational, captured 2026-04-26 against lcs 0.5.2)

Out-of-band probe of every threat sample against every engine. **Not a Phase 1c gate** — validator exits 0 because the *vocab* is recognised; whether lcs *actually fires* is a separate question that v0.1 metrics will quantify in Phase 3.

| Sample | Expected category | simple | yara | syara |
|---|---|---|---|---|
| 001 | prompt_injection | ✓ | ✓ + refusal_suppression | ✓ + refusal_suppression |
| 002 | jailbreak | ✓ + prompt_injection | ✓ | ✓ |
| 003 | secret_probing | — | — | — |
| 004 | context_shift | — | — | — |
| 005 | data_exfiltration | — | — | — |
| 006 | obfuscation | hidden_content | hidden_content + refusal_suppression | refusal_suppression |

Two valid readings, to be resolved during the operator-edit pass:

1. **Detection gap** — patterns 003/004/005 are real attack shapes lcs misses; preserving them gives the harness genuine coverage signal (this is the harness's reason for existing).
2. **Sample drift** — these drafts are too oblique to trigger any rule; tighten their language to fire as labelled, so v0.1 metrics aren't dragged down by intentionally-undetectable threats.

Both readings have merit. The decision is per-sample, and the operator-edit pass owns it. **For Phase 2 (next), this matters not at all** — `runner::invoke` is engine-blind to sample quality.

## Current lcs surface (lcs 0.5.3, verified 2026-04-27)

The harness depends on this surface. Pin: lcs ≥ 0.5.3 (0.5.3 fixes the clean-response `threat_scores` omission that 0.5.2 shipped with).

| Surface | Sample invocation | Returns |
|---|---|---|
| Version | `lcs --version` | `lcs 0.5.3` |
| Engine list | (hardcoded) | three engines: `simple`, `yara`, `syara` |
| Category vocab | `lcs rules --categories -e <eng>` | one category per line. simple=6, yara=14, syara=15 (yara + `obfuscation`) |
| Threat-class vocab | `lcs rules --threat-classes -e <eng>` | one class per line (broader grouping than categories) |
| Fingerprint only | `lcs rules --fingerprint -e <eng>` | single hex line, SHA-256 of loaded rule set |
| Full rule manifest | `lcs rules --json -e <eng>` | `{fingerprint, rules[].{engine, name, category, severity, threat_class, version, threat_level, threshold}}` |
| Engine probe | `lcs scan -e <eng> -f quiet` (stdin = "hi") | exit code + stderr (skip reasons in stderr) |
| Main scan | `lcs scan -e <eng> -f json` (stdin = sample) | `{clean, finding_count, findings[].{category, severity, description, matched_text, byte_range, rule_name, engine}, rule_set_fingerprint, threat_scores.{class_scores, cumulative}}`. Exit 0 = clean, 1 = threat, 2 = error. **0.5.3 guarantees `threat_scores` is present even when `clean=true` (with empty `class_scores` and `cumulative: 0`); 0.5.2 omitted it on clean.** |

Per-engine fingerprints (2026-04-27, unchanged from 2026-04-26 captures — 0.5.3 was a JSON-shape fix, not a rule-set change):
- simple: `4c6cd18ac803ea92cb145a143b6e1629b30ee655e59afa6f60a65f150c11469a`
- yara: `c08cf011a8f298bc5564f731646fc99151243d85fb3a1778fc6ddcefe88dba7e`
- syara: `bb3ce91b0d6816f3676831c3f049f3c69a75425be727dae7467aff4d08f511c1`

These will change when lcs ships rule updates — that's the point of capturing them in `meta.json`.

## Key documents (source of truth, in priority order)

1. **`PRD.md`** — what we're building, why, for whom. Anchored to UC-1..UC-6. Note pre-existing drift items in changelog (§3.1 layout, §4.5 dep set, §6 status) — pending a future cleanup pass.
2. **`ARCHITECTURE.md`** — how it's built. §1 constraint table + §3 modules + §6 corpus model + §12.1 lcs contract are the most-touched references. §1 + §12.1 + §14 reflect the lcs 0.5.2 contract.
3. **`tasks/TODO.md`** — phase plan with checkboxes, "Done when" criteria, and `⏸ Pause for review` markers.
4. **`tasks/BACKLOG.md`** — speculative / deferred work (per-engine vocab narrowing, threat_scores aggregation, lcs version pinning, capability-tier per-engine `--lcs-path`).
5. **`tasks/REFINEMENT.md`** — the original PRD-refinement Q&A. Read once for grounding; not a working doc. (Note: REFINEMENT §6 still says "all five engines" — that's the original conversation, not authoritative. PRD §3.2 supersedes it.)
6. **`tasks/LESSONS.md`** — corrections to apply. Has one entry: "Probe the external CLI before designing against its documented behavior." Apply this lesson when designing against any new external tool.
7. **`tasks/BUGS.md`** — open bug log. Currently empty.
8. **`CLAUDE.md`** — project-level cognitive prefs and workflow rules. Always loaded.

## Decisions a fresh session WILL forget without this file

- **Subprocess-only integration with `lcs`.** Never link `llm_context_shield` as a Cargo dep. (PRD §4.1, ARCH §1.) The CLI surface is now rich enough (post-11.5) that no library introspection is needed — `lcs rules --json` plus the per-finding `rule_name`/`engine` plus top-level `rule_set_fingerprint`/`threat_scores` cover everything the harness needs.
- **Cohort abstraction is first-class.** Samples live at `samples/<cohort>/<verdict>/<id>.<ext>`. Every metric is sliced by cohort. The directory name MUST equal the sidecar `cohort` field. (ARCH §6.1, §6.4.)
- **Engine matrix is THREE engines** (`simple`, `yara`, `syara`). `syara-sbert` and `syara-llm` are *build features* of the `syara` engine, not separate `-e` values. Engine availability is probed (`lcs scan -e <eng> -f quiet`); unavailable engines are skipped, never failed. (ARCH §1, §5; PRD §3.2.)
- **Category vocabulary check is a real blocking validator** behind `--check-lcs-categories`. Probes simple/yara/syara, builds union vocab, rejects unknown categories blocking. Per-engine probe failures degrade to non-blocking notices (so a partial probe still validates). lcs binary entirely missing → exit 2. Default-off so `validate` works without lcs installed.
- **`Finding` JSON shape requires `rule_name` + `engine`.** `ScanReport` requires top-level `rule_set_fingerprint` + `threat_scores`. Don't model these as `Option<>` — lcs ≥ 0.5.3 is the contract (0.5.3 ships the `threat_scores`-on-clean fix that made the "no Option<>" stance defensible). If a pre-0.5.3 binary is encountered, fail loudly — `runner::scan_report` parsing will reject the missing field on the first clean response.
- **Capture all metadata.** Per-scan `outputs/<engine>.jsonl` preserves the full `ScanReport` verbatim (including `threat_scores`, even though v0.1 metrics ignore it). `meta.json` captures per-engine `rule_set_fingerprint` AND the full `lcs rules --json -e <eng>` output (rule manifest with severities, threat_levels, etc.). This avoids re-running the corpus when later metrics want a field.
- **Determinism is contractual.** Sort sample iteration by `(cohort, id)`. Sort outcomes by `(cohort, sample_id, engine_name)` before serialisation. Use `BTreeMap` for any map-typed serialised field. The `rule_set_fingerprint` from each engine is part of this — pin it into `meta.json`. (ARCH §10, §12.1.)
- **Synthetic samples never get auto-validated by `lcs`.** That conflates ground truth with the system under test. Operator decides what enters the corpus. (ARCH §9.)
- **`lcs --log` is now diagnostic-only.** Rule attribution comes from `findings[].rule_name`. `--log` is no longer on the critical path; if it stays, it's for ad-hoc debugging.

## Blessed dependency set (frozen — discuss before adding anything)

10 crates total. All exact-pinned (`=X.Y.Z`) in `Cargo.toml`.

| Crate | Pin | Notes |
|---|---|---|
| serde | =1.0.227 | with `derive` |
| serde_json | =1.0.148 | |
| toml | =1.1.0 | N-2 (1.1.1 was <30d old at pin time) |
| clap | =4.6.0 | with `derive` |
| sha2 | =0.10.9 | prior major; 0.11 just shipped |
| ureq | =3.2.1 | default features for now |
| chrono | =0.4.43 | `default-features = false`, features `clock` + `serde` |
| rayon | =1.11.0 | |
| csv | =1.3.1 | |
| time | (NOT in Cargo.toml) | blessed-but-dormant; add only if `chrono` falls short — see ARCH §13.2 |

Per-crate version-check rule (CLAUDE.md security): **N-1**, never anything <30 days old. Today's date matters; `time` is blessed but unpinned because we have no reason to use it yet.

## User preferences captured in memory (loaded automatically)

These are also in `~/.claude/projects/-Users-john-code-shield-harness/memory/` and loaded as system-level memory on every new session. Listed here for visibility:

- **Never suppress warnings.** No `#[allow(dead_code)]`, no equivalents. Warnings are forced reminders of unfinished work. The current build has 1 `dead_code` warning (`Sample::read_bytes`) that resolves when Phase 2a lands. Leave it.
- **User writes poetry.** Mythopoetic + scientific imagery is a deliberate style. Respond with specific reading, not generic praise. Don't push poetic phrasing into code, comments, or commit messages.
- **Pause downstream work when an upstream gap will be fixed.** This is exactly what just paid off — pausing Phase 2 for 11.5 avoided forward-compat scaffolding.
- (See `MEMORY.md` for the canonical index.)

## Conventions actively in force

- **Commit cycle:** user-controlled. Pause for review at every `⏸` marker in TODO.md. No commits, pushes, or PRs without explicit instruction.
- **Tests:** real, no mocks-of-the-thing-under-test, no always-true assertions. Every bug fix gets a test. (CLAUDE.md §4.)
- **Lessons:** every user correction → entry in `tasks/LESSONS.md`. (CLAUDE.md §3.)
- **File size:** target < 500 LOC per file; pause and ask if a file is going to exceed.
- **Dep additions:** any crate not in the blessed set above requires explicit discussion before being added.
- **Upstream gaps get filed, not papered over.** The harness's reason-for-being is to surface lcs usability problems. When you find one, file a task in `../llm_context_shield/tasks/` (BACKLOG.md for speculative, todo.md for actionable) rather than working around it in our code. The 11.5 cycle is the proof case.

## Active work

**Phase 2a just landed — uncommitted bundle in working tree.** Cleanest state for the next session is at the Phase 2a boundary, post-commit.

Single path when ready:

- **Phase 2b (`runner::probe::probe_engines`).** State machine in ARCH §5 against `lcs scan -e <eng> -f quiet` with constant input (`"hi"` per ARCH). Stderr-derived skip reasons (feature missing, ONNX runtime missing, LMStudio unreachable). Returns `Vec<EngineStatus>` — Available / Skipped(reason). Live integration tests against the real lcs binary (no runtime skip — operator preference). Should reuse `runner::lcs::binary` resolver and (probably) factor a small `Command::new(...).args(["scan", "-e", ...])` builder shared with `runner::invoke` if the duplication grows enough to matter.

Two parallel-track items don't block 2b:

- **Operator-edit pass on seed samples.** User-owned. Decide per-sample whether to tighten language so threats fire as labelled, or preserve as detection-gap signal (see Seed-cohort fire-rate above). Independent of the run pipeline.
- **PRD drift cleanup.** Documentation hygiene; flagged in BACKLOG.md.

## In-flight questions / things to raise

- **`threat_scores` aggregation strategy.** Captured raw in `outputs/<engine>.jsonl` per scan. Phase 3 metrics could surface `cumulative` drift per (cohort, engine) as a complementary signal to F1, and `class_scores` could power a per-threat-class breakdown distinct from per-category. Decide during Phase 3 design. Logged in `BACKLOG.md`.
- **Per-engine vocab narrowing.** Today's union check accepts any category from any engine. Richer: warn when a sample claims a category its target engines (`--engines` filter or default matrix intersection) can't emit. Logged in `BACKLOG.md`.
- **lcs version pinning / ergonomics.** Should the harness probe `lcs --version` at startup and warn (or fail) on lcs < 0.5.2? Or rely on the first `lcs rules` call to fail naturally? Logged in `BACKLOG.md`.
- **PRD drift cleanup pass.** Items deferred from the 2026-04-25 PRD edit: §3.1 sample-path layout still missing `<cohort>` segment, §3.1 sidecar field list missing `cohort`, §4.5 dep-set list outdated (says 4 crates, actual 9), §6 phase-status table now in sync with the renumber but the row narrative hasn't been re-read. Bundle in a future PRD cleanup pass when convenient.

## Files modified during the most recent session (Phase 2a, uncommitted)

**Code (NEW or MODIFIED):**
- `src/runner/scan_report.rs` — NEW. `ScanReport` / `Finding` / `ThreatScores` types matching lcs 0.5.3 JSON exactly. 5 parse tests.
- `src/runner/lcs.rs` — NEW. `binary(Option<&Path>) -> PathBuf` shared resolver. 2 unit tests.
- `src/runner/invoke.rs` — REWRITTEN (was a stub). `scan(sample_bytes, engine, lcs_path) -> Result<ScanOutcome, ScanError>`. 7 live integration tests against lcs 0.5.3.
- `src/runner/introspect.rs` — REFACTORED. Removed local `binary()`; now uses `crate::runner::lcs::binary`. Updates `LcsNotFound.path` to use `PathBuf::display()`. 4 existing tests still pass.
- `src/runner/mod.rs` — added `pub mod lcs;` and `pub mod scan_report;`.

No `Cargo.toml`, no CLI surface, no fixture, no test-helper changes. `Run` subcommand stub unchanged.

**Documentation:**
- `ARCHITECTURE.md` — §1 (two rows) and §12.1 pin line bumped 0.5.2 → 0.5.3; §14 changelog entry added for 2026-04-27.
- `PRD.md` — §7 changelog entry added for 2026-04-27.
- `tasks/BACKLOG.md` — "lcs version pinning ergonomics" entry updated for 0.5.3 reality.
- `tasks/TODO.md` — Phase 2 status row, Phase 2a section (all checkboxes `[x]` with detail), active checklist updated to point at Phase 2b.
- `tasks/CONTINUITY.md` — this rewrite.

For the Phase 1b.5 and Phase 1c bundles (now both in git history), see git log.

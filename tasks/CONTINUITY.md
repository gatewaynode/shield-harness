# Continuity Notes

**Purpose:** working hand-off notes written before context compaction so a fresh session can pick up without losing fidelity. Update this file before any compact, and check it at the start of every new session.

This is **not** a planning document (that's `TODO.md`), **not** a vision document (`PRD.md`), and **not** an architecture document (`ARCHITECTURE.md`). It is a *briefing*: enough to bootstrap, no more.

---

## Last updated

2026-04-26 — end of Phase 1b.5. **lcs Phase 11.5 landed in lcs 0.5.2.** Validator now wires real category-vocabulary checking via `lcs rules --categories`. Phase 7 (per-rule attribution from `lcs --log`) deleted as obsolete; phases 8–11 renumbered to 7–10. Phase 2 unblocked.

## Where we are

- **Phases 1a + 1b + 1b.5 complete.** Loader + validator (with real lcs-introspection-backed vocab check) wired and tested. **25 unit tests green** (was 18; +4 introspect, +3 vocab, +2 display, −1 obsolete pending notice). `cargo run -- --samples-dir <fixture> validate --check-lcs-categories` works end-to-end with correct exit codes (0 / 1 / 2) against real lcs 0.5.2.
- **Phase 1c (seed cohort, ≥6 clean + ≥6 threat hand-written samples) is the only remaining 1.x work.** Operator-paced, not engineering-heavy.
- **Phase 2 (run pipeline) is unblocked** and can start at any time. The 11.5-dependent fields (`rule_set_fingerprint`, `findings[].rule_name`, `findings[].engine`, `threat_scores`) should be modelled as required (not `Option<>`) since lcs ≥ 0.5.2 is the contract. See ARCH §12.1 for the full invocation surface.
- **Awaiting commit cycle.** The user controls commits and pushes. Do not commit, push, or open PRs without explicit instruction.

## What just landed (Phase 1b.5 bundle)

Single commit-worthy bundle resolving the lcs 11.5 dependency:

- **New code:** `src/runner/introspect.rs` — `probe_categories(lcs_path, engine)` wraps `lcs rules --categories -e <engine>`. `ProbeError` distinguishes `LcsNotFound` (binary missing → blocking), `EngineUnavailable` (engine-level skip → notice), `ParseFailed` (defensive). Pure parser is unit-testable; live probe exercised via the validator's smoke test.
- **Validator rewrite:** `IssueKind::LcsCategoryCheckPending` → `IssueKind::UnknownCategory { name }` (blocking) + `IssueKind::LcsProbeFailed { engine, reason }` (notice). `Options::check_lcs_categories: bool` → `Options::category_vocabulary: Option<BTreeSet<String>>`. CLI `run()` probes the three engines, builds union, populates vocab. lcs binary entirely missing → exit 2 with clear message.
- **Phase 7 deletion:** `src/runner/log_scrape.rs` removed; `pub mod log_scrape;` removed from `runner/mod.rs`; `--attribute-rules` flag removed from `RunArgs`; the byte-offset log-scrape narrative deleted from ARCH §12.1.
- **Fixture + tests:** `tests/fixtures/validate-cases/unknown-category/` (1 sidecar pair). 3 vocab-behaviour tests (None skips; Some accepts known; Some rejects unknown), 2 display tests (UnknownCategory; LcsProbeFailed), introspect parser tests (4).
- **Documentation:** PRD §1 (subprocess framing reworded), §3.3 (rule attribution from JSON not log), §3.4 (vocab now wired), §6 (renumbered), §7 (changelog entry). ARCH §1 (vocab + subprocess rows updated, fingerprint mention added), §3 (introspect replaces log_scrape), §4 (sequence diagram updated), §6.2 (class diagram fields added), §7 (run record narrative updated), §12.1 (full lcs CLI contract table rewritten), §14 (changelog entry). TODO.md (status snapshot, Phase 1b.5 done section, Phase 2 unpaused, Phases 7–10 renumbered, active checklist).
- **Build state:** 1 expected `dead_code` warning remains (`Sample::read_bytes` — resolves when Phase 2 wires `runner::invoke`). `Common::lcs_path` is now used by the validator probe; that warning is gone.

## Current lcs surface (lcs 0.5.2, verified 2026-04-26)

The harness depends on this surface. Pin: lcs ≥ 0.5.2.

| Surface | Sample invocation | Returns |
|---|---|---|
| Version | `lcs --version` | `lcs 0.5.2` |
| Engine list | (hardcoded) | three engines: `simple`, `yara`, `syara` |
| Category vocab | `lcs rules --categories -e <eng>` | one category per line. simple=6, yara=14, syara=15 (yara + `obfuscation`) |
| Threat-class vocab | `lcs rules --threat-classes -e <eng>` | one class per line (broader grouping than categories) |
| Fingerprint only | `lcs rules --fingerprint -e <eng>` | single hex line, SHA-256 of loaded rule set |
| Full rule manifest | `lcs rules --json -e <eng>` | `{fingerprint, rules[].{engine, name, category, severity, threat_class, version, threat_level, threshold}}` |
| Engine probe | `lcs scan -e <eng> -f quiet` (stdin = "hi") | exit code + stderr (skip reasons in stderr) |
| Main scan | `lcs scan -e <eng> -f json` (stdin = sample) | `{clean, finding_count, findings[].{category, severity, description, matched_text, byte_range, rule_name, engine}, rule_set_fingerprint, threat_scores.{class_scores, cumulative}}`. Exit 0 = clean, 1 = threat, 2 = error |

Today's per-engine fingerprints (2026-04-26):
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
- **`Finding` JSON shape requires `rule_name` + `engine`.** `ScanReport` requires top-level `rule_set_fingerprint` + `threat_scores`. Don't model these as `Option<>` — lcs ≥ 0.5.2 is the contract. If a pre-0.5.2 binary is encountered, fail loudly.
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

**Phase 1b.5 just landed — single bundle ready for commit when the operator says go.** No code in flight beyond that. Cleanest state is at the Phase 1b.5 boundary.

Two paths when ready:

1. **Phase 1c (seed cohort).** Operator-paced hand-curation. ≥6 clean + ≥6 threat hand-written samples covering 4 formats × ≥4 categories under `samples/seed-handcurated/`; `cargo run -- validate --check-lcs-categories` exits 0 on it. Validator + vocab check both work today.
2. **Phase 2a (`runner::invoke`).** First real scan-pipeline code. Define `ScanReport` + `Finding` types matching the lcs 0.5.2 JSON exactly (required fields, not `Option<>`). Wire `--lcs-path` resolution. Pipe stdin, capture stdout/stderr/exit/latency. Tests gated on lcs availability.

The user's preference (2026-04-25) was to pause for clean restart when 11.5 lands. That's now done. Decision on which of 1c/2a comes next is theirs.

## In-flight questions / things to raise

- **`threat_scores` aggregation strategy.** Captured raw in `outputs/<engine>.jsonl` per scan. Phase 3 metrics could surface `cumulative` drift per (cohort, engine) as a complementary signal to F1, and `class_scores` could power a per-threat-class breakdown distinct from per-category. Decide during Phase 3 design. Logged in `BACKLOG.md`.
- **Per-engine vocab narrowing.** Today's union check accepts any category from any engine. Richer: warn when a sample claims a category its target engines (`--engines` filter or default matrix intersection) can't emit. Logged in `BACKLOG.md`.
- **lcs version pinning / ergonomics.** Should the harness probe `lcs --version` at startup and warn (or fail) on lcs < 0.5.2? Or rely on the first `lcs rules` call to fail naturally? Logged in `BACKLOG.md`.
- **PRD drift cleanup pass.** Items deferred from the 2026-04-25 PRD edit: §3.1 sample-path layout still missing `<cohort>` segment, §3.1 sidecar field list missing `cohort`, §4.5 dep-set list outdated (says 4 crates, actual 9), §6 phase-status table now in sync with the renumber but the row narrative hasn't been re-read. Bundle in a future PRD cleanup pass when convenient.

## Files modified during the most recent session (Phase 1b.5)

**Code & fixtures:**
- `src/runner/introspect.rs` — NEW. probe_categories + ProbeError + parser unit tests.
- `src/runner/log_scrape.rs` — DELETED.
- `src/runner/mod.rs` — `pub mod log_scrape` → `pub mod introspect`.
- `src/cli.rs` — removed `attribute_rules` field from `RunArgs`.
- `src/corpus/validate.rs` — `LcsCategoryCheckPending` → `UnknownCategory` + `LcsProbeFailed`. `Options.check_lcs_categories` → `Options.category_vocabulary`. `run()` does live probe of simple/yara/syara. New per-sample category check loop.
- `tests/fixtures/validate-cases/unknown-category/foo/threat/0101.{toml,txt}` — NEW fixture.

**Documentation:**
- `PRD.md` — §1 subprocess framing, §3.3 rule attribution, §3.4 vocab check now wired, §6 phase table renumbered, §7 changelog entry.
- `ARCHITECTURE.md` — §1 (two rows), §3 (introspect replaces log_scrape), §4 (sequence diagram), §6.2 (class diagram), §7 (run record narrative), §12.1 (lcs CLI contract table rewritten + log-scrape narrative deleted), §14 (changelog).
- `tasks/CONTINUITY.md` — this rewrite.
- `tasks/TODO.md` — status snapshot, Phase 1b.5 done section, Phase 2 unblocked, Phase 7 deleted, Phases 8–11 → 7–10, active checklist.
- `tasks/BACKLOG.md` — seeded with 4 deferred items (per-engine vocab narrowing, threat_scores aggregation, lcs version pinning, capability-tier per-engine `--lcs-path`).

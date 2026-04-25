# Continuity Notes

**Purpose:** working hand-off notes written before context compaction so a fresh session can pick up without losing fidelity. Update this file before any compact, and check it at the start of every new session.

This is **not** a planning document (that's `TODO.md`), **not** a vision document (`PRD.md`), and **not** an architecture document (`ARCHITECTURE.md`). It is a *briefing*: enough to bootstrap, no more.

---

## Last updated

2026-04-25 — end of Phase 0 (project skeleton). About to start Phase 1a.

## Where we are

- **Phase 0 complete.** Project skeleton, Cargo.toml, CLI shell, all module stubs in place. `cargo build` succeeds. `cargo run -- --help` enumerates all six subcommands.
- **Phase 1a is next.** Schema & loader for the corpus. See `tasks/TODO.md` "Phase 1 — Sub-phase 1a" for the checklist.
- **Awaiting commit cycle.** The user controls commits and pushes. Do not commit, push, or open PRs without explicit instruction.

## Key documents (source of truth, in priority order)

1. **`PRD.md`** — what we're building, why, for whom. Anchored to UC-1..UC-6.
2. **`ARCHITECTURE.md`** — how it's built. 14 sections, 8 Mermaid diagrams. Read §6 (corpus / data model) and §3 (module structure) before touching Phase 1a.
3. **`tasks/TODO.md`** — phase plan with checkboxes, "Done when" criteria, and `⏸ Pause for review` markers.
4. **`tasks/REFINEMENT.md`** — the original PRD-refinement Q&A. Read once for grounding; not a working doc.
5. **`tasks/LESSONS.md`** — corrections to apply. Currently empty.
6. **`tasks/BUGS.md`** — open bug log. Currently empty.
7. **`CLAUDE.md`** — project-level cognitive prefs and workflow rules. Always loaded.

## Decisions a fresh session WILL forget without this file

- **Subprocess-only integration with `lcs`.** Never link `llm_context_shield` as a Cargo dep. Per-rule attribution is recovered from `lcs --log` output, not library introspection. (PRD §4.1, ARCH §1.)
- **Cohort abstraction is first-class.** Samples live at `samples/<cohort>/<verdict>/<id>.<ext>`. Every metric is sliced by cohort. The directory name MUST equal the sidecar `cohort` field. The synthetic-vs-real distinction is just one cohort axis. (ARCH §6.1, §6.4.)
- **Engine availability is probed, never assumed.** The harness asks `lcs scan -e <eng> -f quiet` against `"hi"` before the run. Unavailable engines disappear from the report; they never fail it. All five engines (`simple`, `yara`, `syara`, `syara-sbert`, `syara-llm`) are in the day-one matrix; the user's local LMStudio is set up for `syara-llm`. (ARCH §5.)
- **Category vocabulary belongs to `lcs`, not the harness.** Don't hardcode the 15 categories. Query `lcs list -e <engine>` at run start. (ARCH §3.4.)
- **Determinism is contractual.** Sort sample iteration by `(cohort, id)`. Sort outcomes by `(cohort, sample_id, engine_name)` before serialisation. Use `BTreeMap` for any map-typed serialised field. Latency reported only as p50/p95/p99 in metrics; raw values stay in `outputs/*.jsonl`. (ARCH §10.)
- **Per-rule attribution is opt-in and serialises the run.** Log scraping uses byte-offset deltas against the operator's real `~/.local/state/llm_context_shield/`. Concurrent workers would interleave log writes, so `--attribute-rules` forces `--jobs 1`. (ARCH §12.1.)
- **Synthetic samples never get auto-validated by `lcs`.** That conflates ground truth with the system under test. Operator decides what enters the corpus. (ARCH §9.)
- **`lcs --log` writes go to the operator's normal state dir.** We do NOT override `XDG_STATE_HOME` (the user explicitly preferred this).

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

- **Never suppress warnings.** No `#[allow(dead_code)]`, no equivalents. Warnings are forced reminders of unfinished work. Phase 0 produces 9 dead_code warnings on stub types — they're expected and self-resolving as later phases land. Leave them.
- **User writes poetry.** Mythopoetic + scientific imagery is a deliberate style. Respond with specific reading, not generic praise. Don't push poetic phrasing into code, comments, or commit messages.
- (See `MEMORY.md` for the canonical index.)

## Conventions actively in force

- **Commit cycle:** user-controlled. Pause for review at every `⏸` marker in TODO.md. No commits, pushes, or PRs without explicit instruction.
- **Tests:** real, no mocks-of-the-thing-under-test, no always-true assertions. Every bug fix gets a test. (CLAUDE.md §4.)
- **Lessons:** every user correction → entry in `tasks/LESSONS.md`. (CLAUDE.md §3.)
- **File size:** target < 500 LOC per file; pause and ask if a file is going to exceed.
- **Dep additions:** any crate not in the blessed set above requires explicit discussion before being added.

## Active work (pick up here after compact)

**Phase 1a — Schema & loader** (see `tasks/TODO.md` for the checklist).

Concrete next steps:
1. Implement `corpus::loader::load_corpus(root) -> Result<Vec<Sample>>` walking `samples/<cohort>/<verdict>/`.
2. Sort output by `(cohort, id)`.
3. Pair every text file with its sidecar TOML, parse the sidecar via the existing `Sidecar` struct in `corpus/sample.rs`.
4. Tests: build a fixture corpus directory under `tests/fixtures/` and assert ordering + field round-trip.

`Sidecar` and `Sample` types already exist in `src/corpus/sample.rs` from Phase 0 — do not redefine them.

## In-flight questions / things to raise

None at the moment. The dependency set is frozen, the architecture is settled, the cohort abstraction is approved.

## Files modified during the most recent session

- `CLAUDE.md` (Phase 0 setup)
- `PRD.md` (created)
- `ARCHITECTURE.md` (created)
- `Cargo.toml` (deps wired up)
- `.gitignore` (added /runs/, /baselines/)
- `src/main.rs` (replaced Hello World)
- `src/cli.rs` (created)
- `src/{corpus,runner,metrics,report,synth,import,util}/*.rs` (22 stubs)
- `src/diff.rs`
- `tasks/TODO.md` (created, Phase 0 marked done)
- `tasks/REFINEMENT.md` (Q&A captured)
- `tasks/LESSONS.md` (header)
- `tasks/BUGS.md` (header)
- `tasks/CONTINUITY.md` (this file)
- Memory: `~/.claude/projects/-Users-john-code-shield-harness/memory/{MEMORY.md,user_poetry.md,feedback_warnings.md}`

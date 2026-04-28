# `shield-harness` — Backlog

Speculative / deferred work. Items here are deliberately *not* on the active phase plan in `TODO.md`; they're parked here so they stop occupying memory but can be retrieved. Promote to `TODO.md` when ready.

Each entry: **what**, **why** (the trigger or motivation), **when to revisit** (rough condition or phase boundary).

---

## Per-engine vocabulary narrowing

**What:** today's `--check-lcs-categories` builds the *union* of `lcs rules --categories -e <eng>` across all three engines and accepts any category from any engine. Richer behaviour: when `--engines` is set (or when only a subset of engines probed successfully), warn (or block) on sidecar `expected_categories` entries that are valid in *some* engine's vocabulary but not in the engines this run will actually use. Concrete example: a sample claiming `expected_categories = ["context_shift"]` is impossible to detect when the run is restricted to `simple` (which doesn't emit `context_shift`); union-check passes today, per-engine narrowing would flag it.

**Why:** the union check catches typos and renames, but it doesn't catch "wrong tool for the job." Real corpus-curation mistakes will include both kinds.

**When to revisit:** after the seed cohort exists (Phase 1c) and after the first end-to-end runs (Phase 2 done) — that's when we'll see whether the false-friends pattern actually shows up in practice. May want both an interactive `validate --engines simple,yara` mode and a `run`-time pre-check.

---

## `threat_scores` aggregation strategy

**What:** lcs 0.5.2 adds a `threat_scores: {class_scores: {<class>: int}, cumulative: int}` block to every `ScanReport`. v0.1 metrics (P/R/F1, latency) ignore it. The data is captured raw in `outputs/<engine>.jsonl`. Decide what (if anything) to surface in `metrics.csv` / `summary.txt`. Candidates: cumulative-score drift per (cohort, engine) as a sensitivity signal complementing F1; per-threat-class breakdown distinct from per-category (threat-class is broader — e.g. `social_engineering` covers multiple categories).

**Why:** capturing the data was free (we're already preserving the full ScanReport). Surfacing it well takes thought.

**When to revisit:** Phase 3 design. If `threat_scores` discriminates regressions that F1 misses, promote to a v0.1 metric; otherwise leave the raw data and let downstream tooling consume it.

---

## lcs version pinning ergonomics

**What:** the harness assumes lcs ≥ 0.5.3 (Phase 11.5 surface + the 0.5.3 clean-response `threat_scores` fix). Today, a pre-0.5.3 binary fails noisily on the first `lcs rules` call or on the first parse of a clean `ScanReport`. Cleaner: probe `lcs --version` at startup and emit a clear "this harness requires lcs ≥ 0.5.3; you have lcs <X>" message before any subcommand work. Possibly also a `--no-lcs-check` escape hatch for environments without lcs at all.

**Why:** the failure mode today is fine for someone who reads stderr carefully but isn't great for first-time users.

**When to revisit:** before the first external user. Also worth considering whether `validate` should probe `--version` even without `--check-lcs-categories` and warn — but this risks `validate` becoming useless in lcs-less environments. Tied to the broader question of how strictly the harness wants to gate on lcs presence.

---

## Capability-tier per-engine `--lcs-path`

**What:** today `--lcs-path` selects one lcs binary used for all three engines. A more rigorous benchmark would let the operator drive multiple lcs binaries in one run — one per build-feature tier (e.g. `lcs-base`, `lcs-yara`, `lcs-syara-sbert`, `lcs-syara-llm`) — and measure the marginal benefit of each capability. CLI surface: `--lcs-path-simple`, `--lcs-path-yara`, `--lcs-path-syara` (or a config file).

**Why:** answers the question "is enabling syara-llm worth its latency cost?" with concrete numbers from the operator's own corpus.

**When to revisit:** out of v0.1 scope. Reconsider after Phase 3 metrics exist and when the operator has an actual decision to make about which capability tier to deploy.

---

## PRD drift cleanup pass

**What:** several pre-existing PRD-vs-code drift items deferred from the 2026-04-25 edit: §3.1 sample-path layout missing `<cohort>` segment, §3.1 sidecar field list missing `cohort`, §4.5 dep-set list outdated (says 4 crates, actual 9), §6 phase-status table partially stale (the table-row narratives are pre-renumber even after the table itself was updated).

**Why:** drift accretes; periodic cleanups are cheaper than letting it compound. Nothing here is broken — these are documentation hygiene items.

**When to revisit:** bundle with the next substantive PRD/ARCH update so it's all one pass. Or flag for a quick targeted PR once Phase 1c lands.

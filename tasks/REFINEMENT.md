# PRD Refinement Questions — `shield-harness`

Created: 2026-04-25

Answer inline under each question. Silence on a question = accept the listed *Default*.
Once this is settled, `PRD.md` is written from the answers, then `ARCHITECTURE.md` is derived.

---

## 1. Integration surface to `llm_context_shield`

How does the harness invoke the scanner?

- (a) Subprocess `lcs scan -f json` only — clean separation, version-pinnable, slower per sample.
- (b) Cargo library dependency on `llm_context_shield` — faster, in-process, can introspect scoring internals.
- (c) Both, switchable per run.

**Default:** (c) — subprocess for end-to-end realism + library for fine-grained per-rule attribution.

**Answer:** Intentionally run as subprocess only.  Visibility to internal scoring will have to be through logs.

---

## 2. Corpus provenance

Where do samples come from?

- (a) Ingest / mirror `llm_context_shield`'s existing `data/` and `tests/` fixtures.
- (b) Build an independent corpus from scratch.
- (c) Keep them separate but cross-reference.

Also: pull in public collections (garak, Lakera Gandalf, PromptBench, DAN repos, etc.), or stay hand-curated?

**Default:** (c) cross-reference; pull in 1–2 public collections after the schema is settled.

**Answer:** We are going to build our corpus from mixed sources, starting small with some samples from Github repos and some academic papers that are free to use, and then adding from other aggregations like Huggingface data sets and freely available conversation data sets.  And then we are going to synthesize our own variants using local LLMs serverd out of LMstudio (to avoid getting caught in automated scanners at Anthropic with grey area material).

---

## 3. Sample storage & label schema

Storage layout: file-per-sample with sidecar (e.g., `samples/bad/0001.txt` + `0001.toml`)? Single JSONL? SQLite?

Minimum label set proposal:
- `id`
- `text_path`
- `verdict` (clean / threat)
- `expected_categories` (subset of the 15 scanner categories)
- `expected_min_severity`
- `source`
- `license`
- `notes`
- `format` (raw text / markdown / html / chat-history)

**Default:** file-per-sample + sidecar TOML, schema as above.

**Answer:** File per sample with a sidecar is a good start.  Maybe a longer term roadmap will include a DB (ReDB preferred) and a server subroutine.

---

## 4. Primary metrics (priority-ordered)

Which metrics matter, in what order?

- Detection accuracy (precision / recall / F1) overall and per category.
- Latency (p50 / p95 / p99) and memory.
- Throughput (samples / sec).
- Per-engine comparison.
- Per-rule attribution (which rule fired, which rules false-positived).

**Default:** per-category P/R/F1 first; latency/throughput second; per-rule attribution third (requires library mode).

**Answer:** I think the default is fine.  Let's get some data sets, analyze and then see if improvements/refactoring are needed.

---

## 5. Iteration loop — what does "useful run output" look like?

- (a) A run report you eyeball.
- (b) Baselined regression diffs ("rule X regressed on 4 samples since last run").
- (c) CI gate (fail if F1 drops > N%).
- (d) Confusion-matrix / false-positive explorer.
- (e) Adversarial-mutation generator (take a true-positive, paraphrase it, check if detection holds).

**Default:** (a) + (b) for v0.1; (c) and (d) for v0.2; (e) explicitly out of scope for v0.1.

**Answer:** Each has its own value, "A, B, C" seem to be critical to being useful.  "D" and "E" should both be on the future state roadmap.

---

## 6. Engine matrix

Run all engine variants in one invocation and compare, or pick one per invocation?

Which variants are in scope on day one: `simple`, `yara`, `syara` (string-only), `syara-sbert`, `syara-llm`?

**Default:** all-in-one comparison; day-one targets `simple` + `yara` + `syara` (+ `syara-sbert` if local ONNX is wired up); `syara-llm` deferred since it needs an OpenAI-compatible endpoint.

**Answer:**  We should have the ability to follow the options in llm_context_shield and run individual engines, but a default run should contain all five.  We currently have a local LMstudio server setup with several models to test for syara-llm.

---

## 7. Self-supporting / dependency posture

Per CLAUDE.md "Self Supporting" rule, the harness should minimize deps. Acceptable starting set: `serde` + `serde_json` + `toml` + `clap`. Hand-roll CSV (trivial). Add nothing else without flagging.

**Default:** as above.

**Answer:** Those are reasonable crates to pull in to get started.  Always discuss when another crate seems needed.

---

## 8. Distribution

Internal-only tool, or eventual public release of the harness and/or the corpus? Affects licensing of imported samples.

**Default:** internal-only for now; design corpus structure so a public split is possible later.

**Answer:** Default is exactly what I was thinking.

---

## Open questions / additions from the user

(Use this section to raise dimensions I missed.)

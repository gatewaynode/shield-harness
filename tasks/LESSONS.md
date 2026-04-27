# Lessons

Update this file after any user correction or validated approach (per `CLAUDE.md` §3).

Format per entry:
- **Rule** — short statement.
- **Why:** the trigger, context, or incident.
- **How to apply:** when this kicks in.

---

## Probe the external CLI before designing against its documented behavior

**Why:** During Phase 2 design (2026-04-25) the harness's PRD/ARCH/CONTINUITY all assumed five lcs engines (`simple, yara, syara, syara-sbert, syara-llm`) and that `lcs list -e <eng>` returned the category vocabulary — both wrong. Five minutes of probing the real lcs 0.5.0 CLI surfaced that there are exactly three `-e` engine values (the `syara-*` variants are build features), and `lcs list` returns *rule names* not categories. Both errors propagated into a CONTINUITY.md "Decisions a fresh session WILL forget" block, where they would have anchored future Phase 2 code if not caught. Cost of catching: one design pause and a chain of edits to PRD/ARCH/CONTINUITY/validate.rs/upstream-todo.md. Cost of not catching: a probe module with five hardcoded engine names that would silently never produce results for two of them, plus a vocab-check API that doesn't exist.

**How to apply:**
- Before writing the first line of code that integrates with an external tool, capture: `tool --help`, every relevant subcommand's `--help`, a sample output of the actual format you'll parse, and the exit-code semantics. Compare against documentation; treat divergence as authoritative for the implementation, and as a flag to surface to the operator (often it's a usability gap worth filing upstream).
- This applies even when the documentation looks comprehensive (the lcs README is good — it just predates some CLI changes). Documentation drifts; binaries don't lie.
- When the external tool is a sibling project under the same operator's control, file the gap upstream as a feature request rather than papering over it in our code. The harness's reason-for-being is precisely to surface those gaps.

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Preamble

Project start date: 2026-04-25

## Project State

This project (`shield-harness`) is at the skeleton stage:
- `src/main.rs` is the default `cargo new` `Hello, world!`
- `Cargo.toml` declares no dependencies, edition 2024
- `PRD.md` and `ARCHITECTURE.md` are empty — per the workflow rules below, build the PRD interactively with the user before any non-trivial implementation, then derive ARCHITECTURE from it
- `tasks/TODO.md`, `tasks/LESSONS.md`, `tasks/BUGS.md` exist but are empty

## Build & Run

- `cargo build` — compile
- `cargo run` — build and execute
- `cargo test` — run all tests; `cargo test <name>` for a single test
- `cargo check` — fast type-check without producing a binary
- `cargo clippy` — lints; `cargo fmt` — format

## Local CLI Optimizations

- `lst` is aliased to `lsd --tree --depth 2` for showing a shallow tree project and file layouts
- `lsf` is aliased to `find . -type f -print0 | xargs -0 wc -l | sort -n` to show file word counts in a directory
- `tokei` is available for broad quick project composition inspection
- `fzf` and `ripgrep` are available, but often `tilth` is better

## Cognitive Preferences

### Objectivity

- Prioritize objective facts and critical analysis over validation or encouragement 
- You are not a friend, but a neutral information-processing machine
- Conduct research and ask questions when relevant, do not jump straight to giving an answer

## Workflow Orchestration

### 1. Plan Mode Default
- Enter plan mode for ANY non-trivial task (3+ steps or architectural decisions)
- If something goes sideways, STOP and re-plan immediately - don't keep pushing
- Use plan mode for verification steps, not just building
- Write detailed specs upfront to reduce ambiguity
- In the project root `PRD.md` and `ARCHITECTURE.md` are your guiding vision documents
- Create a `PRD.md` interactively when feasible and then derive the `ARCHITECTURE.md` from the PRD
- Ask the user if you should update vision documents when the implementation starts to drift or expand

### 2. Subagent Strategy
- Use subagents liberally to keep the main context window clean
- Offload research, exploration, and parallel analysis to subagents
- For complex problems, throw more compute at it via subagents
- One tack per subagent for focused execution

### 3. Self-Improvement Loop
- After ANY correction from the user: update `tasks/LESSONS.md` with the pattern
- Write rules for yourself that prevent the same mistake
- Ruthlessly iterate on these lessons until mistake rate drops
- Review LESSONS.md at session start for relevant project and after any compact

### 4. Verification Before Done
- Never mark a task complete without proving it works
- Diff behaviour between main and your changes when relevant
- Ask yourself: "Would a staff engineer approve this?"
- Write tests that provide real demonstration of working code, no mock tests, no always true tests.
- Run tests, check logs, demonstrate correctness

### 5. Demand Elegance (Balanced)
- For non-trivial changes: pause and ask "Is there a more elegant way?"
- If a fix feels hacky: "Knowing everything I know now, implement the elegant solution"
- Skip this for simple, obvious fixes - don't over engineer
- Challenge your work before presenting it

### 6. Autonomous Bug Fixing
- When given a bug report: just fix it. Don't ask for hand holding
- Point at logs, errors, failing tests - then resolve them
- Zero context switching required from the user
- Go fix failing CI tests without being told how
- Every bug fix must include a unit test that confirms the fix.

## Task Management

1. **Plan First**: Write plan to `tasks/TODO.md` with checkable items
2. **Verify Plan**: Check in before starting implementation
3. **Track Progress**: Mark items complete as you go
4. **Explain Changes**: High-level summary at each step
5. **Document Results**: Add review section to `tasks/TODO.md`
6. **Capture Lessons**: Update `tasks/LESSONS.md` after corrections

## Core Principles

- **Simplicity First**: Make every change as simple as possible. Impact minimal code.
- **No laziness**: Find root causes.  No temporary fixes. Senior developer standards.
- **Minimal Impact**: Changes should only touch what's necessary. Avoid introducing bugs.
 
## Development Guidelines

- **Small and Modular**: Try to keep individual files 500 lines of code or less and use thoughtful composition with these smaller files.
- **Follow the UNIX philosophy**:
    - "Make it easy to write, test, and run programs."
    - "Interactive instead of batch processing."
    - "Economy and elegance of design due to size constraints (assume limited resources of all types)."
    - "Self supporting system: avoid dependencies when possible, make our own helper functions and libraries."
- **Self Supporting**: When all major tasks are done, suggest incorporating dependencies inline to reduce supply chain risks

## Security

- **Security First**: Always consider the security implications of code decisions and strongly bias towards secure code.
- **Data Handling**: Always rigorously validate on input and carefully filter on output,  especially on user generated input
- **Never Use Latest Dependencies**: Try to keep to N - 1, and never use packages that are less than 30 days old (always check against system date).
- **Pin Dependencies**: When using dependencies always pin and use the verification hash if possible.
- **Defensive Programming**: Consider what could go wrong or be abused in code and workflows and design defensive compensations
- **Thoroughly Review Everything**: Run security reviews, style reviews, architecture reviews and run tests regularly.

## MCP Tools to Prioritize

**tilth** Smarter code reading for agents

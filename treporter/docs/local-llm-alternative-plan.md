# Monthly Reporting + Local LLM Transition Plan

## Goal
Improve monthly report quality when commit collection is incomplete/noisy by adding:
- deterministic data sanitization + optional translation
- project-structure context (5 major goals)
- a reliable local-LLM reporting flow with staged prompts

## Current Issues to Address
- `local-report` is wired in CLI but core modules are empty (`src/multistage_pipeline.rs`, `src/structured_extractor.rs`).
- `config.toml` filename placeholders don’t match generator replacement keys.
- `default_period` in config is not currently used by CLI date parsing.
- Reports can be generic because prompts are not explicitly grounded to project goals.

## Phase 1: Stabilize Baseline (Safe Commit First)
1. Commit only non-breaking changes.
2. Exclude placeholder/empty feature files from that commit.
3. Remove temporary debugging artifacts from tracked source where appropriate.

### Acceptance Criteria
- Build remains in known-good state after commit.
- No new commands are exposed that cannot execute.

## Phase 2: Data Sanitize + Normalize Layer
Add deterministic preprocessing before LLM calls in `src/preprocessor.rs`:
1. Remove noise:
   - bot-only comments
   - duplicate commit text
   - oversized low-signal text blocks
2. Normalize records into a compact schema:
   - `source`, `repo`, `branch`, `date`, `author`, `summary`, `files_changed`, `change_type`, `evidence`
3. Add optional translation step for selected text fields:
   - source language auto-detect (simple heuristic acceptable)
   - target language from config
4. Redact sensitive patterns (tokens, secrets, emails where needed).

### Acceptance Criteria
- Preprocessed output is <= configured token budget.
- Same input always yields same normalized output.
- Sensitive strings are removed/redacted in prompt payloads.

## Phase 3: Add Project Goals to Config + Prompt Context
Extend config model (`src/config.rs`) and `config.toml`:
1. Add a reporting section:
   - `report_language = "en"`
   - `major_goals = ["...", "...", "...", "...", "..."]`
2. Inject goals into monthly-report prompts in `src/llm.rs`.
3. Require explicit per-goal structure in prompt output:
   - progress evidence
   - blockers
   - next step
   - confidence

### Acceptance Criteria
- Every monthly report contains a section for each of the 5 goals.
- Missing evidence is explicitly stated as `No evidence in period`.

## Phase 4: Implement Minimal Working Local Multi-Stage Pipeline
Implement real structs and methods:
1. `src/structured_extractor.rs`
   - `StructuredDataExtractor::new()`
   - `extract(&CollectedData) -> StructuredData`
   - `to_json(&StructuredData) -> String`
2. `src/multistage_pipeline.rs`
   - `MultiStagePipeline::new(config)`
   - `generate_report(&CollectedData) -> MultiStageResult`
3. Keep stages simple and deterministic-first:
   - Stage 0: extraction/normalization
   - Stage 1: fact extraction prompt
   - Stage 2: goal mapping prompt
   - Stage 3: final monthly format prompt

### Acceptance Criteria
- `chronopulse local-report` runs end-to-end.
- `--export-structured` and `--export-stages` produce valid JSON.

## Phase 5: Align Config/Output/Date Behavior
1. Fix filename template mismatch:
   - use `{project_name}_{template_name}_{date}` or update replacement code consistently.
2. Wire `date_range.default_period` into CLI parsing.
3. Ensure branch selection (`default`/`all`/list) is reflected in commit collection logs.

### Acceptance Criteria
- Output filenames resolve without raw placeholders.
- Date range behavior matches config when no CLI dates are passed.

## Phase 6: Quality Gate for Monthly Reports
Add lightweight validation checks:
1. Report includes:
   - Executive summary
   - Metrics section
   - 5-goal mapping section
   - Risks/blockers
   - Next-month priorities
2. Reject/generate warning if:
   - no concrete evidence lines (commit/file refs)
   - excessive generic language
   - missing goals

### Acceptance Criteria
- Generated report passes structure checks automatically.
- At least one evidence line per active goal.

## Suggested Implementation Order
1. Phase 1
2. Phase 2
3. Phase 3
4. Phase 5
5. Phase 4
6. Phase 6

## Suggested Commit Sequence
1. `chore: stabilize config/docs and remove debug leftovers`
2. `feat(preprocessor): add sanitize normalize and optional translation`
3. `feat(reporting): add major goals config and prompt grounding`
4. `fix(config): align filename template and default period handling`
5. `feat(local-report): implement structured extractor and multistage pipeline`
6. `test(reporting): add monthly structure and evidence quality checks`

## Notes
- Prefer local deterministic transforms for reliability; use LLM for synthesis, not raw parsing.
- Keep prompt payloads compact and schema-driven.
- Keep local-model prompts single-task and explicit.
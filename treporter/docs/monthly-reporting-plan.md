# Monthly Reporting Improvement Plan

**Supersedes:** `local-llm-alternative-plan.md`, `local-llm-optimization.md`  
**Status:** Approved — implementation starting  
**Branch:** `feature/treporter-enhancements`

---

## Problem Statement

Commit collection works. Report quality does not. The outputs are generic because:

1. The synthesis prompt is hardcoded to "SPEAR project" with no config-driven context.
2. There is no goal-mapping step — the LLM must infer report structure entirely from raw data.
3. The preprocessor sends everything with no token budget or noise filter, overwhelming local models.
4. A single large LLM call handles synthesis — the worst pattern for local models.
5. `config.toml` has no `major_goals` field, so there is nowhere to express project structure.
6. `default_period` and `filename_template` are not wired into the CLI or generator.

The prior plans (`local-llm-alternative-plan.md`, `local-llm-optimization.md`) correctly
diagnose these issues. The optimization doc proposes a good multi-stage architecture but frames
it as a new parallel pipeline (`multistage_pipeline.rs`). The alternative plan correctly notes
that the existing `llm.rs` flow is what gets invoked. This plan merges both approaches:
**fix the existing flow in place first, then introduce the multi-stage pipeline as an
opt-in upgrade path, not a replacement**.

---

## Design Decisions

### D1 — Upgrade in place, not in parallel
The `LocalReport` command and its empty stubs (`multistage_pipeline.rs`,
`structured_extractor.rs`) currently break the build. Rather than rushing to fill them,
we first fix the existing `generate` path so it produces good output. The stubs become
the new entry point only after the quality improvements are proven in the existing flow.

### D2 — Themes and milestones are auto-extracted, not manually listed
You should not have to enumerate goals each month. Instead the pipeline reads two
optional markdown inputs and extracts structure automatically:

- **Prior month's plan/notes** (`prior_plan_path`) — your working notes, todo lists,
  planning docs. Used to seed theme detection with context about what you were focused on.
- **Prior month's generated report** (`prior_report_path`) — last month's output report.
  The extractor looks for milestone markers here: any heading, checklist item, or line
  containing keywords like `milestone`, `deliverable`, `target`, `due`, `complete`, or
  a date reference. Milestones found become first-class items in theme extraction.

```toml
[reporting]
report_language = "en"
token_budget = 8000
prior_plan_path = ""    # path to last month's notes/plan markdown; optional
prior_report_path = ""  # path to last month's generated report; optional
```

Both paths can be overridden at the command line:
```
--prior-plan path/to/notes.md
--prior-report path/to/last_month_report.md
```

When neither is provided, themes are extracted purely from activity data.
Theme selection is **fully automatic** — no interactive prompts.

### D3 — Token budget for large local models
For a large local model (llama3.1:70b, 128K context; deepseek:33b, 32K context),
a safe per-call input budget is **8000 tokens** (~32KB of text). This leaves ample
room for the system prompt and requested output while staying well within the context
window of any reasonably sized model.

- `8000` tokens: good default for 70B+ models.
- `4000` tokens: use this if hitting a 13B or smaller model.
- The budget applies to the *input payload* only; `max_tokens` in `[llm]` controls
  output length and can stay at 4000.

### D4 — Pipeline has a mandatory theme-extraction stage
Before synthesis, two focused LLM calls run in sequence:

**Stage A — Theme extraction:**
```
TASK: Read the planning notes and activity data below. Identify the 4–7 most
significant work themes or goals active during this period. For each theme,
write one sentence describing it and list 2–3 specific evidence items
(commit hash, issue ID, or filename) from the activity data.

PRIOR PLAN NOTES:
{prior_plan_text or "Not provided."}

ACTIVITY SUMMARY:
{sanitized_summary}

OUTPUT FORMAT (repeat for each theme):
THEME: <name>
DESCRIPTION: <one sentence>
EVIDENCE: <item1>, <item2>, <item3>
```

**Stage B — Per-theme evidence synthesis:**
For each extracted theme, one focused LLM call pulls together a paragraph of
evidence and status. Multiple small calls beat one giant call for local models.

Both stages have deterministic fallbacks: if the LLM call fails, theme names are
extracted by keyword frequency from commit messages (pure Rust).

### D5 — Preprocessor gets a token budget + noise filter
The existing `preprocessor.rs` categorizes and groups but does not remove noise or
enforce size limits. We add a deterministic `sanitize()` step that runs before any LLM
call. This is pure Rust, no LLM involved.

### D6 — Automation via `make monthly`
The full flow — collect → sanitize → extract themes → synthesize → validate → write file —
should be triggerable with a single command. The `Makefile` gains a `monthly` target
that wires the CLI flags with sensible defaults for the current month.

### D7 — De-inductive reasoning architecture
The pipeline is structured in two distinct epistemic phases that must not be mixed:

**Deductive phase (deterministic, Rust only):**
Collect facts, deduplicate, cluster by keyword frequency, extract milestone markers from
markdown, enforce token budget. This phase produces a *ground-truth artifact* — a
structured JSON that is entirely derived from verifiable source data. No LLM is involved.
Same input always yields same output.

**Inductive phase (LLM, bounded):**
Take the structured artifact from the deductive phase and refine it — name the clusters,
describe themes in prose, infer plan shifts, write the narrative. The LLM's job is
*synthesis and language*, not fact-finding. It is given only pre-clustered, pre-verified
evidence; it cannot invent items that weren't in the deductive output.

This separation is the core reliability guarantee: hallucinations are bounded because the
LLM cannot add facts, only labels and prose.

```
Deductive phase                     Inductive phase
──────────────────────             ──────────────────────────────────
collect                             extract_themes_and_milestones()
  → deduplicate                       RetryLoop (max 3, format-correcting)
  → drop bots                         → parse THEME/MILESTONE/PLAN_SHIFT
  → keyword-cluster commits           → fallback: keyword clusters as themes
  → extract milestone markers
  → enforce token budget
  → StructuredData (JSON)           per-theme synthesis()
                                      RetryLoop (max 3, format-correcting)
                                      → paragraph per theme
                                      → fallback: template fill from clusters

                                    final_synthesis()
                                      RetryLoop (max 3)
                                      → full report markdown
```

### D8 — Bounded self-refinement (RetryLoop)
Every LLM call in the inductive phase is wrapped in a `RetryLoop`:

```rust
RetryLoop {
    max_retries: 3,
    prompt_fn:   fn(prev_output: Option<&str>, error_feedback: Option<&str>) -> String,
    validate_fn: fn(&str) -> Result<T, String>,  // returns feedback string on failure
    fallback_fn: fn() -> T,                       // pure Rust, always succeeds
}
```

- On first attempt: `prompt_fn(None, None)`
- On retry: `prompt_fn(Some(prev_output), Some("Missing EVIDENCE for Theme: X"))` —
  the model sees its own previous output and a specific correction instruction.
- Retries target *format errors only*, not reasoning quality. The corrective prompt
  says what field is missing, not "think harder".
- After `max_retries` exhausted: `fallback_fn()` runs — pure Rust, guaranteed output.
- `validate_fn` is deterministic Rust (regex/string checks), never an LLM call.

### D9 — Three execution modes

| Flag | When to use | What it does |
|---|---|---|
| `--mode local` | Default | Full de-inductive pipeline with RetryLoop against local Ollama |
| `--mode prep` | Want best quality | Runs deductive phase only (no LLM); outputs clean `*_structured.json` ready to paste into any big model |
| `--mode cloud` | API key available | Deductive phase + single well-formed prompt sent to GPT-4/Claude API |

`prep` mode is a zero-LLM escape hatch: if local quality is poor, you get a dense
structured artifact in seconds that can be dropped into Claude.ai without re-running
collection.

---

## Phases

### Phase 0 — Stabilize the build (prerequisite)
Before any new work, the build must be clean.

**Changes:**
- Add minimal stub bodies to `multistage_pipeline.rs` and `structured_extractor.rs`
  just enough to compile (`pub struct` + empty `impl`).
- Leave `LocalReport` CLI command wired but gated:
  `println!("not yet implemented"); return Ok(());`

**Acceptance criteria:** `cargo build` passes with zero errors.

---

### Phase 1 — Add `[reporting]` section to config

**Files:** `src/config.rs`, `config.toml`

**Changes:**
- Add `ReportingConfig` struct:
  ```rust
  pub struct ReportingConfig {
      pub report_language: String,        // "en"
      pub token_budget: usize,            // 8000
      pub prior_plan_path: Option<String>,   // path to last month's notes/plan markdown
      pub prior_report_path: Option<String>, // path to last month's generated report
  }
  ```
- Add `reporting: ReportingConfig` field to top-level `Config`.
- Add `[reporting]` section to `config.toml` and `generate_default()`.

Note: **no static goals list** — themes and milestones are extracted dynamically (Phase 3).

**Acceptance criteria:**
- `Config::load()` parses the new section without error.
- Both prior-document paths are accessible from any module and override-able via CLI.

---

### Phase 2 — Deductive phase: sanitize, cluster, structure

**Files:** `src/preprocessor.rs`, `src/structured_extractor.rs`

This is the entire deductive phase (D7). Pure Rust, no LLM. Produces a
`StructuredData` JSON artifact that is the single input to all inductive-phase calls.

**Step 1 — Sanitize** (in `preprocessor.rs`):
1. **Deduplicate** commits across sources by hash.
2. **Drop bot noise** — author matches `dependabot`, `renovate`, or `[bot]`.
3. **Truncate long diffs** — per-commit diff capped at 600 chars; file list capped
   at 10 entries.
4. **Token budget enforcement** — if estimated tokens (`text.len() / 4`) exceed
   `config.reporting.token_budget`, drop oldest/lowest-signal commits first.

**Step 2 — Keyword cluster** (in `structured_extractor.rs`):
1. Tokenize all commit messages (split on whitespace/punctuation, lowercase, drop
   stopwords).
2. Score terms by tf-idf across the commit corpus.
3. Group commits into clusters by top shared terms (simple greedy assignment).
4. Each cluster gets a label (top 2–3 terms), a commit list, and a file-change list.
5. **Extract milestone markers** from `prior_report` markdown: headings and checklist
   items containing `milestone|deliverable|target|due|complete` (case-insensitive).

**Output** — `StructuredData`:
```json
{
  "metadata": { "project": "", "period": "", "sources": [] },
  "summary_metrics": { "total_commits": 0, "unique_authors": 0, "repos": [] },
  "commit_clusters": [
    { "label": "llm pipeline", "terms": ["llm","pipeline","ollama"],
      "commits": [...], "file_changes": [...] }
  ],
  "milestone_markers": [
    { "text": "Milestone: local-report end-to-end", "status_hint": "[ ]" }
  ]
}
```

Same input always yields same output.

**Acceptance criteria:**
- Two identical commits from GitLab and GitHub produce one cluster entry.
- Output JSON is never larger than `token_budget * 4` bytes after sanitization.
- Bot commits are absent.
- At least one cluster produced for any non-empty commit set.
- `--mode prep` writes this JSON and exits without calling any LLM.

---

### Phase 3 — Inductive phase: RetryLoop, theme extraction, synthesis

**Files:** `src/llm.rs` (new methods), new `src/retry_loop.rs`

This phase operates entirely on the `StructuredData` artifact from Phase 2.
No raw git data or unstructured text reaches the LLM.

**New `src/retry_loop.rs`:**
```rust
pub struct RetryLoop<T> {
    pub max_retries: u32,        // default 3
    pub temperature: f32,        // default 0.1 for extraction, 0.4 for synthesis
}
impl<T> RetryLoop<T> {
    pub async fn run(
        &self,
        ollama: &OllamaClient,
        prompt_fn: impl Fn(Option<&str>, Option<&str>) -> String,
        validate_fn: impl Fn(&str) -> Result<T, String>,
        fallback_fn: impl Fn() -> T,
    ) -> T
}
```
On each attempt, `validate_fn` returns either `Ok(parsed)` or `Err(feedback)` — a
human-readable string describing exactly what is missing or malformed. That feedback
is passed to `prompt_fn` on the next attempt so the model sees its prior output and
a specific correction instruction. After `max_retries`, `fallback_fn()` runs.

**New method `extract_themes_and_milestones(data: &StructuredData, prior_plan: Option<&str>) -> ExtractionResult`:**

Builds the extraction prompt from the *pre-clustered* `StructuredData` only — not raw
commit text. The clusters (from Phase 2 Rust clustering) replace the raw activity dump:

```
TASK: You are naming and analyzing pre-computed work clusters for a monthly report.
The clusters below were derived deterministically from commit messages. Do not invent
new clusters or add evidence items that are not listed.

CLUSTERS (from deductive analysis):
{cluster_label}: {commit_count} commits — evidence: {commit_ids}, files: {files}
...

MILESTONE MARKERS (from prior report):
{milestone_text or "None found."}

PRIOR PLAN NOTES:
{prior_plan_text or "Not provided."}

TASKS:
1. For each cluster, assign it to a THEME (name it concisely) or mark it as PIVOT
   if it matches no milestone marker.
2. For each milestone marker with no matching cluster, output PLAN_SHIFT: STALL.

OUTPUT FORMAT (strict — each line exactly as shown):
THEME: <name> | CLUSTERS: <label1,label2> | DESCRIPTION: <one sentence>
MILESTONE: <text> | STATUS: <complete|in-progress|not-started>
PLAN_SHIFT: PIVOT | CLUSTER: <label> | DESCRIPTION: <sentence>
PLAN_SHIFT: STALL | MILESTONE: <text>
```

`validate_fn` checks that every cluster appears in exactly one THEME or PIVOT line.
If any cluster is unaccounted for, the retry feedback is:
`"Cluster 'llm pipeline' is not assigned to any THEME or PIVOT. Please add it."`

**Fallback** (if all retries exhausted): each `cluster.label` becomes a theme name
directly; milestone statuses all default to `not-started`.

**New method `synthesize_with_themes(result: &ExtractionResult, data: &StructuredData) -> String`:**

One focused LLM call per theme, each wrapped in `RetryLoop`. The prompt for each:
```
TASK: Write a 3–5 sentence paragraph summarizing the work in theme "{name}".
Do not invent facts. Use only the evidence listed below.
EVIDENCE: {commits + files from assigned clusters}
OUTPUT: Just the paragraph, no heading.
```
`validate_fn`: response must be >= 40 chars and contain at least one evidence token
(7+ hex chars, `#\d+`, or a filename). Fallback: `"{n} commits related to {label}."`

Modify `synthesize_final_report()` to:
- Call Phase 2 deductive extraction first.
- Run `extract_themes_and_milestones()` via RetryLoop.
- Run per-theme synthesis via RetryLoop.
- Assemble into report with `## Milestones`, `## Identified Themes`, `## Plan Shifts`.
- Remove all hardcoded `"SPEAR project"` strings; use `config.project.name`.

**Acceptance criteria:**
- Every generated report contains `## Milestones` and `## Identified Themes` sections.
- No hardcoded project names in any prompt.
- RetryLoop fallback produces output even if Ollama is completely unavailable.
- Clusters from deductive phase are the *only* evidence source for themes.
- When a cluster has no milestone match, a `PIVOT` shift appears.
- When a milestone marker has no cluster, a `STALL` shift appears.

---

### Phase 4 — Wire `default_period` and fix filename templates

**Files:** `src/cli.rs`, `src/generators.rs`

**Changes:**
1. CLI date parsing: if `--from`/`--to` are absent, read
   `config.date_range.default_period` — parse `"current_month"` as the first and
   last day of the current calendar month, or a literal `"YYYY-MM"` as month
   boundaries.
2. Filename generation: replace `{project_name}` and `{template_name}` with
   `config.project.name` and the template key. Currently these substitutions are
   broken or absent.

**Acceptance criteria:**
- Running without date flags uses the current month automatically.
- Output file is named `<project>_monthly_<YYYY-MM>.md` with no raw placeholder text.

---

### Phase 5 — `make monthly` automation target

**Files:** `Makefile`

**Changes:**

```make
# Run full monthly report for current month (collect + theme-extract + synthesize)
monthly:
	cargo run -- collect --save collected_data.json
	cargo run -- report --data-file collected_data.json \
	    -o reports/$(shell date +%Y-%m)_monthly.md
	@echo "Done: reports/$(shell date +%Y-%m)_monthly.md"

# Dry run: collect + sanitize only, print token count and extracted themes
monthly-dry:
	cargo run -- collect --save collected_data.json --dry-run

# Pass a prior-month plan into the pipeline
monthly-with-plan:
	cargo run -- report --data-file collected_data.json \
	    --prior-plan docs/last_month_plan.md \
	    -o reports/$(shell date +%Y-%m)_monthly.md
```

**Acceptance criteria:**
- `make monthly` completes end-to-end without manual steps.
- `make monthly-dry` prints estimated token count and surfaced themes, then exits.

---

### Phase 6 — Wire `MultiStagePipeline` and `--mode` flag

**Files:** `src/structured_extractor.rs`, `src/multistage_pipeline.rs`, `src/cli.rs`

Now that the deductive extractor (Phase 2) and inductive RetryLoop stages (Phase 3)
exist as tested methods, this phase assembles them into the `local-report` command
and adds the `--mode` flag.

**`MultiStagePipeline::generate_report(data, mode, prior_plan, prior_report)`:**

```
mode = local  →  deductive extract → RetryLoop theme extraction → RetryLoop synthesis
mode = prep   →  deductive extract → write *_structured.json → exit (no LLM)
mode = cloud  →  deductive extract → single well-formed prompt → cloud API → write report
```

For `cloud` mode, the single prompt bundles the full `StructuredData` JSON plus
a concise instruction — the deductive artifact is compact enough to fit any cloud
model's context window comfortably.

**`MultiStageResult` fields for `--export-stages`:**
```rust
pub struct MultiStageResult {
    pub structured_data: StructuredData,   // deductive output
    pub extraction_result: ExtractionResult, // themes, milestones, shifts
    pub theme_paragraphs: Vec<(String, String)>, // (theme_name, paragraph)
    pub final_report: String,
    pub retry_stats: Vec<RetryStats>,      // per-call attempt counts
    pub metadata: PipelineMetadata,
}
```

`retry_stats` lets you see how many retries each LLM call needed — useful for
tuning prompt quality over time.

**Add `--mode` to `LocalReport` CLI command:**
```
--mode local   (default)
--mode prep
--mode cloud
```

**Acceptance criteria:**
- `cargo run -- local-report --data-file collected_data.json` runs end-to-end in local mode.
- `--mode prep` writes `*_structured.json` and exits without calling Ollama.
- `--export-stages` JSON includes `retry_stats` showing per-call attempt counts.
- `--mode cloud` requires `OPENAI_API_KEY` or `ANTHROPIC_API_KEY` and errors clearly if absent.

---

### Phase 7 — Report structure validation (warn-only)

**Files:** `src/generators.rs` or new `src/report_validator.rs`

A lightweight post-generation check (no LLM). Warn-only: never blocks or deletes.

Checks:
1. A `## Identified Themes` section exists.
2. At least one evidence token per theme (commit hash regex `[a-f0-9]{7,}`,
   issue `#\d+`, or a filename with extension).
3. A `## Milestones` section exists when `prior_report_path` was provided.
4. An executive summary paragraph is present.

Note: `## Plan Shifts` is intentionally optional — its absence is valid and means
the period stayed on plan.

On failure: print which themes are missing evidence. Report file is still written.

---

## Implementation Order

```
Phase 0 → Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 5 → Phase 6 → Phase 7
```

Phases 0–5 improve the existing `report` command and are each independently committable.  
Phase 6 makes `local-report` functional.  
Phase 7 is polish, deferrable.

---

## Commit Sequence

```
chore: stabilize build — minimal compile stubs for multistage_pipeline + structured_extractor
feat(config): add [reporting] section — token_budget, report_language, prior_plan_path/report_path
feat(preprocessor): deductive phase — sanitize, dedup, bot-filter, token budget
feat(extractor): deductive phase — keyword clustering, milestone marker extraction, StructuredData JSON
feat(retry_loop): add RetryLoop abstraction with validate + fallback
feat(llm): inductive phase — theme extraction and synthesis via RetryLoop; remove hardcoded project name
fix(cli): wire default_period and fix output filename template substitution
feat(make): add monthly, monthly-dry, monthly-with-plan targets
feat(local-report): wire MultiStagePipeline with --mode local/prep/cloud and --export-stages
feat(validate): add post-generation theme coverage validator (warn-only)
```

---

## Decisions Recorded

- No auto-open after generation.
- Themes and milestones fully automatic, no interactive selection.
- Prior plan and prior report files are always markdown.
- English output only.
- Token budget: 8000 input tokens per LLM call (suitable for 70b+ models).
- Milestones sourced from the prior month's **generated report**, not from config.
- Validator is warn-only; never blocks or deletes output.
- Architecture: **de-inductive** — deductive phase is pure Rust (deterministic, verifiable),
  inductive phase is LLM (synthesis only, operating on pre-structured evidence).
- Every LLM call wrapped in `RetryLoop` (max 3, format-correcting reprompt, Rust fallback).
- Commit clustering for plan-shift detection is Rust-first (tf-idf), not LLM-first.
  LLM names and narrates clusters; it does not discover them.
- Three execution modes: `local` (default), `prep` (deductive only, no LLM), `cloud` (API).
- `prep` mode is the escape hatch when local quality is insufficient.

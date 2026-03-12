# How to run chronopulse — monthly reporting tutorial

This doc walks you through the full de-inductive pipeline from scratch.

---

## Prerequisites

1. **Ollama running locally** with your preferred model:
   ```
   ollama serve
   ollama pull llama3.1:70b   # or whatever is in config.toml [llm] model
   ```
2. `config.toml` exists and has your project name, git repositories, and
   `[reporting]` section.  Run `cargo run -- init` if you need a fresh one.

---

## The two key commands

| Command | What it does |
|---|---|
| `collect` | Pulls commits/MRs/issues from all sources → JSON |
| `local-report` | Sanitize → cluster → LLM → markdown report |

---

## End-to-end in one make target

```bash
# Collect this month's data + generate report (steps 1 & 2 together)
make monthly

# Override the month:
make monthly MONTH=2026-02

# Run deductive phase only (no LLM call, outputs *_structured.json for inspection)
make monthly-prep

# Dry run — collect and show counts, no LLM, no files written
make monthly-dry

# Generate from an already-saved data file
make monthly-from-file DATA_FILE=collected_data.json

# Generate with intermediate stage dumps for debugging
make monthly-debug DATA_FILE=collected_data.json
```

---

## Manual step-by-step

### Step 1 — Collect

```bash
cargo run -- \
  --from 2026-02-01 --to 2026-02-28 \
  collect --output feb_data.json
```

Shows counts: GitLab commits, GitHub commits, local git commits, issues, MRs.

### Step 2 — Inspect the raw data (optional)

```bash
cat feb_data.json | jq '.git_commits | length'
cat feb_data.json | jq '[.git_commits[].author_name] | unique'
```

### Step 3 — Run the de-inductive pipeline

```bash
cargo run -- local-report --data-file feb_data.json
```

What happens inside:
1. **Sanitize** (pure Rust): dedup by hash, drop bots, truncate messages to 600
   chars, truncate file lists to 10, enforce 8000-token budget.
2. **Cluster** (pure Rust, tf-idf): commits are grouped into thematic clusters.
   Each cluster has a label, top terms, constituent commits, and file paths.
3. **Synthesize** (Ollama, up to 3 retries): the LLM receives only the
   structured cluster data — not raw diffs — and writes prose.  If it produces
   output that's too short (< 300 chars), it's automatically retried with
   feedback.  If all retries fail the pure-Rust fallback report is used.

Output: `reports/2026-02_<project>_monthly.md`

### Step 4 — Export the intermediate structured data (optional)

```bash
cargo run -- local-report \
  --data-file feb_data.json \
  --export-structured
```

Writes `reports/2026-02_<project>_monthly_structured.json` — a JSON snapshot
of what the LLM actually received:

```json
{
  "metadata": { "total_commits_sanitized": 47, "total_repos": 3, "repos": ["api", "db", "infra"] },
  "summary_metrics": { "total_additions": 2140, "total_deletions": 830, ... },
  "commit_clusters": [
    {
      "label": "auth",
      "terms": ["auth", "token", "jwt", "login", "session"],
      "commits": [...],
      "file_paths": ["src/auth.rs", "src/middleware.rs"]
    }
  ],
  "milestone_markers": [
    { "text": "Deploy staging environment", "status_hint": "COMPLETED" },
    { "text": "Write load tests", "status_hint": "IN_PROGRESS" }
  ]
}
```

Inspect this to verify the clustering and milestone extraction before you commit
the report to a repo or send it upstream.

---

## Using prior month context

Add paths in `config.toml`:

```toml
[reporting]
report_language = "en"
token_budget = 8000
prior_plan_path = "docs/2026-01_plan.md"        # goals/plan from last month
prior_report_path = "reports/2026-01_monthly.md" # last month's report (for milestone extraction)
```

Or use the `monthly-with-plan` Makefile pattern (see Makefile comments). The
prior plan's first 1500 chars are injected into the LLM prompt as context.
Milestones are extracted from the prior report via GFM task-list checkboxes and
`Milestone:` headings.

---

## Understanding the output

The LLM is constrained to these sections:

1. **Executive Summary** — 3-5 sentences, key numbers from `summary_metrics`
2. **Technical Achievements** — one subsection per commit cluster, named after
   the cluster's theme
3. **Progress on Prior Goals** — references extracted milestone markers
4. **Risks / Blockers** — only present if the data suggests them
5. **Next Steps**

If the LLM fails completely, the fallback report uses the same structure but
fills it from `commit_clusters` directly (no prose, just the commit titles).

---

## Running the tests

```bash
# All deterministic unit tests (fast, no Ollama needed)
cargo test --lib

# Full suite including integration tests
cargo test

# Just the sanitize/dedup/bot-filter tests
cargo test --lib preprocessor

# Just the clustering/milestone tests
cargo test --lib structured_extractor
```

Current test inventory (all pure Rust, deterministic):

| Module | Tests |
|---|---|
| `preprocessor` | dedup same hash, dedup different hashes, bot filter (dependabot / renovate / github-actions[bot]), message truncation to 600, file list truncation to 10, budget drops, budget keeps all |
| `structured_extractor` | cluster non-empty, repo metadata, all commits in some cluster, summary metrics, milestone checkboxes, milestone explicit labels, empty input |
| `retry_loop` | first-try success, retry on short output, fallback after exhaustion |

# Local LLM Optimization Guide

## Overview

ChronoPulse now includes an optimized pipeline for local LLMs that addresses the fundamental challenge: **local models need structured data and explicit guidance**, whereas cloud models like GPT-4 can handle unstructured data with implicit understanding.

## The Problem

When using local LLMs (like Llama, CodeLlama, DeepSeek), traditional report generation approaches fail because:

1. **Unstructured data overload**: Local models struggle to extract relevant information from large, unstructured dumps
2. **Context limitations**: Even with large context windows, local models lose track of key details
3. **Implicit reasoning gaps**: Local models need explicit instructions for each step
4. **Quality inconsistency**: Output varies significantly without proper structure

## The Solution: Multi-Stage Structured Pipeline

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ Stage 0: Structured Data Extraction                          │
│ Input: Raw CollectedData                                     │
│ Output: Structured JSON with explicit fields & metrics       │
└───────────────────────┬─────────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────────┐
│ Stage 1: Fact Extraction                                     │
│ Task: Pull discrete, verifiable facts                        │
│ LLM Use: Focused extraction from feature areas               │
└───────────────────────┬─────────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────────┐
│ Stage 2: Categorization                                      │
│ Task: Group related facts by type                            │
│ LLM Use: Generate category summaries                         │
└───────────────────────┬─────────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────────┐
│ Stage 3: Narrative Synthesis                                 │
│ Task: Create coherent narratives from categories             │
│ LLM Use: Executive summary, technical accomplishments        │
└───────────────────────┬─────────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────────┐
│ Stage 4: Report Formatting                                   │
│ Task: Apply template and structure                           │
│ Output: Final markdown report                                │
└─────────────────────────────────────────────────────────────┘
```

## Key Design Principles

### 1. Structured Data First

Instead of feeding raw git logs, we extract a structured representation:

```json
{
  "metadata": {
    "project_name": "MyProject",
    "date_range_start": "2024-01-01",
    "date_range_end": "2024-01-31"
  },
  "summary_metrics": {
    "total_commits": 150,
    "total_lines_added": 5000,
    "active_contributors": 5
  },
  "feature_areas": [
    {
      "area_name": "API & Services",
      "category": "feature",
      "impact_level": "major",
      "commit_count": 25,
      "key_changes": [...]
    }
  ]
}
```

### 2. Single-Task Prompts

Each LLM call has ONE focused task:

**❌ Bad (traditional approach):**
```
Analyze this data and create a comprehensive report with 
summary, technical details, and recommendations...
```

**✅ Good (multi-stage approach):**
```
Extract ONE sentence describing the main achievement in this area:
AREA: API & Services
COMMITS: 25
What was accomplished and why it matters.
```

### 3. Explicit Formats

Every prompt specifies exact output format:

```
TASK: Write a 3-4 sentence executive summary.
FORMAT: Just the paragraph, no heading.
SUMMARY:
```

### 4. Fallback Strategies

Every stage has rule-based fallbacks if LLM fails:

```rust
match ollama_client.generate(&prompt).await {
    Ok(response) => response.trim().to_string(),
    Err(_) => {
        // Rule-based fallback
        format!("{} commits in {} area", items.len(), category)
    }
}
```

## Usage

### Basic Usage

Generate a report using the local LLM pipeline:

```bash
# Collect data and generate report
chronopulse local-report --from 2024-01-01 --to 2024-01-31

# Use previously collected data
chronopulse local-report --data-file collected_data.json

# Export structured data for analysis
chronopulse local-report --export-structured --export-stages
```

### Configuration

Set up your local LLM in `config.toml`:

```toml
[llm]
provider = "ollama"
model = "llama3.2"  # or "deepseek-coder:33b", "codellama:34b"
max_tokens = 4000
temperature = 0.7
```

### Output Options

```bash
# Custom output location
chronopulse local-report -o reports/my_report.md

# Export intermediate stages for debugging
chronopulse local-report --export-stages

# Export structured data as JSON
chronopulse local-report --export-structured
```

## Structured Data Schema

The structured data extractor creates these key types:

### Feature Areas

Groups commits by functional area with impact assessment:

```rust
FeatureArea {
    area_name: "Data Collection",
    category: "feature",
    impact_level: "major",  // major, moderate, minor
    commit_count: 15,
    file_changes: 42,
    lines_changed: 2500,
    key_changes: [...]
}
```

### Technical Highlights

Automatically identifies major changes:

```rust
TechnicalHighlight {
    highlight_type: "refactor",  // new_feature, performance, refactor, integration
    title: "Restructured LLM pipeline",
    description: "Modified 12 files with 800 lines added",
    evidence: ["commit:abc123", "files:src/llm.rs, ..."],
    impact: "High",
    technical_details: ["large_change", "api_change"]
}
```

### Issue Resolutions

Tracks completed work:

```rust
IssueResolution {
    issue_id: "GitHub-42",
    title: "Add support for local LLMs",
    status: "resolved",
    resolution_type: "enhancement",
    related_commits: [...]
}
```

## Advanced Features

### Export Structured Data

Get the intermediate structured representation:

```bash
chronopulse local-report --export-structured
```

This creates `*_structured.json` with the full structured data that can be:
- Analyzed separately
- Fed to other tools
- Used with external LLM services (GPT-4, Claude)

### Stage-by-Stage Debugging

See output from each pipeline stage:

```bash
chronopulse local-report --export-stages
```

Creates `*_stages.json` showing:
- Extracted facts
- Categorized groups
- Generated narratives
- Processing statistics

### Use with External LLMs

The structured data format is LLM-agnostic. You can:

1. Export structured data: `--export-structured`
2. Send to GPT-4/Claude via API or copy-paste
3. Use appropriate prompts for the advanced model
4. Get higher quality analysis while keeping local data collection

## Performance Characteristics

### Local LLM Pipeline

- **Time**: 30-120 seconds (depends on model)
- **Quality**: Good for metrics, decent for narrative
- **Cost**: Free (local compute only)
- **Privacy**: All data stays local

### Comparison: Traditional vs Multi-Stage

| Aspect | Traditional | Multi-Stage |
|--------|-------------|-------------|
| Data Format | Unstructured text | Structured JSON |
| LLM Calls | 1-2 large calls | 10-15 focused calls |
| Context Used | High (80-90%) | Low (20-40% per call) |
| Fallback Quality | Poor | Good (rule-based) |
| Local LLM Success | 30-40% | 80-90% |

## Recommended Models

### For Report Generation

1. **deepseek-coder:33b** (Best quality)
   - 32K context window
   - Excellent code understanding
   - Slower but high quality

2. **codellama:34b** (Balanced)
   - 16K context window
   - Good technical writing
   - Moderate speed

3. **llama3.2** (Fast)
   - 8K context window
   - Quick generation
   - Lower quality but acceptable

### Configuration by Model

```toml
# For deepseek-coder:33b
[llm]
provider = "ollama"
model = "deepseek-coder:33b-instruct"
max_tokens = 8000
temperature = 0.3  # Lower for more consistency

# For faster models
[llm]
provider = "ollama"
model = "llama3.2"
max_tokens = 2000
temperature = 0.7
```

## Troubleshooting

### Issue: LLM times out

**Solution**: Use a smaller model or reduce `max_tokens`:

```toml
[llm]
model = "llama3.2"  # Smaller, faster model
max_tokens = 2000   # Reduce token limit
```

### Issue: Poor quality output

**Solution**: Try a larger model or export structured data for manual review:

```bash
# Export for manual analysis
chronopulse local-report --export-structured

# Or use larger model
ollama pull deepseek-coder:33b-instruct
```

### Issue: Ollama not responding

**Solution**: Check Ollama is running:

```bash
ollama list
curl http://localhost:11434/api/tags
```

## Integration with External Tools

### Using with Cursor/Claude

1. Generate structured data:
```bash
chronopulse local-report --export-structured -o reports/report.md
```

2. In Cursor, reference the structured JSON:
```
@reports/report_structured.json

Create an executive summary focusing on the major feature 
areas and their business impact.
```

### Using with Custom Scripts

```python
import json

# Load structured data
with open('report_structured.json') as f:
    data = json.load(f)

# Access specific sections
for area in data['feature_areas']:
    if area['impact_level'] == 'major':
        print(f"{area['area_name']}: {area['commit_count']} commits")
```

## API Service (Future)

A future version will include `chronopulse serve` to expose the pipeline as a local API:

```bash
chronopulse serve --port 8080
```

Then call from any tool:

```bash
curl -X POST http://localhost:8080/analyze \
  -H "Content-Type: application/json" \
  -d @collected_data.json
```

## Contributing

To improve the local LLM pipeline:

1. **Better Prompts**: Edit prompts in `multistage_pipeline.rs`
2. **New Extractors**: Add analysis to `structured_extractor.rs`
3. **Stage Improvements**: Enhance stages in `multistage_pipeline.rs`

See [CONTRIBUTING.md](../CONTRIBUTING.md) for details.

## References

- [Ollama Documentation](https://ollama.ai/docs)
- [Structured Data Schema](./structured-data-schema.md)
- [Multi-Stage Pipeline Architecture](./multistage-pipeline-architecture.md)

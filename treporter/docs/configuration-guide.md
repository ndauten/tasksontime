# ChronoPulse Configuration Guide

This guide provides comprehensive documentation for configuring ChronoPulse to collect data from your sources and generate automated reports.

## Table of Contents

- [Quick Start](#quick-start)
- [Configuration File Structure](#configuration-file-structure)
- [Project Settings](#project-settings)
- [Date Range Configuration](#date-range-configuration)
- [Collection Options](#collection-options)
- [Data Sources](#data-sources)
  - [GitLab](#gitlab-configuration)
  - [GitHub](#github-configuration)
  - [Local Files](#local-files-configuration)
- [Repository Configuration](#repository-configuration)
- [LLM Configuration](#llm-configuration)
- [Template Configuration](#template-configuration)
- [Output Configuration](#output-configuration)
- [Environment Variables](#environment-variables)
- [Examples](#configuration-examples)

## Quick Start

### Initialize a New Configuration

```bash
# Create a default config.toml with helpful comments
chronopulse init

# Create config in a specific location
chronopulse init -o /path/to/my-config.toml

# Force overwrite existing config
chronopulse init --force
```

### Verify Your Configuration

```bash
# Show current configuration
chronopulse config

# Test data collection (no reports generated)
chronopulse test
```

## Configuration File Structure

ChronoPulse uses a TOML configuration file (default: `config.toml`) with the following sections:

```toml
[project]           # Project metadata
[date_range]        # Default date settings
[collection]        # Data collection options
[data_sources]      # External data sources (GitLab, GitHub, files)
[[repositories]]    # Local git repositories
[llm]              # LLM provider settings
[templates]        # Report template configuration
[output]           # Output file settings
```

## Project Settings

Define your project's basic information that appears in reports.

```toml
[project]
# Project name (appears in report titles and filenames)
name = "My Project"

# Brief description (appears in report headers)
description = "Automated project reporting"
```

**Fields:**
- `name` (string, required): Your project name
- `description` (string, required): Brief project description

## Date Range Configuration

Set default time periods for data collection.

```toml
[date_range]
# Default period for reports (YYYY-MM format)
# CLI arguments --from and --to override this
default_period = "2025-01"
```

**Fields:**
- `default_period` (string): Default month in YYYY-MM format

**Note:** CLI arguments `--from` and `--to` always override this setting.

## Collection Options

Control how data is collected and processed.

```toml
[collection]
# Include git diffs in collected data
include_diffs = true

# Maximum diff size in characters (0 = unlimited)
# Recommended: 50000-100000 to avoid overwhelming LLM
max_diff_size = 50000
```

**Fields:**
- `include_diffs` (bool): Whether to include code diffs from commits/MRs
  - `true`: More context for LLM analysis (larger data)
  - `false`: Faster collection, smaller data files
- `max_diff_size` (integer): Maximum characters per diff
  - `0`: No limit (use with caution)
  - `50000`: Recommended for most projects
  - Larger values work with high-context LLMs (e.g., llama3.1:70b)

## Data Sources

### GitLab Configuration

Collect data from GitLab repositories and user activity.

```toml
[data_sources.gitlab]
enabled = true
token_env = "GITLAB_TOKEN"
username_env = "GITLAB_USERNAME"
base_url = "https://gitlab.com"
repositories = [
    "namespace/project-name",
    "mycompany/backend"
]
include_issues = true
include_merge_requests = true
include_commits = true
include_wiki = true
include_comments = true
```

**Fields:**
- `enabled` (bool): Enable/disable GitLab data collection
- `token_env` (string): Environment variable containing GitLab personal access token
- `username_env` (string): Environment variable containing GitLab username
- `base_url` (string): GitLab instance URL
  - Use `https://gitlab.com` for GitLab.com
  - Use your self-hosted URL for GitLab CE/EE
- `repositories` (array): List of repositories in `namespace/project` format
- `include_issues` (bool): Collect issues
- `include_merge_requests` (bool): Collect merge requests
- `include_commits` (bool): Collect commits
- `include_wiki` (bool): Collect wiki pages
- `include_comments` (bool): Collect issue/MR comments

**Creating a GitLab Token:**
1. Go to https://gitlab.com/-/profile/personal_access_tokens
2. Create a new token with scopes: `api`, `read_api`, `read_repository`
3. Set environment variable: `export GITLAB_TOKEN="your-token-here"`

### GitHub Configuration

Collect data from GitHub repositories and user activity.

```toml
[data_sources.github]
enabled = true
token_env = "GITHUB_TOKEN"
username_env = "GITHUB_USERNAME"
repositories = [
    "owner/repo-name",
    "facebook/react"
]
include_issues = true
include_pull_requests = true
include_commits = true
include_wiki = true
```

**Fields:**
- `enabled` (bool): Enable/disable GitHub data collection
- `token_env` (string): Environment variable containing GitHub personal access token
- `username_env` (string): Environment variable containing GitHub username
- `repositories` (array): List of repositories in `owner/repo` format
- `include_issues` (bool): Collect issues
- `include_pull_requests` (bool): Collect pull requests
- `include_commits` (bool): Collect commits
- `include_wiki` (bool): Collect wiki pages

**Creating a GitHub Token:**
1. Go to https://github.com/settings/tokens
2. Create a new token (classic) with scopes: `repo`, `read:org`
3. Set environment variable: `export GITHUB_TOKEN="your-token-here"`

### Local Files Configuration

Scan local markdown files for time-based notes and documentation.

```toml
[data_sources.local_files]
enabled = true
paths = [
    "/path/to/docs/**/*.md",
    "/path/to/notes/*.txt",
    "~/project/activities/**/*.md"
]
time_patterns = [
    "# \\d{4}-\\d{2}-\\d{2}",           # Date headers: # 2025-01-15
    "\\d{4}-\\d{2}-\\d{2}:",            # Date prefixes: 2025-01-15:
    "## Week of \\d{4}-\\d{2}-\\d{2}"   # Weekly headers
]
```

**Fields:**
- `enabled` (bool): Enable/disable local file scanning
- `paths` (array): File paths to scan (supports glob patterns)
  - `**/*.md`: Recursively find all markdown files
  - `*.txt`: All text files in directory
  - Tilde (`~`) expands to home directory
- `time_patterns` (array): Regex patterns to identify dated entries
  - Used to extract time-based notes from files
  - Each pattern should match date formats in your notes

**Common Time Patterns:**
```toml
time_patterns = [
    "# \\d{4}-\\d{2}-\\d{2}",           # Markdown headers with dates
    "\\d{4}-\\d{2}-\\d{2}:",            # Date prefixes
    "## Week of \\d{4}-\\d{2}-\\d{2}",  # Weekly headers
    "\\*\\*\\d{4}-\\d{2}-\\d{2}\\*\\*"  # Bold dates: **2025-01-15**
]
```

## Repository Configuration

Track local git repositories for commit history.

```toml
[[repositories]]
name = "my-project"
platform = "local"
path = "/path/to/my-project"
include_commits = true

[[repositories]]
name = "another-repo"
platform = "local"
path = "~/projects/another-repo"
include_commits = true
```

**Fields:**
- `name` (string): Repository display name
- `platform` (string): Must be `"local"` for local repos
- `path` (string): Absolute or home-relative path to repository
- `include_commits` (bool): Whether to collect git commits

**Note:** Each `[[repositories]]` entry defines one repository. Add multiple entries to track multiple repos.

## LLM Configuration

Configure the Language Model for intelligent report generation.

```toml
[llm]
provider = "ollama"
model = "llama3.1:8b"
api_key_env = "OLLAMA_API_KEY"
max_tokens = 4000
temperature = 0.3
```

**Fields:**
- `provider` (string): LLM provider
  - `"ollama"`: Local Ollama models (recommended, free)
  - `"openai"`: OpenAI API
  - `"anthropic"`: Anthropic Claude API
- `model` (string): Model identifier
- `api_key_env` (string): Environment variable containing API key
- `max_tokens` (integer): Maximum response length
  - `2000-4000`: Standard reports
  - `4000-8000`: Detailed analysis
- `temperature` (float, 0.0-1.0): Response creativity
  - `0.0-0.3`: Focused, deterministic (recommended for reports)
  - `0.7-1.0`: Creative, varied

### Recommended Models

**Ollama (Local, Free):**
```toml
# Best quality, large context (128K tokens)
model = "llama3.1:70b"

# Good balance of speed and quality
model = "llama3.1:8b"

# Excellent general purpose
model = "mistral:latest"

# Best for code-heavy projects
model = "codellama:34b"
```

**OpenAI:**
```toml
provider = "openai"
model = "gpt-4-turbo"        # Best quality
# OR
model = "gpt-3.5-turbo"      # Faster, cheaper
```

**Installation (Ollama):**
```bash
# Install Ollama: https://ollama.ai
# Pull a model
ollama pull llama3.1:8b

# Test it
ollama run llama3.1:8b "Hello!"
```

## Template Configuration

Define templates for different report types.

```toml
[templates]

[templates.monthly_report]
path = "templates/monthly-report.md"
output_format = "markdown"
sections = ["executive_summary", "accomplishments", "issues", "next_month"]

[templates.group_slides]
path = "templates/group_slides.md"
output_format = "marp"
sections = ["title", "highlights", "progress", "blockers", "next_steps"]
```

**Fields:**
- `path` (string): Path to template file (relative to config or absolute)
- `output_format` (string): Output format
  - `"markdown"`: Standard markdown
  - `"marp"`: Marp presentation slides
- `sections` (array): Report sections to generate

**Creating Custom Templates:**
Templates use Jinja2-like syntax with these variables:
- `{{ project_name }}`: Project name from config
- `{{ period }}`: Date range for report
- `{{ data }}`: Collected data (issues, commits, etc.)

## Output Configuration

Control where and how reports are saved.

```toml
[output]
base_directory = "reports"
date_format = "%Y-%m"
filename_template = "{project_name}_{template_name}_{date}"
```

**Fields:**
- `base_directory` (string): Directory for generated reports
  - Created automatically if it doesn't exist
  - Relative to current directory or absolute path
- `date_format` (string): strftime format for dates in filenames
  - `"%Y-%m"`: 2025-01
  - `"%Y-%m-%d"`: 2025-01-15
  - `"%B_%Y"`: January_2025
- `filename_template` (string): Template for report filenames
  - `{project_name}`: From `[project].name`
  - `{template_name}`: Template type (monthly_report, group_slides)
  - `{date}`: Formatted date

**Example Output:**
With this config:
```toml
base_directory = "reports"
date_format = "%Y-%m"
filename_template = "{project_name}_{template_name}_{date}"
```

Generated files:
```
reports/
  My_Project_monthly_report_2025-01.md
  My_Project_group_slides_2025-01.md
```

## Environment Variables

ChronoPulse uses environment variables for sensitive data like API tokens.

### Required Environment Variables

**For GitLab:**
```bash
export GITLAB_TOKEN="glpat-xxxxxxxxxxxxxxxxxxxx"
export GITLAB_USERNAME="your-username"
```

**For GitHub:**
```bash
export GITHUB_TOKEN="ghp_xxxxxxxxxxxxxxxxxxxx"
export GITHUB_USERNAME="your-username"
```

**For OpenAI:**
```bash
export OPENAI_API_KEY="sk-xxxxxxxxxxxxxxxxxxxx"
```

**For Anthropic:**
```bash
export ANTHROPIC_API_KEY="sk-ant-xxxxxxxxxxxxxxxxxxxx"
```

### Setting Environment Variables

**Temporarily (current session):**
```bash
export GITLAB_TOKEN="your-token"
```

**Permanently (add to ~/.zshrc or ~/.bashrc):**
```bash
echo 'export GITLAB_TOKEN="your-token"' >> ~/.zshrc
source ~/.zshrc
```

**Using a .env file:**
```bash
# Create .env file
cat > .env << EOF
GITLAB_TOKEN=your-token
GITLAB_USERNAME=your-username
GITHUB_TOKEN=your-token
GITHUB_USERNAME=your-username
EOF

# ChronoPulse automatically loads .env if present
```

## Configuration Examples

### Minimal Configuration (Local Only)

Track only local git repositories, no external APIs:

```toml
[project]
name = "My Local Project"
description = "Local development tracking"

[date_range]
default_period = "2025-01"

[collection]
include_diffs = true
max_diff_size = 50000

[data_sources]

[data_sources.gitlab]
enabled = false

[data_sources.github]
enabled = false

[data_sources.local_files]
enabled = false

[[repositories]]
name = "my-app"
platform = "local"
path = "~/projects/my-app"
include_commits = true

[llm]
provider = "ollama"
model = "llama3.1:8b"
api_key_env = "OLLAMA_API_KEY"
max_tokens = 4000
temperature = 0.3

[templates.monthly_report]
path = "templates/monthly-report.md"
output_format = "markdown"
sections = ["executive_summary", "accomplishments", "issues", "next_month"]

[templates.group_slides]
path = "templates/group_slides.md"
output_format = "marp"
sections = ["title", "highlights", "progress", "blockers", "next_steps"]

[output]
base_directory = "reports"
date_format = "%Y-%m"
filename_template = "{project_name}_{template_name}_{date}"
```

### Full Configuration (All Sources)

Track GitLab, GitHub, local files, and git repositories:

```toml
[project]
name = "Full Stack Project"
description = "Complete project tracking across all sources"

[date_range]
default_period = "2025-01"

[collection]
include_diffs = true
max_diff_size = 100000

[data_sources]

[data_sources.gitlab]
enabled = true
token_env = "GITLAB_TOKEN"
username_env = "GITLAB_USERNAME"
base_url = "https://gitlab.com"
repositories = [
    "mycompany/backend",
    "mycompany/frontend",
    "mycompany/mobile-app"
]
include_issues = true
include_merge_requests = true
include_commits = true
include_wiki = true
include_comments = true

[data_sources.github]
enabled = true
token_env = "GITHUB_TOKEN"
username_env = "GITHUB_USERNAME"
repositories = [
    "mycompany/infrastructure",
    "mycompany/documentation"
]
include_issues = true
include_pull_requests = true
include_commits = true
include_wiki = true

[data_sources.local_files]
enabled = true
paths = [
    "~/projects/docs/**/*.md",
    "~/notes/project/**/*.md"
]
time_patterns = [
    "# \\d{4}-\\d{2}-\\d{2}",
    "\\d{4}-\\d{2}-\\d{2}:",
    "## Week of \\d{4}-\\d{2}-\\d{2}"
]

[[repositories]]
name = "legacy-app"
platform = "local"
path = "~/projects/legacy-app"
include_commits = true

[[repositories]]
name = "internal-tools"
platform = "local"
path = "~/projects/internal-tools"
include_commits = true

[llm]
provider = "ollama"
model = "llama3.1:70b"
api_key_env = "OLLAMA_API_KEY"
max_tokens = 8000
temperature = 0.3

[templates.monthly_report]
path = "templates/monthly-report.md"
output_format = "markdown"
sections = ["executive_summary", "accomplishments", "issues", "next_month"]

[templates.group_slides]
path = "templates/group_slides.md"
output_format = "marp"
sections = ["title", "highlights", "progress", "blockers", "next_steps"]

[output]
base_directory = "reports"
date_format = "%Y-%m"
filename_template = "{project_name}_{template_name}_{date}"
```

### Team Configuration (Multiple Contributors)

Track multiple team members' contributions:

```toml
[project]
name = "Team Project"
description = "Multi-contributor project tracking"

[date_range]
default_period = "2025-01"

[collection]
include_diffs = false  # Faster collection without diffs
max_diff_size = 0

[data_sources]

[data_sources.gitlab]
enabled = true
token_env = "GITLAB_TOKEN"
username_env = "GITLAB_USERNAME"
base_url = "https://gitlab.com"
repositories = [
    "team/project-alpha",
    "team/project-beta"
]
include_issues = true
include_merge_requests = true
include_commits = true
include_wiki = false
include_comments = true

[data_sources.github]
enabled = true
token_env = "GITHUB_TOKEN"
username_env = "GITHUB_USERNAME"
repositories = [
    "team/shared-libraries"
]
include_issues = true
include_pull_requests = true
include_commits = true
include_wiki = false

[data_sources.local_files]
enabled = false

[llm]
provider = "ollama"
model = "llama3.1:8b"
api_key_env = "OLLAMA_API_KEY"
max_tokens = 4000
temperature = 0.3

[templates.monthly_report]
path = "templates/monthly-report.md"
output_format = "markdown"
sections = ["executive_summary", "accomplishments", "issues", "next_month"]

[templates.group_slides]
path = "templates/group_slides.md"
output_format = "marp"
sections = ["title", "highlights", "progress", "blockers", "next_steps"]

[output]
base_directory = "reports"
date_format = "%Y-%m"
filename_template = "{project_name}_{template_name}_{date}"
```

## Troubleshooting

### Configuration not found
```
Error: No such file or directory (os error 2)
```
**Solution:** Create config with `chronopulse init` or specify path with `-c`:
```bash
chronopulse -c /path/to/config.toml test
```

### Invalid TOML syntax
```
Error: TOML parse error at line X
```
**Solution:** Check for:
- Missing quotes around strings
- Unclosed arrays `]`
- Missing commas in arrays
- Use a TOML validator or editor with TOML support

### GitLab/GitHub authentication failed
```
Error: Unauthorized (401)
```
**Solution:**
- Verify token is set: `echo $GITLAB_TOKEN`
- Check token has correct scopes
- Regenerate token if expired

### No data collected
```
Total items collected: 0
```
**Solution:**
- Check date range with `--from` and `--to`
- Verify repositories exist and are accessible
- Check `enabled = true` for data sources
- Run `chronopulse test` to see detailed collection info

### LLM connection failed
```
Error: Connection refused
```
**Solution:**
- For Ollama: Check service is running: `ollama list`
- Start Ollama: `ollama serve`
- Verify model is installed: `ollama pull llama3.1:8b`

## Next Steps

After configuring ChronoPulse:

1. **Test your configuration:** `chronopulse test`
2. **Collect data:** `chronopulse collect -o test-data.json`
3. **Generate a report:** `chronopulse monthly-report --data-file test-data.json`
4. **Automate reports:** Set up a cron job or CI/CD pipeline

See the main [README.md](../README.md) for usage examples and advanced features.

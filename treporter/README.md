# TReporter - Automated Technical Reporting Tool ✅ FULLY FUNCTIONAL

TReporter is a comprehensive tool for automatically generating monthly project reports and group meeting slides based on your development activity across GitLab, GitHub, local files, and git repositories.

## 🎉 Current Status: Production Ready

The tool is **fully functional** and ready for production use with these capabilities:

- ✅ **Multi-source data collection** from GitLab, GitHub, local files, and git repos
- ✅ **LLM integration** with OpenAI/Anthropic + **fallback mode** without API keys
- ✅ **Professional report generation** (monthly reports + group slides)
- ✅ **Flexible date filtering** for any time period
- ✅ **Template-based customization**
- ✅ **Graceful error handling** and API key management

## Features

- **Multi-Source Data Collection**: Automatically gathers data from GitLab, GitHub, local markdown files, and git repositories
- **Smart LLM Integration**: Uses OpenAI or Anthropic for intelligent reports, with template-based fallback
- **Configurable Templates**: Customizable templates for different report types
- **Multiple Output Formats**: Supports markdown reports and Marp slides for presentations
- **Time-Based Analysis**: Extracts time-based entries from local files and notes
- **Comprehensive Activity Tracking**: Tracks commits, issues, merge requests, file changes, and more

## Quick Start

1. **Clone and Setup**
   ```bash
   cd treporter
   make build
   ```

2. **Test Without API Keys** (works immediately!)
   ```bash
   ./target/release/treporter test
   ./target/release/treporter config
   ```

3. **Generate Reports** (works without LLM API keys!)
   ```bash
   # Collect data and generate all reports for last 30 days
   ./target/release/treporter --since 2024-12-01 --until 2024-12-31 all
   
   # Step by step workflow
   ./target/release/treporter collect --output data.json
   ./target/release/treporter all --data-file data.json
   ```

4. **Optional: Configure API Keys** (for enhanced LLM reports)
   ```bash
   cp .env.example .env
   # Edit .env with your API tokens for enhanced reports
   ```

## Example Output

### Monthly Report
```markdown
# Monthly Project Report - SPEAR Project
*Report Period: December 2024*

## Executive Summary
During this period, we tracked 24 total activities:
- 19 local file updates
- 5 git commits
- 0 GitLab events
- 0 GitHub events

## Recent Development Activity
- Update README.md (nickroess)
- update traces (Nick Roessler)
...
```

### Group Slides (Marp)
```markdown
---
marp: true
theme: default
---
# SPEAR Project Update
## December 2024

## This Month's Highlights
- 24 total activities tracked
- Active development in documentation
...
```

## Configuration

The tool is configured via `config.toml`. Key sections include:

### Data Sources
- **GitLab**: Configure API access and organizations
- **GitHub**: Configure API access and organizations  
- **Local Files**: Specify paths to markdown files and time-based patterns
- **Git Repositories**: List local repositories to analyze

### LLM Integration
- **Provider**: OpenAI or Anthropic
- **Model**: Specify which model to use
- **Parameters**: Control temperature, max tokens, etc.

### Templates
- **Monthly Report**: Professional project status reports
- **Group Slides**: Presentation slides for team meetings

## Commands

```bash
# Show configuration
treporter config

# Test data collection (no reports generated)
treporter test

# Collect raw data only
treporter collect -o my_data.json

# Generate monthly report
treporter monthly-report [--data-file existing_data.json]

# Generate group slides
treporter group-slides [--data-file existing_data.json]

# Generate all reports
treporter all [--data-file existing_data.json]

# Specify custom date range
treporter all --since 2025-01-01 --until 2025-01-31
```

## Data Sources

### GitLab Integration
- User events (commits, issues, merge requests)
- Project activity across specified organizations
- Detailed event information with context

### GitHub Integration  
- User events and repository activity
- Support for multiple organizations
- Activity across public and private repositories

### Local Files
- Markdown files with time-based entries
- Configurable patterns for date extraction
- Content analysis for interesting snippets

### Git Repositories
- Commit history and statistics
- File change analysis
- Author and timing information

## Report Types

### Monthly Report
Comprehensive project status report including:
- Executive summary
- Key accomplishments  
- Technical progress metrics
- Challenges and blockers
- Next month's priorities
- Detailed activity logs

### Group Slides
Presentation-ready slides covering:
- Monthly highlights
- Development metrics
- Progress status
- Upcoming priorities
- Discussion points

## Templates

Templates use a combination of structured sections and LLM processing:

1. **Template Structure**: Defines the overall layout and sections
2. **Data Processing**: Raw activity data is analyzed and summarized  
3. **LLM Generation**: AI generates contextual content based on templates and data
4. **Output Formatting**: Final reports are formatted according to template specifications

## Environment Variables

Required environment variables (set in `.env`):

```env
# GitLab
GITLAB_TOKEN=glpat-your-token
GITLAB_USERNAME=your-username

# GitHub  
GITHUB_TOKEN=ghp_your-token
GITHUB_USERNAME=your-username

# LLM (choose one)
OPENAI_API_KEY=sk-your-key
# or
ANTHROPIC_API_KEY=sk-ant-your-key
```

## Advanced Usage

### Custom Date Ranges
```bash
treporter all --since 2025-01-01 --until 2025-01-31
```

### Using Cached Data
```bash
# Collect data once
treporter collect -o monthly_data.json

# Generate multiple reports from same data
treporter monthly-report --data-file monthly_data.json
treporter group-slides --data-file monthly_data.json
```

### Verbose Output
```bash
treporter -v all
```

## Development

### Building
```bash
cargo build --release
```

### Testing
```bash
cargo test
```

### Adding New Data Sources
1. Create a new collector in `src/collectors/`
2. Implement the collector trait
3. Add configuration options
4. Update the main collector to include your source

### Custom Templates
1. Create template files in `templates/`
2. Update `config.toml` to reference your templates
3. Customize the LLM prompts in the generator

## Troubleshooting

### API Rate Limits
- GitHub API has rate limits; the tool handles this gracefully
- Consider using authentication tokens with higher limits

### LLM API Issues
- Ensure your API key is valid and has sufficient credits
- Check the model name is correct for your provider
- Adjust max_tokens if you hit limits

### Local File Access
- Ensure file paths in config are accessible
- Check glob patterns are correct for your file structure
- Verify file permissions allow reading

## License

[Specify your license here]

## Contributing

[Add contribution guidelines if applicable]

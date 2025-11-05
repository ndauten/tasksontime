# ChronoPulse Credential Management

ChronoPulse provides a flexible, multi-layered approach to credential management that works for both individual developers and CI/CD environments.

## Overview

### Four Ways to Set Credentials

1. **Global User Config** (Recommended for personal use)
   - Location: `~/.chronopulse/config.toml`
   - Set once, use everywhere
   - Secure, not in project repos

2. **Project-Specific Config**
   - Location: `./config.toml` (in project directory)
   - For project-specific settings (repos, templates, output)

3. **Project Environment File**
   - Location: `./.env` (in project directory)
   - For project-specific credential overrides
   - Add to `.gitignore`

4. **Environment Variables** (Highest priority)
   - For CI/CD, Docker, dynamic environments
   - Example: `export GITLAB_TOKEN=glpat-xxx`

### Priority Order (Highest to Lowest)

```
Environment Variables → Project .env → Global ~/.chronopulse/config.toml → Error
```

## Quick Start

### Step 1: Set Up Global Credentials

```bash
# Interactive setup wizard
chronopulse setup

# View current global configuration (masked)
chronopulse setup --show

# Force overwrite existing configuration
chronopulse setup --force
```

### Step 2: Initialize Project

```bash
# Create project config (auto-detects git repo)
cd your-project/
chronopulse init

# Force overwrite existing config
chronopulse init --force
```

### Step 3: Collect Data

```bash
# Uses credentials from global config
chronopulse collect
```

## Global Configuration Format

**Location:** `~/.chronopulse/config.toml`

```toml
# ChronoPulse Global Configuration
# Credentials used across all projects

[gitlab]
token = "glpat-your-token-here"
username = "your-gitlab-username"
base_url = "https://gitlab.com"

[github]
token = "ghp_your-token-here"
username = "your-github-username"

[llm]
provider = "ollama"  # or "openai", "anthropic"

[llm.ollama]
base_url = "http://localhost:11434"
model = "llama3.2"

# Optional: OpenAI
# [llm.openai]
# api_key = "sk-your-openai-key"
# model = "gpt-4"

# Optional: Anthropic
# [llm.anthropic]
# api_key = "sk-ant-your-anthropic-key"
# model = "claude-3-opus-20240229"
```

## Project Configuration

**Location:** `./config.toml` (in your project)

This file contains **project-specific settings** like:
- Project name and description
- Repositories to analyze
- Report templates
- Output directories

**It does NOT contain credentials** (those come from global config or env vars).

Example:
```toml
[project]
name = "My Project"
description = "Automated reporting for my awesome project"

[[repositories]]
name = "my-repo"
platform = "gitlab"
path = "/path/to/repo"

# ... more project settings ...
```

## Environment Variables

### For Individual Use

Create a `.env` file in your project (add to `.gitignore`):

```bash
# Override GitLab credentials for this project only
GITLAB_TOKEN=glpat-different-token
GITLAB_USERNAME=different-user

# Override LLM provider for this project
USE_OLLAMA=true
```

### For CI/CD

Set environment variables in your CI/CD platform:

```yaml
# GitHub Actions example
env:
  GITLAB_TOKEN: ${{ secrets.GITLAB_TOKEN }}
  GITLAB_USERNAME: ${{ secrets.GITLAB_USERNAME }}
  OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
```

```yaml
# GitLab CI example
variables:
  GITLAB_TOKEN: $GITLAB_TOKEN  # From CI/CD variables
  GITLAB_USERNAME: $GITLAB_USERNAME
```

## Security Best Practices

### ✅ DO

- **Use global config** for personal credentials (`~/.chronopulse/config.toml`)
- **Use environment variables** in CI/CD
- **Add `.env` to `.gitignore`**
- **Use personal access tokens** (not passwords)
- **Limit token scopes** to minimum required permissions
- **Rotate tokens regularly**

### ❌ DON'T

- **Never commit credentials** to git
- **Never put tokens in project `config.toml`**
- **Don't share your global config file**
- **Don't use overly permissive token scopes**

## Setup Wizard Details

The interactive setup wizard will guide you through:

### GitLab Configuration

1. **Personal Access Token**
   - Create at: https://gitlab.com/-/profile/personal_access_tokens
   - Required scopes: `read_api`, `read_repository`

2. **Username**
   - Your GitLab username

3. **Base URL**
   - Default: `https://gitlab.com`
   - For self-hosted: `https://gitlab.your-company.com`

### GitHub Configuration

1. **Personal Access Token**
   - Create at: https://github.com/settings/tokens
   - Required scopes: `repo`, `read:org`

2. **Username**
   - Your GitHub username

### LLM Configuration

Choose your AI provider:

1. **Ollama** (Recommended - Free & Local)
   - Base URL: `http://localhost:11434`
   - Model: `llama3.2` (or any installed model)
   - Install: https://ollama.ai/

2. **OpenAI** (Paid Service)
   - API Key from: https://platform.openai.com/api-keys
   - Model: `gpt-4`, `gpt-3.5-turbo`, etc.

3. **Anthropic Claude** (Paid Service)
   - API Key from: https://console.anthropic.com/
   - Model: `claude-3-opus-20240229`, etc.

## Troubleshooting

### "Credential 'GITLAB_TOKEN' not found"

This means ChronoPulse couldn't find your credentials. Check:

1. **Global config exists**: `chronopulse setup --show`
2. **Token is set**: Check `~/.chronopulse/config.toml`
3. **Environment variable**: `echo $GITLAB_TOKEN`

Solution:
```bash
# Run setup wizard
chronopulse setup

# Or set environment variable
export GITLAB_TOKEN=glpat-your-token
```

### "No global configuration found"

You haven't run setup yet:

```bash
chronopulse setup
```

### Update Credentials

```bash
# Re-run setup wizard
chronopulse setup --force

# Or manually edit
vim ~/.chronopulse/config.toml
```

### View Current Configuration

```bash
# Show global config (tokens masked)
chronopulse setup --show

# Show project config
chronopulse config
```

## Examples

### Scenario 1: Solo Developer

```bash
# One-time setup
chronopulse setup

# Use in any project
cd project-a/
chronopulse init
chronopulse collect

cd ../project-b/
chronopulse init
chronopulse collect
```

### Scenario 2: Team with Different Credentials

Each team member:
```bash
# Alice sets up her credentials
chronopulse setup
# Uses her GitLab token

# Bob sets up his credentials  
chronopulse setup
# Uses his GitLab token

# Both can work on same project
cd shared-project/
chronopulse collect  # Uses each person's credentials
```

### Scenario 3: CI/CD Pipeline

```yaml
# .github/workflows/report.yml
name: Generate Report
on: [push]

jobs:
  report:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Install ChronoPulse
        run: cargo install --git https://github.com/yourrepo/chronopulse
      
      - name: Generate Report
        env:
          GITLAB_TOKEN: ${{ secrets.GITLAB_TOKEN }}
          GITLAB_USERNAME: ci-bot
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
        run: |
          chronopulse collect
          chronopulse monthly-report
```

### Scenario 4: Docker Container

```dockerfile
FROM rust:latest

# Install ChronoPulse
RUN cargo install chronopulse

# Set credentials via environment
ENV GITLAB_TOKEN=${GITLAB_TOKEN}
ENV GITLAB_USERNAME=docker-bot

WORKDIR /app
COPY config.toml .

CMD ["chronopulse", "all"]
```

```bash
# Run with credentials from environment
docker run -e GITLAB_TOKEN=$GITLAB_TOKEN my-chronopulse-image
```

## Migration from Old Setup

If you were using `.env` files:

1. **Extract credentials** from `.env`
2. **Run setup wizard**: `chronopulse setup`
3. **Remove credentials** from `.env` (keep project-specific vars)
4. **Update `.gitignore`**: Ensure `.env` is ignored

Before:
```bash
# .env (committed by accident!)
GITLAB_TOKEN=glpat-xxx
PROJECT_NAME=MyProject
```

After:
```bash
# ~/.chronopulse/config.toml (secure, global)
[gitlab]
token = "glpat-xxx"

# .env (project-specific, in .gitignore)
PROJECT_NAME=MyProject
```

## File Locations

```
~/.chronopulse/
  └── config.toml          # Global credentials (secure)

/your/project/
  ├── config.toml          # Project settings (safe to commit)
  ├── .env                 # Optional overrides (add to .gitignore)
  └── .gitignore           # Must include .env
```

## Permissions

### macOS/Linux

```bash
# Secure global config (only you can read)
chmod 600 ~/.chronopulse/config.toml

# Secure project .env
chmod 600 .env
```

### Windows

Right-click `config.toml` → Properties → Security → Edit permissions to restrict access.

## Summary

✨ **Best Practice Flow:**

1. ` chronopulse setup` ← Set global credentials once
2. `cd your-project/ && chronopulse init` ← Configure project
3. `chronopulse collect` ← Just works!

🔐 **Credential Hierarchy:**
```
ENV VARS > .env > ~/.chronopulse/config.toml > ❌ Error
```

💡 **Remember:**
- Credentials → Global config or environment
- Project settings → Project config.toml
- Secrets → Never commit to git

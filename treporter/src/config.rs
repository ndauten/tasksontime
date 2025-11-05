use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub project: ProjectConfig,
    pub date_range: DateRangeConfig,
    pub collection: CollectionConfig,
    pub data_sources: DataSourcesConfig,
    pub repositories: Vec<RepositoryConfig>,
    pub repository_discovery: Option<RepositoryDiscoveryConfig>,
    pub llm: LlmConfig,
    pub templates: TemplatesConfig,
    pub output: OutputConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RepositoryDiscoveryConfig {
    pub enabled: bool,
    pub search_paths: Vec<String>,
    pub max_depth: Option<usize>,
    pub ignore_patterns: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProjectConfig {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DateRangeConfig {
    pub default_period: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CollectionConfig {
    pub include_diffs: bool,
    pub max_diff_size: usize,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DataSourcesConfig {
    pub gitlab: Option<GitLabConfig>,
    pub github: Option<GitHubConfig>,
    pub local_files: Option<LocalFilesConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GitLabConfig {
    pub enabled: bool,
    pub token_env: String,
    pub username_env: String,
    pub base_url: String,
    pub repositories: Option<Vec<String>>,
    pub include_issues: Option<bool>,
    pub include_merge_requests: Option<bool>,
    pub include_commits: Option<bool>,
    pub include_wiki: Option<bool>,
    pub include_comments: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GitHubConfig {
    pub enabled: bool,
    pub token_env: String,
    pub username_env: String,
    pub organizations: Option<Vec<String>>,
    pub repositories: Option<Vec<String>>,
    pub include_issues: Option<bool>,
    pub include_pull_requests: Option<bool>,
    pub include_commits: Option<bool>,
    pub include_wiki: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LocalFilesConfig {
    pub enabled: bool,
    pub paths: Vec<String>,
    pub time_patterns: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RepositoryConfig {
    pub name: String,
    pub platform: String,
    pub group: Option<String>,
    pub project_id: Option<u64>,
    pub path: Option<String>,
    pub include_issues: Option<bool>,
    pub include_merge_requests: Option<bool>,
    pub include_commits: Option<bool>,
    pub include_wiki: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LlmConfig {
    pub provider: String,
    pub model: String,
    pub api_key_env: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TemplatesConfig {
    pub monthly_report: TemplateConfig,
    pub group_slides: TemplateConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TemplateConfig {
    pub path: String,
    pub output_format: String,
    pub sections: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OutputConfig {
    pub base_directory: String,
    pub date_format: String,
    pub filename_template: String,
}

impl Config {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// Generate a default config.toml with helpful comments (legacy, uses current dir)
    #[allow(dead_code)]
    pub fn generate_default() -> String {
        Self::generate_default_with_repo_path(".")
    }
    
    /// Generate default config with specified repository path
    #[allow(dead_code)]
    fn generate_default_with_repo_path(repo_path: &str) -> String {
        let repo_name = std::path::Path::new(repo_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("chronopulse");
        
        let template = r###"# ChronoPulse Configuration File
# This file controls how ChronoPulse collects data and generates reports

[project]
# Your project name (appears in reports)
name = "My Project"
# Brief description of your project
description = "Automated project reporting"

[date_range]
# Default period for reports (YYYY-MM format)
# Can be overridden by CLI arguments --from and --to
default_period = "2025-01"

[collection]
# Include git diffs for commits and merge requests
# Provides detailed change information but increases data size
include_diffs = true
# Maximum diff size to include (in characters, 0 = no limit)
# Set to a reasonable size to avoid overwhelming the LLM
max_diff_size = 50000

# ============================================================================
# DATA SOURCES
# ============================================================================

[data_sources]

# ----------------------------------------------------------------------------
# GitLab Configuration
# ----------------------------------------------------------------------------
[data_sources.gitlab]
# Enable/disable GitLab data collection
enabled = false
# Environment variable containing your GitLab personal access token
# Create token at: https://gitlab.com/-/profile/personal_access_tokens
# Required scopes: api, read_api, read_repository
token_env = "GITLAB_TOKEN"
# Environment variable containing your GitLab username
username_env = "GITLAB_USERNAME"
# GitLab instance URL (use https://gitlab.com for GitLab.com)
base_url = "https://gitlab.com"

# List of repositories to collect from (format: namespace/project)
# Example: ["mycompany/backend", "mycompany/frontend"]
repositories = []

# What to collect from GitLab repositories
include_issues = true
include_merge_requests = true
include_commits = true
include_wiki = true
include_comments = true

# ----------------------------------------------------------------------------
# GitHub Configuration
# ----------------------------------------------------------------------------
[data_sources.github]
# Enable/disable GitHub data collection
enabled = false
# Environment variable containing your GitHub personal access token
# Create token at: https://github.com/settings/tokens
# Required scopes: repo, read:org
token_env = "GITHUB_TOKEN"
# Environment variable containing your GitHub username
username_env = "GITHUB_USERNAME"

# List of repositories to collect from (format: owner/repo)
# Example: ["facebook/react", "microsoft/vscode"]
repositories = []

# What to collect from GitHub repositories
include_issues = true
include_pull_requests = true
include_commits = true
include_wiki = true

# ----------------------------------------------------------------------------
# Local Files Configuration
# ----------------------------------------------------------------------------
[data_sources.local_files]
# Enable/disable local file scanning
enabled = false
# File paths to scan (supports glob patterns)
# Example: ["/path/to/docs/**/*.md", "/path/to/notes/*.txt"]
paths = []

# Patterns to identify time-based notes in files
# These regex patterns help extract dated entries from markdown files
time_patterns = [
    "# \\d{4}-\\d{2}-\\d{2}",           # Date headers: # 2025-01-15
    "\\d{4}-\\d{2}-\\d{2}:",            # Date prefixes: 2025-01-15:
    "## Week of \\d{4}-\\d{2}-\\d{2}"   # Weekly headers
]

# ============================================================================
# LOCAL GIT REPOSITORIES
# ============================================================================
# Add local repositories to track git history
# Each repository requires: name, platform="local", and path
# IMPORTANT: At least one repository entry is required. 

[[repositories]]
name = "REPO_NAME_PLACEHOLDER"
platform = "local"
path = "REPO_PATH_PLACEHOLDER"
include_commits = true

# Add more local repositories
# [[repositories]]
# name = "my-project"
# platform = "local"
# path = "/path/to/my-project"
# include_commits = true

# Remote repository examples (GitLab/GitHub)
# Note: Remote repos are primarily accessed via data_sources.gitlab/github above
# Use these entries if you want to clone and analyze local copies
# [[repositories]]
# name = "remote-project"
# platform = "gitlab"  # or "github"
# group = "namespace"  # GitLab group/namespace or GitHub owner
# project_id = 12345   # GitLab project ID (optional)
# include_commits = true
# include_issues = true
# include_merge_requests = true

# ============================================================================
# REPOSITORY DISCOVERY (Optional)
# ============================================================================
# Automatically discover git repositories in specified directories
# Useful if you have multiple related repos in a workspace

# [repository_discovery]
# enabled = false
# # Paths to search for git repositories
# search_paths = [
#     "./repos",           # Search in repos/ subdirectory
#     "~/projects",        # Search in home projects directory
#     "/path/to/workspace" # Any absolute or relative path
# ]
# # Maximum depth to search (prevents deep recursion)
# max_depth = 3
# # Directory patterns to ignore (regex)
# ignore_patterns = [
#     "node_modules",
#     "vendor",
#     ".venv",
#     "venv",
#     "__pycache__",
#     "target",
#     "build",
#     "dist"
# ]

# ============================================================================
# LLM CONFIGURATION
# ============================================================================
[llm]
# LLM provider: "ollama", "openai", "anthropic"
provider = "ollama"

# Model to use for report generation
# Ollama models (run locally):
#   - llama3.1:70b       # Best quality, large context (128K tokens)
#   - llama3.1:8b        # Good balance of speed and quality
#   - mistral:latest     # Excellent general purpose
#   - codellama:34b      # Best for code-heavy projects
#   - deepseek-coder:33b # Alternative for code analysis
# OpenAI models:
#   - gpt-4-turbo        # Best quality
#   - gpt-3.5-turbo      # Faster, cheaper
model = "llama3.1:8b"

# Environment variable containing API key (not needed for Ollama)
api_key_env = "OLLAMA_API_KEY"

# Maximum tokens in LLM response
# Increase for more detailed reports, decrease for faster generation
max_tokens = 4000

# Temperature (0.0-1.0)
# Lower = more focused/deterministic, Higher = more creative/random
temperature = 0.3

# ============================================================================
# REPORT TEMPLATES
# ============================================================================
[templates]

# Monthly report template
[templates.monthly_report]
path = "templates/monthly-report.md"
output_format = "markdown"
sections = ["executive_summary", "accomplishments", "issues", "next_month"]

# Group slides template
[templates.group_slides]
path = "templates/group_slides.md"
output_format = "marp"  # Marp markdown for presentations
sections = ["title", "highlights", "progress", "blockers", "next_steps"]

# ============================================================================
# OUTPUT CONFIGURATION
# ============================================================================
[output]
# Directory for generated reports
base_directory = "reports"
# Date format for report filenames
date_format = "%Y-%m"
# Template for report filenames
# Available variables: {{project_name}}, {{template_name}}, {{date}}
filename_template = "{{project_name}}_{{template_name}}_{{date}}"

# ============================================================================
# GETTING STARTED
# ============================================================================
# 1. Set environment variables for tokens:
#    export GITLAB_TOKEN="your-token"
#    export GITLAB_USERNAME="your-username"
#    export GITHUB_TOKEN="your-token"
#    export GITHUB_USERNAME="your-username"
#
# 2. Enable data sources and add repositories above
#
# 3. Install Ollama (for local LLM): https://ollama.ai
#    ollama pull llama3.1:8b
#
# 4. Test your configuration:
#    chronopulse test
#
# 5. Collect data:
#    chronopulse collect
#
# 6. Generate reports:
#    chronopulse all --from 2025-01-01 --to 2025-01-31
"###;
        
        template
            .replace("REPO_NAME_PLACEHOLDER", repo_name)
            .replace("REPO_PATH_PLACEHOLDER", repo_path)
    }
    
    /// Write default config to a file
    pub fn write_default(path: &str, force: bool) -> anyhow::Result<()> {
        use std::path::Path;
        
        // Check if file exists
        if Path::new(path).exists() && !force {
            anyhow::bail!("Config file '{}' already exists. Use --force to overwrite.", path);
        }
        
        // Find git repository root by searching up directories
        let repo_path = Self::find_git_root().unwrap_or_else(|| ".".to_string());
        
        let config_content = Self::generate_default_with_repo_path(&repo_path);
        std::fs::write(path, config_content)?;
        println!("✓ Created default config file: {}", path);
        
        if repo_path != "." {
            println!("✓ Detected git repository at: {}", repo_path);
        }
        
        println!("\nNext steps:");
        println!("1. Edit {} to configure your project", path);
        println!("2. Set environment variables for API tokens (see config comments)");
        println!("3. Run 'chronopulse test' to verify your configuration");
        
        Ok(())
    }
    
    /// Find the git repository root by searching up directories
    pub fn find_git_root() -> Option<String> {
        use std::env;
        
        
        let current_dir = env::current_dir().ok()?;
        let mut path = current_dir.as_path();
        
        loop {
            let git_dir = path.join(".git");
            if git_dir.exists() {
                return Some(path.to_string_lossy().to_string());
            }
            
            // Move up to parent directory
            path = path.parent()?;
        }
    }
    
    /// Discover git repositories in configured search paths
    #[allow(dead_code)]
    pub fn discover_repositories(&self) -> Vec<RepositoryConfig> {
        let discovery = match &self.repository_discovery {
            Some(d) if d.enabled => d,
            _ => return vec![],
        };
        
        let mut discovered = Vec::new();
        let max_depth = discovery.max_depth.unwrap_or(3);
        let ignore_patterns: Vec<regex::Regex> = discovery.ignore_patterns
            .as_ref()
            .map(|patterns| {
                patterns.iter()
                    .filter_map(|p| regex::Regex::new(p).ok())
                    .collect()
            })
            .unwrap_or_default();
        
        for search_path in &discovery.search_paths {
            let expanded_path = shellexpand::tilde(search_path).to_string();
            if let Ok(repos) = Self::find_git_repos(&expanded_path, max_depth, &ignore_patterns) {
                discovered.extend(repos);
            }
        }
        
        discovered
    }
    
    /// Recursively find git repositories in a directory
    #[allow(dead_code)]
    fn find_git_repos(
        path: &str,
        max_depth: usize,
        ignore_patterns: &[regex::Regex],
    ) -> std::io::Result<Vec<RepositoryConfig>> {
        Self::find_git_repos_recursive(path, max_depth, 0, ignore_patterns)
    }
    
    #[allow(dead_code)]
    fn find_git_repos_recursive(
        path: &str,
        max_depth: usize,
        current_depth: usize,
        ignore_patterns: &[regex::Regex],
    ) -> std::io::Result<Vec<RepositoryConfig>> {
        use std::fs;
        use std::path::Path;
        
        if current_depth > max_depth {
            return Ok(vec![]);
        }
        
        let path_obj = Path::new(path);
        if !path_obj.exists() || !path_obj.is_dir() {
            return Ok(vec![]);
        }
        
        // Check if current directory should be ignored
        if let Some(dir_name) = path_obj.file_name().and_then(|n| n.to_str()) {
            for pattern in ignore_patterns {
                if pattern.is_match(dir_name) {
                    return Ok(vec![]);
                }
            }
        }
        
        let mut repos = Vec::new();
        
        // Check if this directory is a git repo
        if path_obj.join(".git").exists() {
            // Get the repository name from the directory name
            let name = if let Some(file_name) = path_obj.file_name().and_then(|n| n.to_str()) {
                file_name.to_string()
            } else {
                // For paths like ".", get the canonical path and extract the name
                path_obj.canonicalize()
                    .ok()
                    .and_then(|p| p.file_name().and_then(|n| n.to_str().map(|s| s.to_string())))
                    .unwrap_or_else(|| "unknown".to_string())
            };
            
            repos.push(RepositoryConfig {
                name,
                platform: "local".to_string(),
                group: None,
                project_id: None,
                path: Some(path.to_string()),
                include_issues: None,
                include_merge_requests: None,
                include_commits: Some(true),
                include_wiki: None,
            });
        }
        
        // Recurse into subdirectories (even if current dir is a git repo, to find nested repos)
        if let Ok(entries) = fs::read_dir(path_obj) {
            for entry in entries.filter_map(|e| e.ok()) {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        if let Some(subpath) = entry.path().to_str() {
                            if let Ok(mut subrepos) = Self::find_git_repos_recursive(
                                subpath,
                                max_depth,
                                current_depth + 1,
                                ignore_patterns,
                            ) {
                                repos.append(&mut subrepos);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(repos)
    }
}

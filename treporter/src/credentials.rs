use anyhow::{Result, Context, anyhow};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use std::io::{self, Write};

/// Global credentials configuration stored in ~/.chronopulse/config.toml
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct GlobalConfig {
    #[serde(default)]
    pub gitlab: Option<GlobalGitLabConfig>,
    #[serde(default)]
    pub github: Option<GlobalGitHubConfig>,
    #[serde(default)]
    pub llm: Option<GlobalLlmConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GlobalGitLabConfig {
    pub token: Option<String>,
    pub username: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GlobalGitHubConfig {
    pub token: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GlobalLlmConfig {
    pub provider: Option<String>,
    #[serde(default)]
    pub ollama: Option<OllamaConfig>,
    #[serde(default)]
    pub openai: Option<OpenAIConfig>,
    #[serde(default)]
    pub anthropic: Option<AnthropicConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OllamaConfig {
    pub base_url: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenAIConfig {
    pub api_key: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AnthropicConfig {
    pub api_key: Option<String>,
    pub model: Option<String>,
}

impl GlobalConfig {
    /// Get the path to global config directory: ~/.chronopulse/
    pub fn global_config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not determine home directory"))?;
        Ok(home.join(".chronopulse"))
    }
    
    /// Get the path to global config file: ~/.chronopulse/config.toml
    pub fn global_config_path() -> Result<PathBuf> {
        Ok(Self::global_config_dir()?.join("config.toml"))
    }
    
    /// Load global configuration from ~/.chronopulse/config.toml
    pub fn load() -> Result<Self> {
        let path = Self::global_config_path()?;
        
        if !path.exists() {
            return Ok(Self::default());
        }
        
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read global config from {}", path.display()))?;
        
        let config: GlobalConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse global config from {}", path.display()))?;
        
        Ok(config)
    }
    
    /// Save global configuration to ~/.chronopulse/config.toml
    pub fn save(&self) -> Result<()> {
        let dir = Self::global_config_dir()?;
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create directory {}", dir.display()))?;
        
        let path = Self::global_config_path()?;
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize global config")?;
        
        fs::write(&path, content)
            .with_context(|| format!("Failed to write global config to {}", path.display()))?;
        
        println!("✓ Global configuration saved to: {}", path.display());
        Ok(())
    }
    
    /// Generate a template global config with helpful comments
    #[allow(dead_code)]
    pub fn generate_template() -> String {
        r#"# ChronoPulse Global Configuration
# This file contains credentials and default settings used across all projects
# Location: ~/.chronopulse/config.toml
#
# Credential Priority (highest to lowest):
# 1. Environment variables (e.g., GITLAB_TOKEN)
# 2. Project .env file
# 3. Project config.toml
# 4. Global ~/.chronopulse/config.toml (this file)

[gitlab]
# GitLab Personal Access Token
# Create at: https://gitlab.com/-/profile/personal_access_tokens
# Scopes needed: read_api, read_repository
token = "glpat-your-token-here"
username = "your-gitlab-username"
base_url = "https://gitlab.com"

[github]
# GitHub Personal Access Token
# Create at: https://github.com/settings/tokens
# Scopes needed: repo, read:org
token = "ghp_your-token-here"
username = "your-github-username"

[llm]
# Default LLM provider: "ollama", "openai", or "anthropic"
provider = "ollama"

[llm.ollama]
# Local Ollama server (free, runs locally)
base_url = "http://localhost:11434"
model = "llama3.2"

# [llm.openai]
# # OpenAI API (paid service)
# api_key = "sk-your-openai-key"
# model = "gpt-4"

# [llm.anthropic]
# # Anthropic Claude API (paid service)
# api_key = "sk-ant-your-anthropic-key"
# model = "claude-3-opus-20240229"
"#.to_string()
    }
    
    /// Interactive setup wizard for global credentials
    pub fn interactive_setup(force: bool, project_only: bool, global_only: bool) -> Result<()> {
        // Determine what to set up
        let setup_global = !project_only;
        let setup_project = !global_only;
        
        // If neither flag is set, show menu
        let (do_global, do_project) = if !project_only && !global_only {
            println!("🔧 ChronoPulse Setup Wizard");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!();
            println!("What would you like to set up?");
            println!("  1) Global credentials only (~/.chronopulse/config.toml)");
            println!("  2) Project configuration only (./config.toml)");
            println!("  3) Both global and project");
            println!();
            
            let choice = prompt_with_default("Choice", "3")?;
            
            match choice.as_str() {
                "1" => (true, false),
                "2" => (false, true),
                _ => (true, true),
            }
        } else {
            (setup_global, setup_project)
        };
        
        println!();
        
        // Global credentials setup
        if do_global {
            Self::setup_global_credentials(force)?;
        }
        
        // Project configuration setup
        if do_project {
            Self::setup_project_configuration(force)?;
        }
        
        println!();
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("✨ Setup complete!");
        println!();
        
        if do_global && do_project {
            println!("Next steps:");
            println!("  1. Review and edit config.toml for project-specific settings");
            println!("  2. Run 'chronopulse test' to verify your configuration");
            println!("  3. Run 'chronopulse collect' to start collecting data");
        } else if do_global {
            println!("Next steps:");
            println!("  1. Run 'chronopulse setup --project' to set up a project");
            println!("  2. Or run 'chronopulse init' in your project directory");
        } else {
            println!("Next steps:");
            println!("  1. Run 'chronopulse test' to verify your configuration");
            println!("  2. Run 'chronopulse collect' to start collecting data");
        }
        
        Ok(())
    }
    
    /// Set up global credentials
    fn setup_global_credentials(force: bool) -> Result<()> {
        let path = Self::global_config_path()?;
        
        if path.exists() && !force {
            println!("⚠️  Global config already exists at: {}", path.display());
            print!("Overwrite? (y/N): ");
            io::stdout().flush()?;
            
            let mut response = String::new();
            io::stdin().read_line(&mut response)?;
            
            if !response.trim().eq_ignore_ascii_case("y") {
                println!("Setup cancelled.");
                return Ok(());
            }
        }
        
        println!("🔧 ChronoPulse Global Setup");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();
        println!("This wizard will help you set up global credentials.");
        println!("You can skip any section by pressing Enter.");
        println!();
        
        let mut config = GlobalConfig::default();
        
        // GitLab setup
        println!("📦 GitLab Configuration");
        println!("Create token at: https://gitlab.com/-/profile/personal_access_tokens");
        println!("Required scopes: read_api, read_repository");
        println!();
        
        let gitlab_token = prompt_optional("GitLab Personal Access Token")?;
        let gitlab_username = prompt_optional("GitLab Username")?;
        let gitlab_url = prompt_with_default("GitLab Base URL", "https://gitlab.com")?;
        
        if gitlab_token.is_some() || gitlab_username.is_some() {
            config.gitlab = Some(GlobalGitLabConfig {
                token: gitlab_token,
                username: gitlab_username,
                base_url: Some(gitlab_url),
            });
        }
        
        println!();
        
        // GitHub setup
        println!("📦 GitHub Configuration");
        println!("Create token at: https://github.com/settings/tokens");
        println!("Required scopes: repo, read:org");
        println!();
        
        let github_token = prompt_optional("GitHub Personal Access Token")?;
        let github_username = prompt_optional("GitHub Username")?;
        
        if github_token.is_some() || github_username.is_some() {
            config.github = Some(GlobalGitHubConfig {
                token: github_token,
                username: github_username,
            });
        }
        
        println!();
        
        // LLM setup
        println!("🤖 LLM Configuration");
        println!("Choose your AI provider:");
        println!("  1) Ollama (free, runs locally - recommended)");
        println!("  2) OpenAI (paid service)");
        println!("  3) Anthropic Claude (paid service)");
        println!("  4) Skip");
        println!();
        
        let llm_choice = prompt_with_default("LLM Provider", "1")?;
        
        match llm_choice.as_str() {
            "1" => {
                let ollama_url = prompt_with_default("Ollama Base URL", "http://localhost:11434")?;
                let ollama_model = prompt_with_default("Ollama Model", "llama3.2")?;
                
                config.llm = Some(GlobalLlmConfig {
                    provider: Some("ollama".to_string()),
                    ollama: Some(OllamaConfig {
                        base_url: Some(ollama_url),
                        model: Some(ollama_model),
                    }),
                    openai: None,
                    anthropic: None,
                });
            },
            "2" => {
                let openai_key = prompt_required("OpenAI API Key")?;
                let openai_model = prompt_with_default("OpenAI Model", "gpt-4")?;
                
                config.llm = Some(GlobalLlmConfig {
                    provider: Some("openai".to_string()),
                    ollama: None,
                    openai: Some(OpenAIConfig {
                        api_key: Some(openai_key),
                        model: Some(openai_model),
                    }),
                    anthropic: None,
                });
            },
            "3" => {
                let anthropic_key = prompt_required("Anthropic API Key")?;
                let anthropic_model = prompt_with_default("Anthropic Model", "claude-3-opus-20240229")?;
                
                config.llm = Some(GlobalLlmConfig {
                    provider: Some("anthropic".to_string()),
                    ollama: None,
                    openai: None,
                    anthropic: Some(AnthropicConfig {
                        api_key: Some(anthropic_key),
                        model: Some(anthropic_model),
                    }),
                });
            },
            _ => {
                println!("Skipping LLM configuration.");
            }
        }
        
        println!();
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        
        // Save the configuration
        config.save()?;
        
        Ok(())
    }
    
    /// Set up project configuration interactively
    fn setup_project_configuration(force: bool) -> Result<()> {
        use crate::config::Config;
        use std::path::Path;
        
        let config_path = "config.toml";
        
        if Path::new(config_path).exists() && !force {
            println!("⚠️  Project config already exists at: {}", config_path);
            print!("Overwrite? (y/N): ");
            io::stdout().flush()?;
            
            let mut response = String::new();
            io::stdin().read_line(&mut response)?;
            
            if !response.trim().eq_ignore_ascii_case("y") {
                println!("Skipping project setup.");
                return Ok(());
            }
        }
        
        println!();
        println!("📋 Project Configuration Setup");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();
        
        // Detect git repository
        let repo_path = Config::find_git_root().unwrap_or_else(|| ".".to_string());
        let repo_name = std::path::Path::new(&repo_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("My Project");
        
        // Project information
        println!("📁 Project Information");
        println!();
        let project_name = prompt_with_default("Project Name", repo_name)?;
        let project_desc = prompt_with_default("Project Description", "Automated project reporting")?;
        
        println!();
        
        // Data sources
        println!("📦 Data Sources");
        println!();
        println!("Which data sources would you like to enable?");
        
        let enable_gitlab = prompt_yes_no("Enable GitLab?", true)?;
        let enable_github = prompt_yes_no("Enable GitHub?", false)?;
        let enable_local_files = prompt_yes_no("Enable Local Files?", true)?;
        
        println!();
        
        // Repository discovery
        println!("🔍 Repository Discovery");
        println!();
        let enable_discovery = prompt_yes_no("Enable automatic repository discovery?", false)?;
        
        let mut search_paths = vec![];
        let mut max_depth = 3;
        
        if enable_discovery {
            println!();
            println!("Enter search paths (one per line, empty line to finish):");
            println!("Examples: ~/projects, /work/repos, .");
            
            loop {
                print!("Search path: ");
                io::stdout().flush()?;
                let mut path = String::new();
                io::stdin().read_line(&mut path)?;
                let path = path.trim();
                
                if path.is_empty() {
                    break;
                }
                search_paths.push(path.to_string());
            }
            
            if search_paths.is_empty() {
                search_paths.push(".".to_string());
            }
            
            let depth_str = prompt_with_default("Maximum depth to search", "3")?;
            max_depth = depth_str.parse().unwrap_or(3);
        }
        
        println!();
        
        // LLM Configuration  
        println!("🤖 LLM Provider");
        println!();
        println!("Choose LLM provider:");
        println!("  1) Use global config settings");
        println!("  2) Ollama (local)");
        println!("  3) OpenAI");
        println!("  4) Anthropic");
        println!();
        
        let llm_choice = prompt_with_default("LLM Provider", "1")?;
        
        let (llm_provider, llm_model) = match llm_choice.as_str() {
            "2" => ("ollama".to_string(), prompt_with_default("Model", "llama3.2")?),
            "3" => ("openai".to_string(), prompt_with_default("Model", "gpt-4")?),
            "4" => ("anthropic".to_string(), prompt_with_default("Model", "claude-3-opus-20240229")?),
            _ => {
                // Use global config - try to load it
                match GlobalConfig::load() {
                    Ok(global) => {
                        let provider = global.llm.as_ref()
                            .and_then(|l| l.provider.clone())
                            .unwrap_or_else(|| "ollama".to_string());
                        
                        let model = match provider.as_str() {
                            "ollama" => global.llm.as_ref()
                                .and_then(|l| l.ollama.as_ref())
                                .and_then(|o| o.model.clone())
                                .unwrap_or_else(|| "llama3.2".to_string()),
                            "openai" => global.llm.as_ref()
                                .and_then(|l| l.openai.as_ref())
                                .and_then(|o| o.model.clone())
                                .unwrap_or_else(|| "gpt-4".to_string()),
                            "anthropic" => global.llm.as_ref()
                                .and_then(|l| l.anthropic.as_ref())
                                .and_then(|a| a.model.clone())
                                .unwrap_or_else(|| "claude-3-opus-20240229".to_string()),
                            _ => "llama3.2".to_string(),
                        };
                        
                        (provider, model)
                    },
                    Err(_) => ("ollama".to_string(), "llama3.2".to_string()),
                }
            }
        };
        
        println!();
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("📝 Generating config.toml...");
        
        // Build the configuration content
        let mut config_content = String::new();
        
        config_content.push_str(&format!(r###"# ChronoPulse Project Configuration

[project]
name = "{}"
description = "{}"

[date_range]
default_period = "current_month"

[collection]
include_diffs = false
max_diff_size = 10000

[data_sources]

"###, project_name, project_desc));
        
        // GitLab configuration
        if enable_gitlab {
            config_content.push_str(r###"[data_sources.gitlab]
enabled = true
token_env = "GITLAB_TOKEN"
username_env = "GITLAB_USERNAME"
base_url = "https://gitlab.com"
include_issues = true
include_merge_requests = true
include_commits = true
include_wiki = false
include_comments = true

"###);
        }
        
        // GitHub configuration
        if enable_github {
            config_content.push_str(r###"[data_sources.github]
enabled = true
token_env = "GITHUB_TOKEN"
username_env = "GITHUB_USERNAME"
include_issues = true
include_pull_requests = true
include_commits = true
include_wiki = false

"###);
        }
        
        // Local files configuration
        if enable_local_files {
            config_content.push_str(r###"[data_sources.local_files]
enabled = true
paths = ["./notes/**/*.md", "./docs/**/*.md"]
time_patterns = [
    '\d{4}-\d{2}-\d{2}',
    '(?i)(january|february|march|april|may|june|july|august|september|october|november|december)\s+\d{1,2},?\s+\d{4}',
]

"###);
        }
        
        // Repository configuration
        config_content.push_str(&format!(r###"# Repositories to analyze
[[repositories]]
name = "{}"
platform = "local"
path = "{}"
include_commits = true

"###, project_name, repo_path));
        
        // Repository discovery
        if enable_discovery && !search_paths.is_empty() {
            config_content.push_str(&format!(r###"# Automatic repository discovery
[repository_discovery]
enabled = true
search_paths = [{}]
max_depth = {}
ignore_patterns = ["node_modules", "vendor", ".venv", "target", "build", "dist"]

"###, 
                search_paths.iter().map(|p| format!("\"{}\"", p)).collect::<Vec<_>>().join(", "),
                max_depth
            ));
        }
        
        // LLM configuration
        config_content.push_str(&format!(r###"[llm]
provider = "{}"
model = "{}"
api_key_env = "OPENAI_API_KEY"
max_tokens = 4000
temperature = 0.7

[templates]

[templates.monthly_report]
path = "templates/monthly_report.md"
output_format = "markdown"
sections = ["summary", "accomplishments", "metrics", "challenges", "next_steps"]

[templates.group_slides]
path = "templates/group_slides.md"
output_format = "marp"
sections = ["highlights", "metrics", "status", "priorities"]

[output]
base_directory = "./reports"
date_format = "%Y-%m"
filename_template = "{{project}}_{{type}}_{{date}}"
"###, llm_provider, llm_model));
        
        // Write the file
        std::fs::write(config_path, config_content)?;
        println!("✓ Created project configuration: {}", config_path);
        
        if repo_path != "." {
            println!("✓ Detected git repository at: {}", repo_path);
        }
        
        // If repository discovery is enabled, run it now and show results
        if enable_discovery && !search_paths.is_empty() {
            use crate::config::Config;
            
            println!();
            println!("🔍 Discovering repositories...");
            
            // Load the config we just created
            if let Ok(config) = Config::load(config_path) {
                let discovered = config.discover_repositories();
                
                if discovered.is_empty() {
                    println!("   No additional repositories found.");
                } else {
                    println!("   Found {} repositories", discovered.len());
                    println!();
                    
                    // Ask user about each discovered repository
                    let mut selected_repos = Vec::new();
                    for repo in &discovered {
                        if let Some(path) = &repo.path {
                            let add = prompt_yes_no(
                                &format!("Add repository '{}' ({})?", repo.name, path),
                                true
                            )?;
                            
                            if add {
                                selected_repos.push(repo.clone());
                                println!("   ✓ Added {}", repo.name);
                            } else {
                                println!("   ⊘ Skipped {}", repo.name);
                            }
                        }
                    }
                    
                    if !selected_repos.is_empty() {
                        println!();
                        println!("📝 Adding {} selected repositories to config...", selected_repos.len());
                        
                        // Read the current config file
                        let mut config_content = std::fs::read_to_string(config_path)?;
                        
                        // Find where to insert the new repositories (after the existing [[repositories]] section)
                        let mut new_repos_section = String::new();
                        for repo in &selected_repos {
                            if let Some(path) = &repo.path {
                                new_repos_section.push_str(&format!(r###"
[[repositories]]
name = "{}"
platform = "local"
path = "{}"
include_commits = true

"###, repo.name, path));
                            }
                        }
                        
                        // Insert after the first [[repositories]] section
                        if let Some(pos) = config_content.find("# Automatic repository discovery") {
                            config_content.insert_str(pos, &new_repos_section);
                        } else if let Some(pos) = config_content.find("[llm]") {
                            config_content.insert_str(pos, &new_repos_section);
                        }
                        
                        // Write updated config
                        std::fs::write(config_path, config_content)?;
                        println!("   ✓ Configuration updated with selected repositories");
                    } else {
                        println!();
                        println!("   No repositories selected. Discovery will run automatically during collection.");
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Show current global configuration (with masked secrets)
    pub fn show() -> Result<()> {
        let path = Self::global_config_path()?;
        
        if !path.exists() {
            println!("❌ No global configuration found.");
            println!("   Run 'chronopulse setup' to create one.");
            return Ok(());
        }
        
        let config = Self::load()?;
        
        println!("📋 Global Configuration");
        println!("Location: {}", path.display());
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();
        
        // GitLab
        if let Some(gitlab) = &config.gitlab {
            println!("📦 GitLab:");
            if let Some(token) = &gitlab.token {
                println!("  Token: {}***", &token.chars().take(8).collect::<String>());
            }
            if let Some(username) = &gitlab.username {
                println!("  Username: {}", username);
            }
            if let Some(url) = &gitlab.base_url {
                println!("  Base URL: {}", url);
            }
            println!();
        }
        
        // GitHub
        if let Some(github) = &config.github {
            println!("📦 GitHub:");
            if let Some(token) = &github.token {
                println!("  Token: {}***", &token.chars().take(4).collect::<String>());
            }
            if let Some(username) = &github.username {
                println!("  Username: {}", username);
            }
            println!();
        }
        
        // LLM
        if let Some(llm) = &config.llm {
            println!("🤖 LLM:");
            if let Some(provider) = &llm.provider {
                println!("  Provider: {}", provider);
            }
            
            if let Some(ollama) = &llm.ollama {
                println!("  Ollama URL: {}", ollama.base_url.as_ref().unwrap_or(&"(not set)".to_string()));
                println!("  Ollama Model: {}", ollama.model.as_ref().unwrap_or(&"(not set)".to_string()));
            }
            
            if let Some(openai) = &llm.openai {
                if let Some(key) = &openai.api_key {
                    println!("  OpenAI Key: {}***", &key.chars().take(8).collect::<String>());
                }
                if let Some(model) = &openai.model {
                    println!("  OpenAI Model: {}", model);
                }
            }
            
            if let Some(anthropic) = &llm.anthropic {
                if let Some(key) = &anthropic.api_key {
                    println!("  Anthropic Key: {}***", &key.chars().take(8).collect::<String>());
                }
                if let Some(model) = &anthropic.model {
                    println!("  Anthropic Model: {}", model);
                }
            }
            println!();
        }
        
        if config.gitlab.is_none() && config.github.is_none() && config.llm.is_none() {
            println!("⚠️  Configuration file is empty.");
            println!("   Run 'chronopulse setup' to configure credentials.");
        }
        
        Ok(())
    }
}

/// Helper function to prompt for optional input
fn prompt_optional(prompt: &str) -> Result<Option<String>> {
    print!("{} (optional): ", prompt);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim();
    
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

/// Helper function to prompt for required input
fn prompt_required(prompt: &str) -> Result<String> {
    loop {
        print!("{}: ", prompt);
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
        
        println!("❌ This field is required. Please enter a value.");
    }
}

/// Helper function to prompt with default value
fn prompt_with_default(prompt: &str, default: &str) -> Result<String> {
    print!("{} [{}]: ", prompt, default);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim();
    
    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

/// Helper function to prompt for yes/no with default
fn prompt_yes_no(prompt: &str, default: bool) -> Result<bool> {
    let default_str = if default { "Y/n" } else { "y/N" };
    print!("{} [{}]: ", prompt, default_str);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim().to_lowercase();
    
    if trimmed.is_empty() {
        Ok(default)
    } else {
        Ok(trimmed.starts_with('y'))
    }
}

/// Get a credential value with priority: env var > project .env > global config
#[allow(dead_code)]
pub fn get_credential(
    env_var_name: &str,
    global_value: Option<&String>,
) -> Result<String> {
    // 1. Check environment variable
    if let Ok(value) = std::env::var(env_var_name) {
        return Ok(value);
    }
    
    // 2. Check global config
    if let Some(value) = global_value {
        return Ok(value.clone());
    }
    
    // 3. Return error with helpful message
    Err(anyhow!(
        "Credential '{}' not found. Set it via:\n\
         1. Environment variable: export {}=your-value\n\
         2. Global config: chronopulse setup\n\
         3. Project .env file",
        env_var_name, env_var_name
    ))
}

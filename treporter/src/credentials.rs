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
    pub fn interactive_setup(force: bool) -> Result<()> {
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
        
        println!();
        println!("✨ Setup complete!");
        println!();
        println!("Next steps:");
        println!("  1. Run 'chronopulse init' in your project directory");
        println!("  2. Edit the generated config.toml for project-specific settings");
        println!("  3. Run 'chronopulse collect' to start collecting data");
        
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

/// Get a credential value with priority: env var > project .env > global config
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

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub project: ProjectConfig,
    pub date_range: DateRangeConfig,
    pub data_sources: DataSourcesConfig,
    pub repositories: Vec<RepositoryConfig>,
    pub llm: LlmConfig,
    pub templates: TemplatesConfig,
    pub output: OutputConfig,
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
    pub organizations: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GitHubConfig {
    pub enabled: bool,
    pub token_env: String,
    pub username_env: String,
    pub organizations: Vec<String>,
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
}

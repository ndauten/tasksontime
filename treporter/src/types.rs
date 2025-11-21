use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CollectedData {
    pub gitlab_raw: Vec<serde_json::Value>,
    pub github_raw: Vec<serde_json::Value>,
    pub local_files: Vec<LocalFileData>,
    pub git_commits: Vec<GitCommitData>,
    pub metadata: CollectionMetadata,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CollectionMetadata {
    pub collection_time: DateTime<Utc>,
    pub date_range_start: DateTime<Utc>,
    pub date_range_end: DateTime<Utc>,
    pub sources_used: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitLabEvent {
    pub id: u64,
    pub action_name: String,
    pub target_type: Option<String>,
    pub target_title: Option<String>,
    pub project_id: Option<u64>,
    pub project_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub details: Option<String>,
    pub author_name: Option<String>,
    pub commit_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitHubEvent {
    pub id: String,
    pub event_type: String,
    pub repo_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub actor: Option<String>,
    pub payload: Option<serde_json::Value>,
    pub details: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalFileData {
    pub file_path: String,
    pub file_name: String,
    pub last_modified: DateTime<Utc>,
    pub content_snippets: Vec<ContentSnippet>,
    pub time_based_entries: Vec<TimeBasedEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentSnippet {
    pub line_number: usize,
    pub content: String,
    pub context_lines: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeBasedEntry {
    pub timestamp: DateTime<Utc>,
    pub content: String,
    pub entry_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitCommitData {
    pub hash: String,
    pub author_name: String,
    pub author_email: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub repo_path: String,
    pub files_changed: Vec<String>,
}

impl Default for CollectedData {
    fn default() -> Self {
        CollectedData {
            gitlab_raw: Vec::new(),
            github_raw: Vec::new(),
            local_files: Vec::new(),
            git_commits: Vec::new(),
            metadata: CollectionMetadata {
                collection_time: Utc::now(),
                date_range_start: Utc::now(),
                date_range_end: Utc::now(),
                sources_used: Vec::new(),
            },
        }
    }
}

impl CollectedData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn total_items(&self) -> usize {
        self.gitlab_raw.len() + 
        self.github_raw.len() + 
        self.local_files.len() + 
        self.git_commits.len()
    }

    pub fn save_to_file(&self, file_path: &str) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(file_path, content)?;
        Ok(())
    }

    pub fn load_from_file(file_path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(file_path)?;
        let data: CollectedData = serde_json::from_str(&content)?;
        Ok(data)
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CollectedData {
    pub gitlab_events: Vec<GitLabEvent>,
    pub github_events: Vec<GitHubEvent>,
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
    pub date: DateTime<Utc>,
    pub content: String,
    pub entry_type: String, // "daily_note", "task_update", "meeting_note", etc.
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitCommitData {
    pub repository: String,
    pub commit_hash: String,
    pub message: String,
    pub author: String,
    pub author_email: String,
    pub date: DateTime<Utc>,
    pub files_changed: Vec<String>,
    pub insertions: usize,
    pub deletions: usize,
}

impl CollectedData {
    pub fn new() -> Self {
        Self {
            gitlab_events: Vec::new(),
            github_events: Vec::new(),
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

    pub fn total_events(&self) -> usize {
        self.gitlab_events.len() + 
        self.github_events.len() + 
        self.local_files.len() + 
        self.git_commits.len()
    }

    pub fn save_to_file(&self, path: &str) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let data: CollectedData = serde_json::from_str(&content)?;
        Ok(data)
    }
}

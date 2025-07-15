use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CollectedData {
    pub gitlab: GitLabData,
    pub github: GitHubData,
    pub local_files: Vec<LocalFileData>,
    pub git_commits: Vec<GitCommitData>,
    pub metadata: CollectionMetadata,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitLabData {
    pub commits: Vec<serde_json::Value>,
    pub merge_requests: Vec<serde_json::Value>,
    pub issues: Vec<serde_json::Value>,
    pub comments: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitHubData {
    pub commits: Vec<serde_json::Value>,
    pub pull_requests: Vec<serde_json::Value>,
    pub issues: Vec<serde_json::Value>,
    pub comments: Vec<serde_json::Value>,
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
            gitlab: GitLabData::default(),
            github: GitHubData::default(),
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

impl Default for GitLabData {
    fn default() -> Self {
        GitLabData {
            commits: Vec::new(),
            merge_requests: Vec::new(),
            issues: Vec::new(),
            comments: Vec::new(),
        }
    }
}

impl Default for GitHubData {
    fn default() -> Self {
        GitHubData {
            commits: Vec::new(),
            pull_requests: Vec::new(),
            issues: Vec::new(),
            comments: Vec::new(),
        }
    }
}

impl CollectedData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn total_items(&self) -> usize {
        self.gitlab.total_items() + 
        self.github.total_items() + 
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

    pub fn merge(&mut self, other: CollectedData) {
        // Merge GitLab data
        self.gitlab.commits.extend(other.gitlab.commits);
        self.gitlab.merge_requests.extend(other.gitlab.merge_requests);
        self.gitlab.issues.extend(other.gitlab.issues);
        self.gitlab.comments.extend(other.gitlab.comments);
        
        // Merge GitHub data
        self.github.commits.extend(other.github.commits);
        self.github.pull_requests.extend(other.github.pull_requests);
        self.github.issues.extend(other.github.issues);
        self.github.comments.extend(other.github.comments);
        
        // Merge local data
        self.local_files.extend(other.local_files);
        self.git_commits.extend(other.git_commits);
        
        // Update metadata
        self.metadata.sources_used.push(format!("Merged from multiple sources at {}", 
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")));
    }

    pub fn load_and_merge_files(file_paths: &[String]) -> anyhow::Result<Self> {
        if file_paths.is_empty() {
            return Err(anyhow::anyhow!("No files provided for merging"));
        }
        
        let mut merged_data = Self::load_from_file(&file_paths[0])?;
        
        for file_path in &file_paths[1..] {
            let data = Self::load_from_file(file_path)?;
            merged_data.merge(data);
        }
        
        Ok(merged_data)
    }
}

impl GitLabData {
    pub fn total_items(&self) -> usize {
        self.commits.len() + 
        self.merge_requests.len() + 
        self.issues.len() + 
        self.comments.len()
    }
}

impl GitHubData {
    pub fn total_items(&self) -> usize {
        self.commits.len() + 
        self.pull_requests.len() + 
        self.issues.len() + 
        self.comments.len()
    }
}

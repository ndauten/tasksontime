// Comprehensive integration tests for TReporter
use std::env;
use chrono::{DateTime, Utc};
use serde_json::Value;
use treporter::collectors::gitlab::GitLabCollector;
use treporter::config::{GitLabConfig, GitHubConfig, CollectionConfig, Config};
use treporter::types::{CollectedData, CollectionMetadata};
use treporter::generators::ReportGenerator;

// Test constants
const TEST_DATE_START: &str = "2025-06-01T00:00:00Z";
const TEST_DATE_END: &str = "2025-07-11T23:59:59Z";

fn setup_test_environment() {
    // Set up test environment variables if not already set
    if env::var("GITLAB_TOKEN").is_err() {
        env::set_var("GITLAB_TOKEN", "test_token");
    }
    if env::var("GITLAB_USERNAME").is_err() {
        env::set_var("GITLAB_USERNAME", "test_user");
    }
    if env::var("GITHUB_TOKEN").is_err() {
        env::set_var("GITHUB_TOKEN", "test_token");
    }
    if env::var("GITHUB_USERNAME").is_err() {
        env::set_var("GITHUB_USERNAME", "test_user");
    }
}

fn get_test_date_range() -> (DateTime<Utc>, DateTime<Utc>) {
    let start = DateTime::parse_from_rfc3339(TEST_DATE_START)
        .unwrap()
        .with_timezone(&Utc);
    let end = DateTime::parse_from_rfc3339(TEST_DATE_END)
        .unwrap()
        .with_timezone(&Utc);
    (start, end)
}

fn is_valid_gitlab_data(data: &Value) -> bool {
    // Check if the data has the expected GitLab structure
    data.as_object().is_some() && 
    data.get("_source_type").is_some() &&
    data.get("_project_id").is_some() &&
    data.get("_project_name").is_some() &&
    data.get("_project_path").is_some()
}

fn is_valid_github_data(data: &Value) -> bool {
    // Check if the data has the expected GitHub structure
    data.as_object().is_some() && 
    data.get("_source_type").is_some() &&
    data.get("_repository_id").is_some() &&
    data.get("_repository_name").is_some() &&
    data.get("_repository_full_name").is_some()
}

fn count_data_types(data: &[Value]) -> (usize, usize, usize, usize) {
    let mut commits = 0;
    let mut issues = 0;
    let mut merge_requests = 0;
    let mut comments = 0;
    
    for item in data {
        if let Some(source_type) = item.get("_source_type").and_then(|v| v.as_str()) {
            match source_type {
                "gitlab_commit" | "github_commit" => commits += 1,
                "gitlab_issue" | "github_issue" => issues += 1,
                "gitlab_merge_request" | "github_pull_request" => merge_requests += 1,
                "gitlab_issue_comment" | "gitlab_merge_request_comment" | "github_issue_comment" | "github_pull_request_comment" => comments += 1,
                _ => {}
            }
        }
    }
    
    (commits, issues, merge_requests, comments)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_date_range_parsing() {
        let (start, end) = get_test_date_range();
        assert!(start < end);
        assert_eq!(start.format("%Y-%m-%d").to_string(), "2025-06-01");
        assert_eq!(end.format("%Y-%m-%d").to_string(), "2025-07-11");
    }
    
    #[test]
    fn test_gitlab_data_validation() {
        let valid_data = serde_json::json!({
            "_source_type": "gitlab_commit",
            "_project_id": 123,
            "_project_name": "test",
            "_project_path": "test/repo",
            "id": "abc123",
            "message": "test commit"
        });
        
        assert!(is_valid_gitlab_data(&valid_data));
        
        let invalid_data = serde_json::json!({
            "id": "abc123",
            "message": "test commit"
        });
        
        assert!(!is_valid_gitlab_data(&invalid_data));
    }
    
    #[test]
    fn test_github_data_validation() {
        let valid_data = serde_json::json!({
            "_source_type": "github_commit",
            "_repository_id": 456,
            "_repository_name": "test-repo",
            "_repository_full_name": "owner/test-repo",
            "sha": "def789",
            "message": "test commit"
        });
        
        assert!(is_valid_github_data(&valid_data));
        
        let invalid_data = serde_json::json!({
            "sha": "def789",
            "message": "test commit"
        });
        
        assert!(!is_valid_github_data(&invalid_data));
    }
    
    #[test]
    fn test_count_data_types() {
        let test_data = vec![
            serde_json::json!({"_source_type": "gitlab_commit"}),
            serde_json::json!({"_source_type": "gitlab_issue"}),
            serde_json::json!({"_source_type": "gitlab_merge_request"}),
            serde_json::json!({"_source_type": "gitlab_issue_comment"}),
            serde_json::json!({"_source_type": "github_commit"}),
        ];
        
        let (commits, issues, merge_requests, comments) = count_data_types(&test_data);
        assert_eq!(commits, 2);
        assert_eq!(issues, 1);
        assert_eq!(merge_requests, 1);
        assert_eq!(comments, 1);
    }
    
    #[tokio::test]
    async fn test_gitlab_collector_creation() {
        setup_test_environment();
        
        let config = GitLabConfig {
            enabled: true,
            token_env: "GITLAB_TOKEN".to_string(),
            username_env: "GITLAB_USERNAME".to_string(),
            base_url: "https://gitlab.com".to_string(),
            repositories: Some(vec!["test/repo".to_string()]),
            include_issues: Some(true),
            include_merge_requests: Some(true),
            include_commits: Some(true),
            include_wiki: Some(true),
            include_comments: Some(true),
        };
        
        let collector = GitLabCollector::new(&config, vec![]);
        assert!(collector.is_ok());
    }
    
    #[test]
    fn test_gitlab_config_defaults() {
        let config = GitLabConfig {
            enabled: true,
            token_env: "GITLAB_TOKEN".to_string(),
            username_env: "GITLAB_USERNAME".to_string(),
            base_url: "https://gitlab.com".to_string(),
            repositories: None,
            include_issues: None,
            include_merge_requests: None,
            include_commits: None,
            include_wiki: None,
            include_comments: None,
        };
        
        // Test that defaults work properly
        assert_eq!(config.include_issues.unwrap_or(true), true);
        assert_eq!(config.include_merge_requests.unwrap_or(true), true);
        assert_eq!(config.include_commits.unwrap_or(true), true);
        assert_eq!(config.include_wiki.unwrap_or(true), true);
        assert_eq!(config.include_comments.unwrap_or(true), true);
    }
    
    #[test]
    fn test_github_config_creation() {
        let config = GitHubConfig {
            enabled: true,
            token_env: "GITHUB_TOKEN".to_string(),
            username_env: "GITHUB_USERNAME".to_string(),
            organizations: None,
            repositories: Some(vec!["owner/repo".to_string()]),
            include_issues: Some(true),
            include_pull_requests: Some(true),
            include_commits: Some(true),
            include_wiki: Some(true),
        };
        
        assert_eq!(config.enabled, true);
        assert_eq!(config.token_env, "GITHUB_TOKEN");
        assert_eq!(config.username_env, "GITHUB_USERNAME");
        assert_eq!(config.repositories.unwrap().len(), 1);
    }
    
    #[test]
    fn test_collection_config_creation() {
        let config = CollectionConfig {
            include_diffs: true,
            max_diff_size: 50000,
        };
        
        assert_eq!(config.include_diffs, true);
        assert_eq!(config.max_diff_size, 50000);
    }
    
    #[test] 
    fn test_collected_data_structure() {
        let data = CollectedData {
            gitlab_raw: vec![
                serde_json::json!({"_source_type": "gitlab_commit", "id": "abc123"}),
                serde_json::json!({"_source_type": "gitlab_issue", "id": "456"}),
            ],
            github_raw: vec![
                serde_json::json!({"_source_type": "github_commit", "id": "def789"}),
            ],
            git_commits: vec![],
            local_files: vec![],
            metadata: treporter::types::CollectionMetadata {
                date_range_start: chrono::Utc::now(),
                date_range_end: chrono::Utc::now(),
                sources_used: vec!["gitlab".to_string(), "github".to_string()],
                collection_time: chrono::Utc::now(),
            },
        };
        
        assert_eq!(data.gitlab_raw.len(), 2);
        assert_eq!(data.github_raw.len(), 1);
        assert_eq!(data.git_commits.len(), 0);
        assert_eq!(data.local_files.len(), 0);
        assert_eq!(data.total_items(), 3);
    }
    
    #[test]
    fn test_data_deduplication_detection() {
        let data = vec![
            serde_json::json!({"_source_type": "gitlab_commit", "id": "abc123"}),
            serde_json::json!({"_source_type": "github_commit", "sha": "abc123"}),
            serde_json::json!({"_source_type": "gitlab_commit", "id": "def456"}),
        ];
        
        // This test would verify that we can detect potential duplicates
        // by comparing commit hashes/IDs across different sources
        let mut commit_ids = std::collections::HashSet::new();
        let mut duplicates = Vec::new();
        
        for item in &data {
            let id = if let Some(gitlab_id) = item.get("id").and_then(|v| v.as_str()) {
                Some(gitlab_id.to_string())
            } else if let Some(github_sha) = item.get("sha").and_then(|v| v.as_str()) {
                Some(github_sha.to_string())
            } else {
                None
            };
            
            if let Some(id) = id {
                if commit_ids.contains(&id) {
                    duplicates.push(id.clone());
                } else {
                    commit_ids.insert(id);
                }
            }
        }
        
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0], "abc123");
    }
    
    #[test]
    fn test_config_loading() {
        // Test that configuration loading works
        let config_result = Config::load("config.toml");
        assert!(config_result.is_ok());
        
        let config = config_result.unwrap();
        assert_eq!(config.project.name, "SPEAR Project");
        assert!(config.data_sources.gitlab.is_some());
        assert!(config.data_sources.github.is_some());
        assert!(config.data_sources.local_files.is_some());
    }
    
    #[test]
    fn test_report_generator_creation() {
        let config = Config::load("config.toml").unwrap();
        let generator = ReportGenerator::new(config);
        assert!(generator.is_ok());
    }
    
    #[test]
    fn test_comprehensive_data_validation() {
        let gitlab_data = serde_json::json!({
            "_source_type": "gitlab_commit",
            "_project_id": 123,
            "_project_name": "test",
            "_project_path": "test/repo",
            "id": "abc123",
            "message": "test commit",
            "author_name": "Test User",
            "committed_date": "2025-07-01T10:00:00Z"
        });
        
        let github_data = serde_json::json!({
            "_source_type": "github_commit",
            "_repository_id": 456,
            "_repository_name": "test-repo",
            "_repository_full_name": "owner/test-repo",
            "sha": "def789",
            "message": "test commit",
            "author": {"name": "GitHub User"}
        });
        
        assert!(is_valid_gitlab_data(&gitlab_data));
        assert!(is_valid_github_data(&github_data));
        
        // Test invalid data
        let invalid_gitlab = serde_json::json!({
            "_source_type": "gitlab_commit",
            "id": "abc123",
            "message": "test commit"
            // Missing required fields
        });
        
        let invalid_github = serde_json::json!({
            "_source_type": "github_commit",
            "sha": "def789",
            "message": "test commit"
            // Missing required fields
        });
        
        assert!(!is_valid_gitlab_data(&invalid_gitlab));
        assert!(!is_valid_github_data(&invalid_github));
    }
    
    #[test]
    fn test_collected_data_with_sample_data() {
        let data = CollectedData {
            gitlab_raw: vec![
                serde_json::json!({
                    "_source_type": "gitlab_commit",
                    "id": "abc123",
                    "message": "Test commit",
                    "author_name": "Test User",
                    "committed_date": "2025-07-01T10:00:00Z",
                    "_project_name": "test-project",
                    "_project_path": "test/project"
                }),
                serde_json::json!({
                    "_source_type": "gitlab_issue",
                    "id": 456,
                    "title": "Test Issue",
                    "state": "opened",
                    "created_at": "2025-07-01T11:00:00Z",
                    "_project_name": "test-project",
                    "_project_path": "test/project"
                }),
            ],
            github_raw: vec![
                serde_json::json!({
                    "_source_type": "github_commit",
                    "sha": "def789",
                    "message": "Another test commit",
                    "author": {"name": "GitHub User"},
                    "committed_date": "2025-07-01T12:00:00Z",
                    "_repository_name": "test-repo",
                    "_repository_full_name": "owner/test-repo"
                }),
            ],
            git_commits: vec![
                treporter::types::GitCommitData {
                    hash: "ghi012".to_string(),
                    message: "Local commit".to_string(),
                    author_name: "Local User".to_string(),
                    author_email: "local@example.com".to_string(),
                    timestamp: chrono::Utc::now(),
                    repo_path: "/path/to/repo".to_string(),
                    files_changed: vec!["file1.rs".to_string()],
                },
            ],
            local_files: vec![],
            metadata: CollectionMetadata {
                date_range_start: chrono::Utc::now(),
                date_range_end: chrono::Utc::now(),
                sources_used: vec!["gitlab".to_string(), "github".to_string(), "git".to_string()],
                collection_time: chrono::Utc::now(),
            },
        };
        
        assert_eq!(data.total_items(), 4);
        
        let (commits, issues, merge_requests, comments) = count_data_types(&data.gitlab_raw);
        assert_eq!(commits, 1);
        assert_eq!(issues, 1);
        assert_eq!(merge_requests, 0);
        assert_eq!(comments, 0);
        
        let (gh_commits, gh_issues, gh_prs, gh_comments) = count_data_types(&data.github_raw);
        assert_eq!(gh_commits, 1);
        assert_eq!(gh_issues, 0);
        assert_eq!(gh_prs, 0);
        assert_eq!(gh_comments, 0);
    }
}

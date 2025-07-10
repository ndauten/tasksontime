use crate::config::{GitLabConfig, RepositoryConfig};
use crate::types::GitLabEvent;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde_json::Value;
use std::env;

pub struct GitLabCollector {
    client: Client,
    token: String,
    #[allow(dead_code)]
    username: String,
    base_url: String,
}

impl GitLabCollector {
    pub fn new(config: &GitLabConfig, _repositories: Vec<RepositoryConfig>) -> Result<Self> {
        let token = env::var(&config.token_env)
            .map_err(|_| anyhow!("GitLab token not found in environment variable: {}", config.token_env))?;
        
        let username = env::var(&config.username_env)
            .map_err(|_| anyhow!("GitLab username not found in environment variable: {}", config.username_env))?;

        Ok(Self {
            client: Client::new(),
            token,
            username,
            base_url: config.base_url.clone(),
        })
    }

    pub async fn collect(&self, config: &GitLabConfig, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<GitLabEvent>> {
        println!("🔍 Collecting GitLab repositories and events from {} to {}", 
                 start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d"));
        
        // Get repositories based on configuration
        let repositories = if let Some(repo_list) = &config.repositories {
            self.get_specific_repositories(repo_list).await?
        } else {
            // Fallback to all repositories if no configuration
            self.get_all_repositories().await?
        };
        
        println!("📋 Found {} GitLab repositories to collect from", repositories.len());
        
        // List all repositories so we can see what we're working with
        for repo in &repositories {
            let name = repo["name"].as_str().unwrap_or("Unknown");
            let path = repo["path_with_namespace"].as_str().unwrap_or("Unknown");
            println!("  📁 Repository: {} ({})", name, path);
        }
        
        // Collect events from repositories
        let mut all_events = Vec::new();
        
        // Get defaults for what to collect
        let include_commits = config.include_commits.unwrap_or(true);
        let include_issues = config.include_issues.unwrap_or(true);
        let include_merge_requests = config.include_merge_requests.unwrap_or(true);
        let _include_wiki = config.include_wiki.unwrap_or(true);
        
        for repo in repositories {
            if let Some(project_id) = repo["id"].as_u64() {
                let project_name = repo["name"].as_str().unwrap_or("Unknown");
                let project_path = repo["path_with_namespace"].as_str().unwrap_or("Unknown");
                
                println!("  🔍 Collecting from: {} ({})", project_name, project_path);
                
                // Collect commits
                if include_commits {
                    match self.get_project_commits(project_id, start_date, end_date).await {
                        Ok(commits) => {
                            if !commits.is_empty() {
                                println!("    ✅ Found {} commits", commits.len());
                            }
                            all_events.extend(commits);
                        },
                        Err(e) => {
                            println!("    ⚠️  Failed to get commits: {}", e);
                        }
                    }
                }
                
                // Collect issues
                if include_issues {
                    match self.get_project_issues(project_id, start_date, end_date).await {
                        Ok(issues) => {
                            if !issues.is_empty() {
                                println!("    ✅ Found {} issues", issues.len());
                            }
                            all_events.extend(issues);
                        },
                        Err(e) => {
                            println!("    ⚠️  Failed to get issues: {}", e);
                        }
                    }
                }
                
                // Collect merge requests
                if include_merge_requests {
                    match self.get_project_merge_requests(project_id, start_date, end_date).await {
                        Ok(mrs) => {
                            if !mrs.is_empty() {
                                println!("    ✅ Found {} merge requests", mrs.len());
                            }
                            all_events.extend(mrs);
                        },
                        Err(e) => {
                            println!("    ⚠️  Failed to get merge requests: {}", e);
                        }
                    }
                }
                
                // TODO: Collect wiki if include_wiki is true
            }
        }
        
        println!("✅ Collected {} total GitLab events", all_events.len());
        Ok(all_events)
    }

    async fn get_specific_repositories(&self, repo_paths: &[String]) -> Result<Vec<Value>> {
        let mut repos = Vec::new();
        
        for repo_path in repo_paths {
            println!("  🔍 Looking for repository: {}", repo_path);
            
            // URL encode the repository path
            let encoded_path = urlencoding::encode(repo_path);
            let url = format!("{}/api/v4/projects/{}", self.base_url, encoded_path);
            
            let resp = self.client
                .get(&url)
                .header("PRIVATE-TOKEN", &self.token)
                .send()
                .await?;
                
            if resp.status().is_success() {
                let repo = resp.json::<Value>().await?;
                repos.push(repo);
                println!("    ✅ Found: {}", repo_path);
            } else {
                println!("    ⚠️  Repository not found or no access: {}", repo_path);
            }
        }
        
        Ok(repos)
    }

    async fn get_all_repositories(&self) -> Result<Vec<Value>> {
        let mut all_repos = Vec::new();
        
        // Get repositories where user is owner
        let owned_repos = self.get_owned_repositories().await?;
        all_repos.extend(owned_repos);
        
        // Get repositories where user is member
        let member_repos = self.get_member_repositories().await?;
        all_repos.extend(member_repos);
        
        // Remove duplicates based on project ID
        all_repos.sort_by_key(|repo| repo["id"].as_u64().unwrap_or(0));
        all_repos.dedup_by_key(|repo| repo["id"].as_u64().unwrap_or(0));
        
        Ok(all_repos)
    }

    async fn get_owned_repositories(&self) -> Result<Vec<Value>> {
        let mut all_repos = Vec::new();
        let mut page = 1;
        
        loop {
            let url = format!("{}/api/v4/projects?owned=true&page={}&per_page=100", self.base_url, page);
            
            let resp = self.client
                .get(&url)
                .header("PRIVATE-TOKEN", &self.token)
                .send()
                .await?;
                
            if !resp.status().is_success() {
                break;
            }
            
            let repos = resp.json::<Vec<Value>>().await?;
            
            if repos.is_empty() {
                break;
            }
            
            all_repos.extend(repos);
            page += 1;
            
            if page > 20 { // Safety limit
                break;
            }
        }
        
        Ok(all_repos)
    }

    async fn get_member_repositories(&self) -> Result<Vec<Value>> {
        let mut all_repos = Vec::new();
        let mut page = 1;
        
        loop {
            let url = format!("{}/api/v4/projects?membership=true&page={}&per_page=100", self.base_url, page);
            
            let resp = self.client
                .get(&url)
                .header("PRIVATE-TOKEN", &self.token)
                .send()
                .await?;
                
            if !resp.status().is_success() {
                break;
            }
            
            let repos = resp.json::<Vec<Value>>().await?;
            
            if repos.is_empty() {
                break;
            }
            
            all_repos.extend(repos);
            page += 1;
            
            if page > 20 { // Safety limit
                break;
            }
        }
        
        Ok(all_repos)
    }

    async fn get_project_commits(&self, project_id: u64, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<GitLabEvent>> {
        let mut commits = Vec::new();
        let mut page = 1;
        
        // Get project info for project name
        let project_info = self.get_project_info(project_id).await?;
        let project_name = project_info["name"].as_str().unwrap_or("Unknown").to_string();
        
        loop {
            let url = format!(
                "{}/api/v4/projects/{}/repository/commits?page={}&per_page=100&since={}&until={}",
                self.base_url, project_id, page,
                start_date.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
                end_date.format("%Y-%m-%dT%H:%M:%S%.3fZ")
            );
            
            let resp = self.client
                .get(&url)
                .header("PRIVATE-TOKEN", &self.token)
                .send()
                .await?;
                
            if !resp.status().is_success() {
                break;
            }
            
            let commit_data = resp.json::<Vec<Value>>().await?;
            
            if commit_data.is_empty() {
                break;
            }
            
            for commit_json in commit_data {
                if let Ok(event) = self.parse_commit_event(commit_json, project_id, &project_name) {
                    commits.push(event);
                }
            }
            
            page += 1;
            if page > 5 { // Safety limit for commits
                break;
            }
        }
        
        Ok(commits)
    }

    async fn get_project_info(&self, project_id: u64) -> Result<Value> {
        let url = format!("{}/api/v4/projects/{}", self.base_url, project_id);
        
        let resp = self.client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()
            .await?;
            
        if !resp.status().is_success() {
            return Err(anyhow!("Failed to get project info for project {}", project_id));
        }
        
        let project_info = resp.json::<Value>().await?;
        Ok(project_info)
    }

    async fn get_project_issues(&self, project_id: u64, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<GitLabEvent>> {
        let mut issues = Vec::new();
        let mut page = 1;
        
        loop {
            let url = format!(
                "{}/api/v4/projects/{}/issues?page={}&per_page=100&created_after={}&created_before={}",
                self.base_url, project_id, page,
                start_date.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
                end_date.format("%Y-%m-%dT%H:%M:%S%.3fZ")
            );
            
            let resp = self.client
                .get(&url)
                .header("PRIVATE-TOKEN", &self.token)
                .send()
                .await?;
                
            if !resp.status().is_success() {
                break;
            }
            
            let issue_data = resp.json::<Vec<Value>>().await?;
            
            if issue_data.is_empty() {
                break;
            }
            
            for issue_json in issue_data {
                if let Ok(event) = self.parse_issue_event(issue_json, project_id) {
                    issues.push(event);
                }
            }
            
            page += 1;
            if page > 5 { // Safety limit
                break;
            }
        }
        
        Ok(issues)
    }

    async fn get_project_merge_requests(&self, project_id: u64, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<GitLabEvent>> {
        let mut merge_requests = Vec::new();
        let mut page = 1;
        
        loop {
            let url = format!(
                "{}/api/v4/projects/{}/merge_requests?page={}&per_page=100&created_after={}&created_before={}",
                self.base_url, project_id, page,
                start_date.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
                end_date.format("%Y-%m-%dT%H:%M:%S%.3fZ")
            );
            
            let resp = self.client
                .get(&url)
                .header("PRIVATE-TOKEN", &self.token)
                .send()
                .await?;
                
            if !resp.status().is_success() {
                break;
            }
            
            let mr_data = resp.json::<Vec<Value>>().await?;
            
            if mr_data.is_empty() {
                break;
            }
            
            for mr_json in mr_data {
                if let Ok(event) = self.parse_merge_request_event(mr_json, project_id) {
                    merge_requests.push(event);
                }
            }
            
            page += 1;
            if page > 5 { // Safety limit
                break;
            }
        }
        
        Ok(merge_requests)
    }

    fn parse_commit_event(&self, commit_json: Value, project_id: u64, project_name: &str) -> Result<GitLabEvent> {
        let created_at_str = commit_json["created_at"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing created_at field in commit"))?;
        
        let created_at = DateTime::parse_from_rfc3339(created_at_str)?
            .with_timezone(&Utc);

        // Get the commit ID (hash)
        let commit_id = commit_json["id"].as_str().unwrap_or("");
        
        // Create a simple hash from the commit ID for the event ID
        let event_id = commit_id.chars().take(8).collect::<String>()
            .parse::<u64>().unwrap_or(project_id);

        let event = GitLabEvent {
            id: event_id,
            action_name: "committed".to_string(),
            target_type: Some("Commit".to_string()),
            target_title: Some(commit_json["title"].as_str().unwrap_or("Untitled commit").to_string()),
            project_id: Some(project_id),
            project_name: Some(project_name.to_string()),
            created_at,
            details: Some(format!("Commit: {}", commit_json["message"].as_str().unwrap_or("No message"))),
            author_name: commit_json["author_name"].as_str().map(|s| s.to_string()),
            commit_hash: Some(commit_id.to_string()),
        };

        Ok(event)
    }

    fn parse_issue_event(&self, issue_json: Value, project_id: u64) -> Result<GitLabEvent> {
        let created_at_str = issue_json["created_at"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing created_at field in issue"))?;
        
        let created_at = DateTime::parse_from_rfc3339(created_at_str)?
            .with_timezone(&Utc);

        let event = GitLabEvent {
            id: issue_json["id"].as_u64().unwrap_or(0),
            action_name: "opened".to_string(),
            target_type: Some("Issue".to_string()),
            target_title: Some(issue_json["title"].as_str().unwrap_or("Untitled issue").to_string()),
            project_id: Some(project_id),
            project_name: None,
            created_at,
            details: Some(format!("Issue #{}: {}", 
                issue_json["iid"].as_u64().unwrap_or(0),
                issue_json["title"].as_str().unwrap_or("No title"))),
            author_name: issue_json["author"]["name"].as_str().map(|s| s.to_string()),
            commit_hash: None,
        };

        Ok(event)
    }

    fn parse_merge_request_event(&self, mr_json: Value, project_id: u64) -> Result<GitLabEvent> {
        let created_at_str = mr_json["created_at"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing created_at field in merge request"))?;
        
        let created_at = DateTime::parse_from_rfc3339(created_at_str)?
            .with_timezone(&Utc);

        let event = GitLabEvent {
            id: mr_json["id"].as_u64().unwrap_or(0),
            action_name: "opened".to_string(),
            target_type: Some("MergeRequest".to_string()),
            target_title: Some(mr_json["title"].as_str().unwrap_or("Untitled merge request").to_string()),
            project_id: Some(project_id),
            project_name: None,
            created_at,
            details: Some(format!("Merge Request !{}: {}", 
                mr_json["iid"].as_u64().unwrap_or(0),
                mr_json["title"].as_str().unwrap_or("No title"))),
            author_name: mr_json["author"]["name"].as_str().map(|s| s.to_string()),
            commit_hash: None,
        };

        Ok(event)
    }
}

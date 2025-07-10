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

    pub async fn collect(&self, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<GitLabEvent>> {
        println!("🔍 Collecting GitLab repositories and commits from {} to {}", 
                 start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d"));
        
        // Step 1: Get ALL repositories - both owned and member
        let all_repos = self.get_all_repositories().await?;
        
        println!("📋 Found {} total GitLab repositories", all_repos.len());
        
        // Step 2: List all repositories so we can see what we're working with
        for repo in &all_repos {
            let name = repo["name"].as_str().unwrap_or("Unknown");
            let path = repo["path_with_namespace"].as_str().unwrap_or("Unknown");
            println!("  📁 Repository: {} ({})", name, path);
        }
        
        // Step 3: Collect commits from ALL repositories
        let mut all_events = Vec::new();
        
        for repo in all_repos {
            if let Some(project_id) = repo["id"].as_u64() {
                let project_name = repo["name"].as_str().unwrap_or("Unknown");
                let project_path = repo["path_with_namespace"].as_str().unwrap_or("Unknown");
                
                println!("  🔍 Collecting commits from: {} ({})", project_name, project_path);
                
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
        }
        
        println!("✅ Collected {} total GitLab events", all_events.len());
        Ok(all_events)
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
                if let Ok(event) = self.parse_commit_event(commit_json, project_id) {
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

    fn parse_commit_event(&self, commit_json: Value, project_id: u64) -> Result<GitLabEvent> {
        let created_at_str = commit_json["created_at"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing created_at field in commit"))?;
        
        let created_at = DateTime::parse_from_rfc3339(created_at_str)?
            .with_timezone(&Utc);

        // Create a simple hash from the commit ID for the event ID
        let commit_id = commit_json["id"].as_str().unwrap_or("");
        let event_id = commit_id.chars().take(8).collect::<String>()
            .parse::<u64>().unwrap_or(project_id);

        let event = GitLabEvent {
            id: event_id,
            action_name: "committed".to_string(),
            target_type: Some("Commit".to_string()),
            target_title: Some(commit_json["title"].as_str().unwrap_or("Untitled commit").to_string()),
            project_id: Some(project_id),
            project_name: None,
            created_at,
            details: Some(format!("Commit: {}", commit_json["message"].as_str().unwrap_or("No message"))),
            author_name: commit_json["author_name"].as_str().map(|s| s.to_string()),
        };

        Ok(event)
    }
}
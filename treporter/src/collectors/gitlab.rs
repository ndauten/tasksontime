use crate::config::{GitLabConfig, RepositoryConfig};
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

    pub async fn collect(&self, config: &GitLabConfig, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<Value>> {
        println!("🔍 Collecting GitLab raw data from {} to {}", 
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
        
        // Collect raw data from repositories
        let mut all_raw_data = Vec::new();
        
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
                    match self.get_project_commits_raw(project_id, &repo, start_date, end_date).await {
                        Ok(commits) => {
                            if !commits.is_empty() {
                                println!("    ✅ Found {} commits", commits.len());
                            }
                            all_raw_data.extend(commits);
                        },
                        Err(e) => {
                            println!("    ⚠️  Failed to get commits: {}", e);
                        }
                    }
                }
                
                // Collect issues
                if include_issues {
                    match self.get_project_issues_raw(project_id, &repo, start_date, end_date).await {
                        Ok(issues) => {
                            if !issues.is_empty() {
                                println!("    ✅ Found {} issues", issues.len());
                            }
                            all_raw_data.extend(issues);
                        },
                        Err(e) => {
                            println!("    ⚠️  Failed to get issues: {}", e);
                        }
                    }
                }
                
                // Collect merge requests
                if include_merge_requests {
                    match self.get_project_merge_requests_raw(project_id, &repo, start_date, end_date).await {
                        Ok(mrs) => {
                            if !mrs.is_empty() {
                                println!("    ✅ Found {} merge requests", mrs.len());
                            }
                            all_raw_data.extend(mrs);
                        },
                        Err(e) => {
                            println!("    ⚠️  Failed to get merge requests: {}", e);
                        }
                    }
                }
                
                // TODO: Collect wiki if include_wiki is true
            }
        }
        
        println!("✅ Collected {} total GitLab raw data items", all_raw_data.len());
        Ok(all_raw_data)
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

    async fn get_project_commits_raw(&self, project_id: u64, repo_info: &Value, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<Value>> {
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
            
            for mut commit_json in commit_data {
                // Add metadata about the source repository
                if let Some(commit_obj) = commit_json.as_object_mut() {
                    commit_obj.insert("_source_type".to_string(), serde_json::Value::String("gitlab_commit".to_string()));
                    commit_obj.insert("_project_id".to_string(), serde_json::Value::Number(project_id.into()));
                    commit_obj.insert("_project_name".to_string(), serde_json::Value::String(repo_info["name"].as_str().unwrap_or("Unknown").to_string()));
                    commit_obj.insert("_project_path".to_string(), serde_json::Value::String(repo_info["path_with_namespace"].as_str().unwrap_or("Unknown").to_string()));
                }
                commits.push(commit_json);
            }
            
            page += 1;
            if page > 5 { // Safety limit for commits
                break;
            }
        }
        
        Ok(commits)
    }

    async fn get_project_issues_raw(&self, project_id: u64, repo_info: &Value, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<Value>> {
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
            
            for mut issue_json in issue_data {
                // Add metadata about the source repository
                if let Some(issue_obj) = issue_json.as_object_mut() {
                    issue_obj.insert("_source_type".to_string(), serde_json::Value::String("gitlab_issue".to_string()));
                    issue_obj.insert("_project_id".to_string(), serde_json::Value::Number(project_id.into()));
                    issue_obj.insert("_project_name".to_string(), serde_json::Value::String(repo_info["name"].as_str().unwrap_or("Unknown").to_string()));
                    issue_obj.insert("_project_path".to_string(), serde_json::Value::String(repo_info["path_with_namespace"].as_str().unwrap_or("Unknown").to_string()));
                }
                issues.push(issue_json);
            }
            
            page += 1;
            if page > 5 { // Safety limit
                break;
            }
        }
        
        Ok(issues)
    }

    async fn get_project_merge_requests_raw(&self, project_id: u64, repo_info: &Value, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<Value>> {
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
            
            for mut mr_json in mr_data {
                // Add metadata about the source repository
                if let Some(mr_obj) = mr_json.as_object_mut() {
                    mr_obj.insert("_source_type".to_string(), serde_json::Value::String("gitlab_merge_request".to_string()));
                    mr_obj.insert("_project_id".to_string(), serde_json::Value::Number(project_id.into()));
                    mr_obj.insert("_project_name".to_string(), serde_json::Value::String(repo_info["name"].as_str().unwrap_or("Unknown").to_string()));
                    mr_obj.insert("_project_path".to_string(), serde_json::Value::String(repo_info["path_with_namespace"].as_str().unwrap_or("Unknown").to_string()));
                }
                merge_requests.push(mr_json);
            }
            
            page += 1;
            if page > 5 { // Safety limit
                break;
            }
        }
        
        Ok(merge_requests)
    }
}

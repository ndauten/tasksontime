use crate::config::{GitHubConfig, RepositoryConfig, CollectionConfig};
use crate::types::GitHubData;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde_json::Value;
use std::env;

pub struct GitHubCollector {
    client: Client,
    token: String,
    #[allow(dead_code)]
    username: String,
}

impl GitHubCollector {
    pub fn new(config: &GitHubConfig, _repositories: Vec<RepositoryConfig>) -> Result<Self> {
        let token = env::var(&config.token_env)
            .map_err(|_| anyhow!("GitHub token not found in environment variable: {}", config.token_env))?;
        
        let username = env::var(&config.username_env)
            .map_err(|_| anyhow!("GitHub username not found in environment variable: {}", config.username_env))?;

        Ok(Self {
            client: Client::new(),
            token,
            username,
        })
    }

    pub async fn collect(&self, config: &GitHubConfig, collection_config: &CollectionConfig, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<GitHubData> {
        println!("🔍 Collecting GitHub raw data from {} to {}", 
                 start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d"));
        
        // Get repositories based on configuration
        let repositories = if let Some(repo_list) = &config.repositories {
            self.get_specific_repositories(repo_list).await?
        } else {
            // Fallback to organization-based collection if no specific repositories
            self.get_organization_repositories(config).await?
        };
        
        println!("📋 Found {} GitHub repositories to collect from", repositories.len());
        
        // List all repositories so we can see what we're working with
        for repo in &repositories {
            let name = repo["name"].as_str().unwrap_or("Unknown");
            let full_name = repo["full_name"].as_str().unwrap_or("Unknown");
            println!("  📁 Repository: {} ({})", name, full_name);
        }
        
        // Initialize organized data structure
        let mut github_data = GitHubData::default();
        
        // Get defaults for what to collect
        let include_commits = config.include_commits.unwrap_or(true);
        let include_issues = config.include_issues.unwrap_or(true);
        let include_pull_requests = config.include_pull_requests.unwrap_or(true);
        let _include_wiki = config.include_wiki.unwrap_or(true);
        
        for repo in repositories {
            if let Some(repo_name) = repo["full_name"].as_str() {
                let repo_display_name = repo["name"].as_str().unwrap_or("Unknown");
                
                println!("  🔍 Collecting from: {} ({})", repo_display_name, repo_name);
                
                // Collect commits
                if include_commits {
                    match self.get_repository_commits_raw(repo_name, &repo, collection_config, start_date, end_date).await {
                        Ok(commits) => {
                            if !commits.is_empty() {
                                println!("    ✅ Found {} commits", commits.len());
                            }
                            github_data.commits.extend(commits);
                        },
                        Err(e) => {
                            println!("    ⚠️  Failed to get commits: {}", e);
                        }
                    }
                }
                
                // Collect issues
                if include_issues {
                    match self.get_repository_issues_raw(repo_name, &repo, start_date, end_date).await {
                        Ok(issues) => {
                            if !issues.is_empty() {
                                println!("    ✅ Found {} issues", issues.len());
                            }
                            github_data.issues.extend(issues);
                        },
                        Err(e) => {
                            println!("    ⚠️  Failed to get issues: {}", e);
                        }
                    }
                }
                
                // Collect pull requests
                if include_pull_requests {
                    match self.get_repository_pull_requests_raw(repo_name, &repo, collection_config, start_date, end_date).await {
                        Ok(prs) => {
                            if !prs.is_empty() {
                                println!("    ✅ Found {} pull requests", prs.len());
                            }
                            github_data.pull_requests.extend(prs);
                        },
                        Err(e) => {
                            println!("    ⚠️  Failed to get pull requests: {}", e);
                        }
                    }
                }
                
                // TODO: Collect wiki if include_wiki is true
            }
        }
        
        println!("✅ Collected {} total GitHub raw data items", github_data.total_items());
        Ok(github_data)
    }

    async fn get_specific_repositories(&self, repo_paths: &[String]) -> Result<Vec<Value>> {
        let mut repos = Vec::new();
        
        for repo_path in repo_paths {
            println!("  🔍 Looking for repository: {}", repo_path);
            
            let url = format!("https://api.github.com/repos/{}", repo_path);
            
            let resp = self.client
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.token))
                .header("User-Agent", "treporter/1.0")
                .header("Accept", "application/vnd.github+json")
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

    async fn get_organization_repositories(&self, config: &GitHubConfig) -> Result<Vec<Value>> {
        let mut all_repos = Vec::new();
        
        if let Some(organizations) = &config.organizations {
            for org in organizations {
                println!("  🔍 Getting repositories for organization: {}", org);
                
                let mut page = 1;
                loop {
                    let url = format!("https://api.github.com/orgs/{}/repos?page={}&per_page=100", org, page);
                    
                    let resp = self.client
                        .get(&url)
                        .header("Authorization", format!("Bearer {}", self.token))
                        .header("User-Agent", "treporter/1.0")
                        .header("Accept", "application/vnd.github+json")
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
            }
        }
        
        Ok(all_repos)
    }

    async fn get_repository_commits_raw(&self, repo_name: &str, repo_info: &Value, collection_config: &CollectionConfig, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<Value>> {
        let mut commits = Vec::new();
        let mut page = 1;
        
        loop {
            let url = format!(
                "https://api.github.com/repos/{}/commits?page={}&per_page=100&since={}&until={}",
                repo_name, page,
                start_date.format("%Y-%m-%dT%H:%M:%SZ"),
                end_date.format("%Y-%m-%dT%H:%M:%SZ")
            );
            
            let resp = self.client
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.token))
                .header("User-Agent", "treporter/1.0")
                .header("Accept", "application/vnd.github+json")
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
                // Extract commit SHA before modifying the object
                let commit_sha = commit_json["sha"].as_str().map(|s| s.to_string());
                
                // Add metadata about the source repository
                if let Some(commit_obj) = commit_json.as_object_mut() {
                    commit_obj.insert("_source_type".to_string(), serde_json::Value::String("github_commit".to_string()));
                    commit_obj.insert("_repo_name".to_string(), serde_json::Value::String(repo_info["name"].as_str().unwrap_or("Unknown").to_string()));
                    commit_obj.insert("_repo_full_name".to_string(), serde_json::Value::String(repo_info["full_name"].as_str().unwrap_or("Unknown").to_string()));
                    
                    // Fetch diff if enabled
                    if collection_config.include_diffs {
                        if let Some(commit_sha_str) = commit_sha {
                            match self.get_commit_diff(repo_name, &commit_sha_str, collection_config.max_diff_size).await {
                                Ok(diff) => {
                                    commit_obj.insert("_diff".to_string(), serde_json::Value::String(diff));
                                },
                                Err(e) => {
                                    println!("    ⚠️  Failed to get diff for commit {}: {}", &commit_sha_str[0..8], e);
                                    commit_obj.insert("_diff_error".to_string(), serde_json::Value::String(e.to_string()));
                                }
                            }
                        }
                    }
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

    async fn get_repository_issues_raw(&self, repo_name: &str, repo_info: &Value, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<Value>> {
        let mut issues = Vec::new();
        let mut page = 1;
        
        loop {
            let url = format!(
                "https://api.github.com/repos/{}/issues?page={}&per_page=100&state=all&since={}",
                repo_name, page,
                start_date.format("%Y-%m-%dT%H:%M:%SZ")
            );
            
            let resp = self.client
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.token))
                .header("User-Agent", "treporter/1.0")
                .header("Accept", "application/vnd.github+json")
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
                // Filter out pull requests (GitHub API returns PRs as issues)
                if issue_json["pull_request"].is_object() {
                    continue;
                }
                
                // Check if within date range
                if let Some(created_at_str) = issue_json["created_at"].as_str() {
                    if let Ok(created_at) = DateTime::parse_from_rfc3339(created_at_str) {
                        let issue_date = created_at.with_timezone(&Utc);
                        
                        if issue_date >= start_date && issue_date <= end_date {
                            // Add metadata about the source repository
                            if let Some(issue_obj) = issue_json.as_object_mut() {
                                issue_obj.insert("_source_type".to_string(), serde_json::Value::String("github_issue".to_string()));
                                issue_obj.insert("_repo_name".to_string(), serde_json::Value::String(repo_info["name"].as_str().unwrap_or("Unknown").to_string()));
                                issue_obj.insert("_repo_full_name".to_string(), serde_json::Value::String(repo_info["full_name"].as_str().unwrap_or("Unknown").to_string()));
                            }
                            issues.push(issue_json);
                        }
                    }
                }
            }
            
            page += 1;
            if page > 5 { // Safety limit
                break;
            }
        }
        
        Ok(issues)
    }

    async fn get_repository_pull_requests_raw(&self, repo_name: &str, repo_info: &Value, collection_config: &CollectionConfig, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<Value>> {
        let mut pull_requests = Vec::new();
        let mut page = 1;
        
        loop {
            let url = format!(
                "https://api.github.com/repos/{}/pulls?page={}&per_page=100&state=all",
                repo_name, page
            );
            
            let resp = self.client
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.token))
                .header("User-Agent", "treporter/1.0")
                .header("Accept", "application/vnd.github+json")
                .send()
                .await?;
                
            if !resp.status().is_success() {
                break;
            }
            
            let pr_data = resp.json::<Vec<Value>>().await?;
            
            if pr_data.is_empty() {
                break;
            }
            
            for mut pr_json in pr_data {
                // Extract PR number before modifying the object
                let pr_number = pr_json["number"].as_u64();
                
                // Check if within date range
                if let Some(created_at_str) = pr_json["created_at"].as_str() {
                    if let Ok(created_at) = DateTime::parse_from_rfc3339(created_at_str) {
                        let pr_date = created_at.with_timezone(&Utc);
                        
                        if pr_date >= start_date && pr_date <= end_date {
                            // Add metadata about the source repository
                            if let Some(pr_obj) = pr_json.as_object_mut() {
                                pr_obj.insert("_source_type".to_string(), serde_json::Value::String("github_pull_request".to_string()));
                                pr_obj.insert("_repo_name".to_string(), serde_json::Value::String(repo_info["name"].as_str().unwrap_or("Unknown").to_string()));
                                pr_obj.insert("_repo_full_name".to_string(), serde_json::Value::String(repo_info["full_name"].as_str().unwrap_or("Unknown").to_string()));
                                
                                // Fetch diff if enabled
                                if collection_config.include_diffs {
                                    if let Some(pr_num) = pr_number {
                                        match self.get_pull_request_diff(repo_name, pr_num, collection_config.max_diff_size).await {
                                            Ok(diff) => {
                                                pr_obj.insert("_diff".to_string(), serde_json::Value::String(diff));
                                            },
                                            Err(e) => {
                                                println!("    ⚠️  Failed to get diff for PR {}: {}", pr_num, e);
                                                pr_obj.insert("_diff_error".to_string(), serde_json::Value::String(e.to_string()));
                                            }
                                        }
                                    }
                                }
                            }
                            pull_requests.push(pr_json);
                        }
                    }
                }
            }
            
            page += 1;
            if page > 5 { // Safety limit
                break;
            }
        }
        
        Ok(pull_requests)
    }

    /// Fetch the diff for a specific commit
    async fn get_commit_diff(&self, repo_name: &str, commit_sha: &str, max_diff_size: usize) -> Result<String> {
        let url = format!(
            "https://api.github.com/repos/{}/commits/{}",
            repo_name, commit_sha
        );
        
        let resp = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", "treporter/1.0")
            .header("Accept", "application/vnd.github.diff")
            .send()
            .await?;
            
        if !resp.status().is_success() {
            return Err(anyhow!("Failed to fetch commit diff: HTTP {}", resp.status()));
        }
        
        let mut diff_text = resp.text().await?;
        
        // Truncate if too large
        if max_diff_size > 0 && diff_text.len() > max_diff_size {
            diff_text.truncate(max_diff_size);
            diff_text.push_str("\n\n[DIFF TRUNCATED - TOO LARGE]");
        }
        
        Ok(diff_text)
    }

    /// Fetch the diff for a specific pull request
    async fn get_pull_request_diff(&self, repo_name: &str, pr_number: u64, max_diff_size: usize) -> Result<String> {
        let url = format!(
            "https://api.github.com/repos/{}/pulls/{}",
            repo_name, pr_number
        );
        
        let resp = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", "treporter/1.0")
            .header("Accept", "application/vnd.github.diff")
            .send()
            .await?;
            
        if !resp.status().is_success() {
            return Err(anyhow!("Failed to fetch pull request diff: HTTP {}", resp.status()));
        }
        
        let mut diff_text = resp.text().await?;
        
        // Truncate if too large
        if max_diff_size > 0 && diff_text.len() > max_diff_size {
            diff_text.truncate(max_diff_size);
            diff_text.push_str("\n\n[DIFF TRUNCATED - TOO LARGE]");
        }
        
        Ok(diff_text)
    }
}

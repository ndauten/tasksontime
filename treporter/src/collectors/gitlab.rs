use crate::config::{GitLabConfig, RepositoryConfig, CollectionConfig};
use crate::types::GitLabData;
use crate::credentials::GlobalConfig;
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
        // Try to get credentials with priority: env var > global config
        let token = env::var(&config.token_env)
            .or_else(|_| {
                // Try global config
                GlobalConfig::load()
                    .ok()
                    .and_then(|gc| gc.gitlab)
                    .and_then(|gitlab| gitlab.token)
                    .ok_or_else(|| anyhow!("GitLab token not found in environment variable '{}' or global config", config.token_env))
            })?;
        
        let username = env::var(&config.username_env)
            .or_else(|_| {
                // Try global config
                GlobalConfig::load()
                    .ok()
                    .and_then(|gc| gc.gitlab)
                    .and_then(|gitlab| gitlab.username)
                    .ok_or_else(|| anyhow!("GitLab username not found in environment variable '{}' or global config", config.username_env))
            })?;

        Ok(Self {
            client: Client::new(),
            token,
            username,
            base_url: config.base_url.clone(),
        })
    }

    pub async fn collect(&self, config: &GitLabConfig, collection_config: &CollectionConfig, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<GitLabData> {
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
        
        // Initialize organized data structure
        let mut gitlab_data = GitLabData::default();
        
        // Get defaults for what to collect
        let include_commits = config.include_commits.unwrap_or(true);
        let include_issues = config.include_issues.unwrap_or(true);
        let include_merge_requests = config.include_merge_requests.unwrap_or(true);
        let include_comments = config.include_comments.unwrap_or(true);
        let _include_wiki = config.include_wiki.unwrap_or(true);
        
        for repo in repositories {
            if let Some(project_id) = repo["id"].as_u64() {
                let project_name = repo["name"].as_str().unwrap_or("Unknown");
                let project_path = repo["path_with_namespace"].as_str().unwrap_or("Unknown");
                
                println!("  🔍 Collecting from: {} ({})", project_name, project_path);
                
                // Collect commits
                if include_commits {
                    match self.get_project_commits_raw(project_id, &repo, collection_config, start_date, end_date).await {
                        Ok(commits) => {
                            if !commits.is_empty() {
                                println!("    ✅ Found {} commits", commits.len());
                            }
                            gitlab_data.commits.extend(commits);
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
                            gitlab_data.issues.extend(issues);
                        },
                        Err(e) => {
                            println!("    ⚠️  Failed to get issues: {}", e);
                        }
                    }
                }
                
                // Collect merge requests
                if include_merge_requests {
                    match self.get_project_merge_requests_raw(project_id, &repo, collection_config, start_date, end_date).await {
                        Ok(mrs) => {
                            if !mrs.is_empty() {
                                println!("    ✅ Found {} merge requests", mrs.len());
                            }
                            gitlab_data.merge_requests.extend(mrs);
                        },
                        Err(e) => {
                            println!("    ⚠️  Failed to get merge requests: {}", e);
                        }
                    }
                }
                
                // Collect comments (issue comments + merge request comments)
                if include_comments {
                    match self.get_project_comments_raw(project_id, &repo, start_date, end_date).await {
                        Ok(comments) => {
                            if !comments.is_empty() {
                                println!("    ✅ Found {} comments", comments.len());
                            }
                            gitlab_data.comments.extend(comments);
                        },
                        Err(e) => {
                            println!("    ⚠️  Failed to get comments: {}", e);
                        }
                    }
                }
                
                // TODO: Collect wiki if include_wiki is true
            }
        }
        
        println!("✅ Collected {} total GitLab raw data items", gitlab_data.total_items());
        Ok(gitlab_data)
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

    async fn get_project_commits_raw(&self, project_id: u64, repo_info: &Value, collection_config: &CollectionConfig, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<Value>> {
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
                // Extract commit ID before modifying the object
                let commit_id = commit_json["id"].as_str().map(|s| s.to_string());
                
                // Add metadata about the source repository
                if let Some(commit_obj) = commit_json.as_object_mut() {
                    commit_obj.insert("_source_type".to_string(), serde_json::Value::String("gitlab_commit".to_string()));
                    commit_obj.insert("_project_id".to_string(), serde_json::Value::Number(project_id.into()));
                    commit_obj.insert("_project_name".to_string(), serde_json::Value::String(repo_info["name"].as_str().unwrap_or("Unknown").to_string()));
                    commit_obj.insert("_project_path".to_string(), serde_json::Value::String(repo_info["path_with_namespace"].as_str().unwrap_or("Unknown").to_string()));
                    
                    // Fetch diff if enabled
                    if collection_config.include_diffs {
                        if let Some(commit_id_str) = commit_id {
                            match self.get_commit_diff(project_id, &commit_id_str, collection_config.max_diff_size).await {
                                Ok(diff) => {
                                    commit_obj.insert("_diff".to_string(), serde_json::Value::String(diff));
                                },
                                Err(e) => {
                                    println!("    ⚠️  Failed to get diff for commit {}: {}", &commit_id_str[0..8], e);
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

    async fn get_project_merge_requests_raw(&self, project_id: u64, repo_info: &Value, collection_config: &CollectionConfig, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<Value>> {
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
                // Extract MR IID before modifying the object
                let mr_iid = mr_json["iid"].as_u64();
                
                // Add metadata about the source repository
                if let Some(mr_obj) = mr_json.as_object_mut() {
                    mr_obj.insert("_source_type".to_string(), serde_json::Value::String("gitlab_merge_request".to_string()));
                    mr_obj.insert("_project_id".to_string(), serde_json::Value::Number(project_id.into()));
                    mr_obj.insert("_project_name".to_string(), serde_json::Value::String(repo_info["name"].as_str().unwrap_or("Unknown").to_string()));
                    mr_obj.insert("_project_path".to_string(), serde_json::Value::String(repo_info["path_with_namespace"].as_str().unwrap_or("Unknown").to_string()));
                    
                    // Fetch diff if enabled
                    if collection_config.include_diffs {
                        if let Some(mr_iid_val) = mr_iid {
                            match self.get_merge_request_diff(project_id, mr_iid_val, collection_config.max_diff_size).await {
                                Ok(diff) => {
                                    mr_obj.insert("_diff".to_string(), serde_json::Value::String(diff));
                                },
                                Err(e) => {
                                    println!("    ⚠️  Failed to get diff for MR {}: {}", mr_iid_val, e);
                                    mr_obj.insert("_diff_error".to_string(), serde_json::Value::String(e.to_string()));
                                }
                            }
                        }
                    }
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

    async fn get_project_comments_raw(&self, project_id: u64, repo_info: &Value, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<Value>> {
        let mut all_comments = Vec::new();
        
        // Get issue comments
        match self.get_issue_comments_raw(project_id, repo_info, start_date, end_date).await {
            Ok(comments) => all_comments.extend(comments),
            Err(e) => println!("      ⚠️  Failed to get issue comments: {}", e),
        }
        
        // Get merge request comments
        match self.get_merge_request_comments_raw(project_id, repo_info, start_date, end_date).await {
            Ok(comments) => all_comments.extend(comments),
            Err(e) => println!("      ⚠️  Failed to get merge request comments: {}", e),
        }
        
        Ok(all_comments)
    }
    
    async fn get_issue_comments_raw(&self, project_id: u64, repo_info: &Value, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<Value>> {
        let mut all_comments = Vec::new();
        
        // First get all issues to get their IIDs
        let issues_url = format!("{}/api/v4/projects/{}/issues?per_page=100", self.base_url, project_id);
        let resp = self.client
            .get(&issues_url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()
            .await?;
            
        if !resp.status().is_success() {
            return Ok(all_comments);
        }
        
        let issues = resp.json::<Vec<Value>>().await?;
        
        // For each issue, get its comments
        for issue in issues {
            if let Some(issue_iid) = issue["iid"].as_u64() {
                let comments_url = format!("{}/api/v4/projects/{}/issues/{}/notes", self.base_url, project_id, issue_iid);
                
                let resp = self.client
                    .get(&comments_url)
                    .header("PRIVATE-TOKEN", &self.token)
                    .send()
                    .await?;
                    
                if resp.status().is_success() {
                    let comments = resp.json::<Vec<Value>>().await?;
                    
                    for mut comment in comments {
                        // Check if comment is in date range
                        if let Some(created_at_str) = comment["created_at"].as_str() {
                            if let Ok(created_at) = DateTime::parse_from_rfc3339(created_at_str) {
                                let created_at_utc = created_at.with_timezone(&Utc);
                                if created_at_utc >= start_date && created_at_utc <= end_date {
                                    // Add metadata
                                    if let Some(comment_obj) = comment.as_object_mut() {
                                        comment_obj.insert("_source_type".to_string(), serde_json::Value::String("gitlab_issue_comment".to_string()));
                                        comment_obj.insert("_project_id".to_string(), serde_json::Value::Number(project_id.into()));
                                        comment_obj.insert("_project_name".to_string(), serde_json::Value::String(repo_info["name"].as_str().unwrap_or("Unknown").to_string()));
                                        comment_obj.insert("_project_path".to_string(), serde_json::Value::String(repo_info["path_with_namespace"].as_str().unwrap_or("Unknown").to_string()));
                                        comment_obj.insert("_issue_iid".to_string(), serde_json::Value::Number(issue_iid.into()));
                                    }
                                    all_comments.push(comment);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(all_comments)
    }
    
    async fn get_merge_request_comments_raw(&self, project_id: u64, repo_info: &Value, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<Value>> {
        let mut all_comments = Vec::new();
        
        // First get all merge requests to get their IIDs
        let mrs_url = format!("{}/api/v4/projects/{}/merge_requests?per_page=100", self.base_url, project_id);
        let resp = self.client
            .get(&mrs_url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()
            .await?;
            
        if !resp.status().is_success() {
            return Ok(all_comments);
        }
        
        let mrs = resp.json::<Vec<Value>>().await?;
        
        // For each merge request, get its comments
        for mr in mrs {
            if let Some(mr_iid) = mr["iid"].as_u64() {
                let comments_url = format!("{}/api/v4/projects/{}/merge_requests/{}/notes", self.base_url, project_id, mr_iid);
                
                let resp = self.client
                    .get(&comments_url)
                    .header("PRIVATE-TOKEN", &self.token)
                    .send()
                    .await?;
                    
                if resp.status().is_success() {
                    let comments = resp.json::<Vec<Value>>().await?;
                    
                    for mut comment in comments {
                        // Check if comment is in date range
                        if let Some(created_at_str) = comment["created_at"].as_str() {
                            if let Ok(created_at) = DateTime::parse_from_rfc3339(created_at_str) {
                                let created_at_utc = created_at.with_timezone(&Utc);
                                if created_at_utc >= start_date && created_at_utc <= end_date {
                                    // Add metadata
                                    if let Some(comment_obj) = comment.as_object_mut() {
                                        comment_obj.insert("_source_type".to_string(), serde_json::Value::String("gitlab_merge_request_comment".to_string()));
                                        comment_obj.insert("_project_id".to_string(), serde_json::Value::Number(project_id.into()));
                                        comment_obj.insert("_project_name".to_string(), serde_json::Value::String(repo_info["name"].as_str().unwrap_or("Unknown").to_string()));
                                        comment_obj.insert("_project_path".to_string(), serde_json::Value::String(repo_info["path_with_namespace"].as_str().unwrap_or("Unknown").to_string()));
                                        comment_obj.insert("_merge_request_iid".to_string(), serde_json::Value::Number(mr_iid.into()));
                                    }
                                    all_comments.push(comment);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(all_comments)
    }
    
    /// Fetch the diff for a specific commit
    async fn get_commit_diff(&self, project_id: u64, commit_id: &str, max_diff_size: usize) -> Result<String> {
        let url = format!(
            "{}/api/v4/projects/{}/repository/commits/{}/diff",
            self.base_url, project_id, commit_id
        );
        
        let resp = self.client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()
            .await?;
            
        if !resp.status().is_success() {
            return Err(anyhow!("Failed to fetch commit diff: HTTP {}", resp.status()));
        }
        
        let diff_data = resp.json::<Vec<Value>>().await?;
        
        // Convert diff data to a readable format
        let mut diff_text = String::new();
        for diff_item in diff_data {
            if let Some(diff_str) = diff_item["diff"].as_str() {
                // Add file path as header
                if let Some(new_path) = diff_item["new_path"].as_str() {
                    diff_text.push_str(&format!("--- {}\n", new_path));
                }
                
                diff_text.push_str(diff_str);
                diff_text.push('\n');
            }
        }
        
        // Truncate if too large
        if max_diff_size > 0 && diff_text.len() > max_diff_size {
            diff_text.truncate(max_diff_size);
            diff_text.push_str("\n\n[DIFF TRUNCATED - TOO LARGE]");
        }
        
        Ok(diff_text)
    }

    /// Fetch the diff for a specific merge request
    async fn get_merge_request_diff(&self, project_id: u64, mr_iid: u64, max_diff_size: usize) -> Result<String> {
        let url = format!(
            "{}/api/v4/projects/{}/merge_requests/{}/diffs",
            self.base_url, project_id, mr_iid
        );
        
        let resp = self.client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()
            .await?;
            
        if !resp.status().is_success() {
            return Err(anyhow!("Failed to fetch merge request diff: HTTP {}", resp.status()));
        }
        
        let diff_data = resp.json::<Vec<Value>>().await?;
        
        // Convert diff data to a readable format
        let mut diff_text = String::new();
        for diff_item in diff_data {
            if let Some(diff_str) = diff_item["diff"].as_str() {
                // Add file path as header
                if let Some(new_path) = diff_item["new_path"].as_str() {
                    diff_text.push_str(&format!("--- {}\n", new_path));
                }
                
                diff_text.push_str(diff_str);
                diff_text.push('\n');
            }
        }
        
        // Truncate if too large
        if max_diff_size > 0 && diff_text.len() > max_diff_size {
            diff_text.truncate(max_diff_size);
            diff_text.push_str("\n\n[DIFF TRUNCATED - TOO LARGE]");
        }
        
        Ok(diff_text)
    }
}

pub mod gitlab;
pub mod github;
pub mod local_files;
pub mod git;

use crate::config::Config;
use crate::types::CollectedData;
use anyhow::Result;
use chrono::{DateTime, Utc};

pub use gitlab::GitLabCollector;
pub use github::GitHubCollector;
pub use local_files::LocalFilesCollector;
pub use git::GitCollector;

pub struct DataCollector {
    config: Config,
}

impl DataCollector {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn collect(&self, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<CollectedData> {
        let mut data = CollectedData::new();
        data.metadata.date_range_start = start_date;
        data.metadata.date_range_end = end_date;

        // Collect GitLab events
        if let Some(gitlab_config) = &self.config.data_sources.gitlab {
            // Get GitLab repositories from config
            let gitlab_repos: Vec<_> = self.config.repositories.iter()
                .filter(|repo| repo.platform == "gitlab")
                .cloned()
                .collect();
                
            match GitLabCollector::new(gitlab_config, gitlab_repos) {
                Ok(collector) => {
                    match collector.collect(gitlab_config, &self.config.collection, start_date, end_date).await {
                        Ok(events) => {
                            data.gitlab_raw = events;
                            data.metadata.sources_used.push("GitLab".to_string());
                        },
                        Err(e) => println!("⚠️  Failed to collect GitLab data: {}", e),
                    }
                },
                Err(e) => println!("⚠️  Failed to initialize GitLab collector: {}", e),
            }
        }

        // Collect GitHub events
        if let Some(github_config) = &self.config.data_sources.github {
            if github_config.enabled {
                // Get GitHub repositories from config
                let github_repos: Vec<_> = self.config.repositories.iter()
                    .filter(|repo| repo.platform == "github")
                    .cloned()
                    .collect();
                    
                match GitHubCollector::new(github_config, github_repos) {
                    Ok(collector) => {
                        match collector.collect(github_config, &self.config.collection, start_date, end_date).await {
                            Ok(events) => {
                                data.github_raw = events;
                                data.metadata.sources_used.push("GitHub".to_string());
                            },
                            Err(e) => println!("⚠️  Failed to collect GitHub data: {}", e),
                        }
                    },
                    Err(e) => println!("⚠️  Failed to initialize GitHub collector: {}", e),
                }
            }
        }

        // Collect local files
        if let Some(local_files_config) = &self.config.data_sources.local_files {
            match LocalFilesCollector::new(local_files_config) {
                Ok(collector) => {
                    match collector.collect(start_date, end_date) {
                        Ok(files) => {
                            data.local_files = files;
                            data.metadata.sources_used.push("Local Files".to_string());
                        },
                        Err(e) => println!("⚠️  Failed to collect local files data: {}", e),
                    }
                },
                Err(e) => println!("⚠️  Failed to initialize local files collector: {}", e),
            }
        }

        // Collect Git commits
        let git_collector = GitCollector::new(self.config.repositories.clone());
        match git_collector.collect(start_date, end_date) {
            Ok(commits) => {
                data.git_commits = commits;
                data.metadata.sources_used.push("Git Commits".to_string());
            },
            Err(e) => println!("⚠️  Failed to collect Git commits: {}", e),
        }

        Ok(data)
    }
}

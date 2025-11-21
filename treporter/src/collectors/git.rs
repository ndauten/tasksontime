use crate::config::RepositoryConfig;
use crate::types::GitCommitData;
use anyhow::Result;
use chrono::{DateTime, Utc};
use git2::Repository;
use std::path::Path;

pub struct GitCollector {
    repositories: Vec<RepositoryConfig>,
}

impl GitCollector {
    pub fn new(repositories: Vec<RepositoryConfig>) -> Self {
        Self { repositories }
    }

    pub fn collect(&self, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<GitCommitData>> {
        println!("🔍 Collecting Git commits from {} to {}", start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d"));
        
        let mut all_commits = Vec::new();

        for repo_config in &self.repositories {
            if let Some(path) = &repo_config.path {
                if repo_config.include_commits.unwrap_or(true) {
                    match self.collect_from_repository(path, &repo_config.name, start_date, end_date) {
                        Ok(mut commits) => {
                            all_commits.append(&mut commits);
                        },
                        Err(e) => {
                            println!("⚠️  Failed to collect from {}: {}", repo_config.name, e);
                        }
                    }
                }
            }
        }

        println!("✅ Collected {} Git commits", all_commits.len());
        Ok(all_commits)
    }

    fn collect_from_repository(&self, repo_path: &str, repo_name: &str, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<GitCommitData>> {
        let repo = Repository::open(repo_path)?;
        let mut commits = Vec::new();

        // Get the current branch HEAD
        let head = repo.head()?;
        let oid = head.target().unwrap();
        
        // Walk the commit history
        let mut revwalk = repo.revwalk()?;
        revwalk.push(oid)?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        for commit_oid in revwalk {
            let commit_oid = commit_oid?;
            let commit = repo.find_commit(commit_oid)?;
            
            let commit_time = DateTime::from_timestamp(commit.time().seconds(), 0)
                .unwrap_or(Utc::now());

            // Filter by date range
            if commit_time < start_date {
                break; // Since we're walking in chronological order, we can stop here
            }
            
            if commit_time > end_date {
                continue;
            }

            let stats = self.get_commit_stats(&repo, &commit)?;
            
            let commit_data = GitCommitData {
                hash: commit.id().to_string(),
                author_name: commit.author().name().unwrap_or("").to_string(),
                author_email: commit.author().email().unwrap_or("").to_string(),
                message: commit.message().unwrap_or("").to_string(),
                timestamp: commit_time,
                repo_path: repo_name.to_string(),
                files_changed: stats.files_changed,
            };

            commits.push(commit_data);
        }

        Ok(commits)
    }

    fn get_commit_stats(&self, repo: &Repository, commit: &git2::Commit) -> Result<CommitStats> {
        let mut stats = CommitStats::default();

        let tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;

        // Count files changed
        stats.files_changed = diff.deltas().map(|delta| {
            delta.new_file().path().unwrap_or(Path::new("")).to_string_lossy().to_string()
        }).collect::<std::collections::HashSet<_>>().into_iter().collect();

        Ok(stats)
    }
}

#[derive(Default)]
struct CommitStats {
    files_changed: Vec<String>,
}

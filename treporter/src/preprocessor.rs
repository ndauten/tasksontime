use crate::types::{CollectedData, GitLabData, GitHubData, GitCommitData};
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;

pub struct DataPreprocessor;

impl DataPreprocessor {
    pub fn new() -> Self {
        Self
    }

    /// Convert raw collected data into LLM-friendly structured text
    /// This does the heavy lifting locally to minimize API costs
    pub fn preprocess_for_llm(&self, data: &CollectedData) -> Result<String> {
        let mut sections = Vec::new();

        // Process GitLab data
        sections.push(self.process_gitlab_section(&data.gitlab)?);

        // Process GitHub data  
        sections.push(self.process_github_section(&data.github)?);

        // Process Git commits
        if !data.git_commits.is_empty() {
            sections.push(self.process_git_commits(&data.git_commits)?);
        }

        Ok(sections.join("\n\n---\n\n"))
    }

    fn process_gitlab_section(&self, gitlab: &GitLabData) -> Result<String> {
        let mut content = String::from("# GitLab Activity Summary\n\n");

        // Process commits with intelligent summarization
        if !gitlab.commits.is_empty() {
            content.push_str(&self.summarize_commits(&gitlab.commits, "GitLab")?);
        }

        // Process merge requests
        if !gitlab.merge_requests.is_empty() {
            content.push_str(&self.summarize_merge_requests(&gitlab.merge_requests, "GitLab")?);
        }

        // Process issues
        if !gitlab.issues.is_empty() {
            content.push_str(&self.summarize_issues(&gitlab.issues, "GitLab")?);
        }

        // Process comments (extract key decisions/discussions)
        if !gitlab.comments.is_empty() {
            content.push_str(&self.summarize_comments(&gitlab.comments, "GitLab")?);
        }

        Ok(content)
    }

    fn process_github_section(&self, github: &GitHubData) -> Result<String> {
        let mut content = String::from("# GitHub Activity Summary\n\n");

        if !github.commits.is_empty() {
            content.push_str(&self.summarize_commits(&github.commits, "GitHub")?);
        }

        if !github.pull_requests.is_empty() {
            content.push_str(&self.summarize_pull_requests(&github.pull_requests)?);
        }

        Ok(content)
    }

    fn process_git_commits(&self, commits: &[GitCommitData]) -> Result<String> {
        let mut content = String::from("# Local Git Commits Summary\n\n");
        
        // Convert GitCommitData to Value for consistent processing
        let commit_values: Vec<Value> = commits.iter()
            .map(|c| serde_json::to_value(c).unwrap_or_default())
            .collect();
            
        content.push_str(&self.summarize_commits(&commit_values, "Local Git")?);
        Ok(content)
    }

    fn summarize_commits(&self, commits: &[Value], source: &str) -> Result<String> {
        if commits.is_empty() {
            return Ok(format!("## {} Commits\nNo commits in this period.\n\n", source));
        }

        let mut content = format!("## {} Commits ({} total)\n\n", source, commits.len());

        // Group by repository/project
        let mut by_repo: HashMap<String, Vec<&Value>> = HashMap::new();
        for commit in commits {
            let repo = self.extract_repo_name(commit);
            by_repo.entry(repo).or_default().push(commit);
        }

        for (repo, repo_commits) in by_repo {
            content.push_str(&format!("### Repository: {}\n", repo));
            content.push_str(&format!("**Commits:** {}\n\n", repo_commits.len()));

            // Categorize commits by type
            let mut features = Vec::new();
            let mut fixes = Vec::new();
            let mut docs = Vec::new();
            let mut infrastructure = Vec::new();
            let mut other = Vec::new();

            for commit in repo_commits {
                let summary = self.extract_commit_summary(commit);
                let category = self.categorize_commit(&summary);
                
                match category {
                    CommitCategory::Feature => features.push(summary),
                    CommitCategory::Fix => fixes.push(summary),
                    CommitCategory::Documentation => docs.push(summary),
                    CommitCategory::Infrastructure => infrastructure.push(summary),
                    CommitCategory::Other => other.push(summary),
                }
            }

            if !features.is_empty() {
                content.push_str("**New Features:**\n");
                for feature in features {
                    content.push_str(&format!("- {}\n", feature.title));
                }
                content.push('\n');
            }

            if !infrastructure.is_empty() {
                content.push_str("**Infrastructure & Build:**\n");
                for infra in infrastructure {
                    content.push_str(&format!("- {}\n", infra.title));
                }
                content.push('\n');
            }

            if !fixes.is_empty() {
                content.push_str("**Bug Fixes:**\n");
                for fix in fixes {
                    content.push_str(&format!("- {}\n", fix.title));
                }
                content.push('\n');
            }

            if !docs.is_empty() {
                content.push_str("**Documentation:**\n");
                for doc in docs {
                    content.push_str(&format!("- {}\n", doc.title));
                }
                content.push('\n');
            }

            if !other.is_empty() && other.len() <= 5 {
                content.push_str("**Other Changes:**\n");
                for change in other {
                    content.push_str(&format!("- {}\n", change.title));
                }
                content.push('\n');
            }
        }

        Ok(content)
    }

    fn summarize_merge_requests(&self, mrs: &[Value], source: &str) -> Result<String> {
        if mrs.is_empty() {
            return Ok(format!("## {} Merge Requests\nNo merge requests in this period.\n\n", source));
        }

        let mut content = format!("## {} Merge Requests ({} total)\n\n", source, mrs.len());

        for mr in mrs {
            let title = mr.get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("Untitled");
            
            let state = mr.get("state")
                .and_then(|s| s.as_str())
                .unwrap_or("unknown");

            let description = mr.get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .lines()
                .take(3)
                .collect::<Vec<_>>()
                .join(" ");

            content.push_str(&format!("- **{}** [{}]\n", title, state));
            if !description.is_empty() && description.len() > 10 {
                let truncated = if description.len() > 200 {
                    format!("{}...", &description[..200])
                } else {
                    description
                };
                content.push_str(&format!("  {}\n", truncated));
            }
        }

        content.push('\n');
        Ok(content)
    }

    fn summarize_pull_requests(&self, prs: &[Value]) -> Result<String> {
        // Similar to merge requests but for GitHub
        self.summarize_merge_requests(prs, "GitHub Pull")
    }

    fn summarize_issues(&self, issues: &[Value], source: &str) -> Result<String> {
        if issues.is_empty() {
            return Ok(format!("## {} Issues\nNo issues in this period.\n\n", source));
        }

        let mut content = format!("## {} Issues ({} total)\n\n", source, issues.len());

        // Group by status
        let mut open = Vec::new();
        let mut closed = Vec::new();
        
        for issue in issues {
            let state = issue.get("state").and_then(|s| s.as_str()).unwrap_or("unknown");
            let title = issue.get("title").and_then(|t| t.as_str()).unwrap_or("Untitled");
            
            match state {
                "opened" | "open" => open.push(title),
                "closed" => closed.push(title),
                _ => {}
            }
        }

        if !open.is_empty() {
            content.push_str("**Open Issues:**\n");
            for issue in open {
                content.push_str(&format!("- {}\n", issue));
            }
            content.push('\n');
        }

        if !closed.is_empty() {
            content.push_str("**Closed Issues:**\n");
            for issue in closed {
                content.push_str(&format!("- {}\n", issue));
            }
            content.push('\n');
        }

        Ok(content)
    }

    fn summarize_comments(&self, comments: &[Value], source: &str) -> Result<String> {
        if comments.is_empty() {
            return Ok(format!("## {} Key Discussions\nNo significant discussions in this period.\n\n", source));
        }

        let mut content = format!("## {} Key Discussions\n\n", source);

        // Extract only meaningful comments (longer than 50 chars, avoid automated stuff)
        let meaningful_comments: Vec<&Value> = comments.iter()
            .filter(|comment| {
                if let Some(body) = comment.get("body").and_then(|b| b.as_str()) {
                    body.len() > 50 && 
                    !body.contains("automatically") &&
                    !body.contains("bot") &&
                    !body.starts_with("@")
                } else {
                    false
                }
            })
            .take(10) // Limit to most important
            .collect();

        if meaningful_comments.is_empty() {
            content.push_str("No significant technical discussions captured.\n\n");
        } else {
            content.push_str(&format!("**Key Technical Discussions ({} captured):**\n", meaningful_comments.len()));
            for comment in meaningful_comments {
                if let Some(body) = comment.get("body").and_then(|b| b.as_str()) {
                    let truncated = if body.len() > 150 {
                        format!("{}...", &body[..150])
                    } else {
                        body.to_string()
                    };
                    content.push_str(&format!("- {}\n", truncated.replace('\n', " ")));
                }
            }
            content.push('\n');
        }

        Ok(content)
    }

    fn extract_repo_name(&self, commit: &Value) -> String {
        commit.get("_project_name")
            .and_then(|n| n.as_str())
            .or_else(|| commit.get("repository")
                .and_then(|r| r.get("name"))
                .and_then(|n| n.as_str()))
            .unwrap_or("Unknown Repository")
            .to_string()
    }

    fn extract_commit_summary(&self, commit: &Value) -> CommitSummary {
        let title = commit.get("title")
            .or_else(|| commit.get("message"))
            .and_then(|t| t.as_str())
            .unwrap_or("No title")
            .lines()
            .next()
            .unwrap_or("No title")
            .to_string();

        let author = commit.get("author_name")
            .and_then(|a| a.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let id = commit.get("short_id")
            .or_else(|| commit.get("id"))
            .and_then(|i| i.as_str())
            .unwrap_or("")
            .to_string();

        CommitSummary { title, author, id }
    }

    fn categorize_commit(&self, summary: &CommitSummary) -> CommitCategory {
        let title_lower = summary.title.to_lowercase();
        
        if title_lower.contains("fix") || title_lower.contains("bug") || title_lower.contains("error") {
            CommitCategory::Fix
        } else if title_lower.contains("add") || title_lower.contains("implement") || 
                  title_lower.contains("feature") || title_lower.contains("support") {
            CommitCategory::Feature
        } else if title_lower.contains("doc") || title_lower.contains("readme") || 
                  title_lower.contains("comment") {
            CommitCategory::Documentation
        } else if title_lower.contains("makefile") || title_lower.contains("build") || 
                  title_lower.contains("ci") || title_lower.contains("pipeline") ||
                  title_lower.contains("refactor") || title_lower.contains("clean") {
            CommitCategory::Infrastructure
        } else {
            CommitCategory::Other
        }
    }
}

#[derive(Debug)]
struct CommitSummary {
    title: String,
    author: String,
    id: String,
}

#[derive(Debug)]
enum CommitCategory {
    Feature,
    Fix,
    Documentation,
    Infrastructure,
    Other,
}

use crate::types::{CollectedData, GitLabData, GitHubData, GitCommitData};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

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

            for commit in &repo_commits {
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
                    content.push_str(&format!("- `{}` - {} ({} files, +{} -{} lines)\n", 
                        feature.id, feature.title, feature.files_changed, feature.additions, feature.deletions));
                    if !feature.files_list.is_empty() {
                        content.push_str(&format!("  Files: {}\n", feature.files_list));
                    }
                }
                content.push('\n');
            }

            if !infrastructure.is_empty() {
                content.push_str("**Infrastructure & Build:**\n");
                for infra in infrastructure {
                    content.push_str(&format!("- `{}` - {} ({} files, +{} -{} lines)\n", 
                        infra.id, infra.title, infra.files_changed, infra.additions, infra.deletions));
                    if !infra.files_list.is_empty() {
                        content.push_str(&format!("  Files: {}\n", infra.files_list));
                    }
                }
                content.push('\n');
            }

            if !fixes.is_empty() {
                content.push_str("**Bug Fixes:**\n");
                for fix in fixes {
                    content.push_str(&format!("- `{}` - {} ({} files, +{} -{} lines)\n", 
                        fix.id, fix.title, fix.files_changed, fix.additions, fix.deletions));
                    if !fix.files_list.is_empty() {
                        content.push_str(&format!("  Files: {}\n", fix.files_list));
                    }
                }
                content.push('\n');
            }

            if !docs.is_empty() {
                content.push_str("**Documentation:**\n");
                for doc in docs {
                    content.push_str(&format!("- `{}` - {} ({} files, +{} -{} lines)\n", 
                        doc.id, doc.title, doc.files_changed, doc.additions, doc.deletions));
                }
                content.push('\n');
            }

            if !other.is_empty() && other.len() <= 5 {
                content.push_str("**Other Changes:**\n");
                for change in other {
                    content.push_str(&format!("- `{}` - {} ({} files, +{} -{} lines)\n", 
                        change.id, change.title, change.files_changed, change.additions, change.deletions));
                }
                content.push('\n');
            }

            // Add summary metrics for the repository
            let total_commits = repo_commits.len();
            let total_files: usize = repo_commits.iter()
                .map(|c| self.extract_commit_summary(c).files_changed)
                .sum();
            let total_additions: u64 = repo_commits.iter()
                .map(|c| self.extract_commit_summary(c).additions)
                .sum();
            let total_deletions: u64 = repo_commits.iter()
                .map(|c| self.extract_commit_summary(c).deletions)
                .sum();
            
            content.push_str(&format!("**Repository Summary:** {} commits, {} files modified, +{} -{} lines\n\n", 
                total_commits, total_files, total_additions, total_deletions));
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
            .or_else(|| commit.get("hash"))
            .and_then(|i| i.as_str())
            .unwrap_or("")
            .to_string();

        // Extract technical details for architectural analysis
        let full_message = commit.get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .to_string();

        let files_changed = commit.get("files_changed")
            .and_then(|f| f.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);

        let files_list = commit.get("files_changed")
            .and_then(|f| f.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|f| f.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();

        // Extract stats if available
        let additions = commit.get("stats")
            .and_then(|s| s.get("additions"))
            .and_then(|a| a.as_u64())
            .unwrap_or(0);

        let deletions = commit.get("stats")
            .and_then(|s| s.get("deletions"))
            .and_then(|d| d.as_u64())
            .unwrap_or(0);

        CommitSummary { 
            title, 
            author, 
            id, 
            full_message, 
            files_changed, 
            files_list, 
            additions, 
            deletions 
        }
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
    #[allow(dead_code)]
    author: String,
    id: String,
    #[allow(dead_code)]
    full_message: String,
    files_changed: usize,
    files_list: String,
    additions: u64,
    deletions: u64,
}

#[derive(Debug)]
enum CommitCategory {
    Feature,
    Fix,
    Documentation,
    Infrastructure,
    Other,
}

// ---- Sanitized data types (output of the deductive sanitize step) ----

/// A commit normalized from any source (GitLab, GitHub, local git).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizedCommit {
    /// Short hash used for deduplication.
    pub hash: String,
    pub title: String,
    pub author: String,
    pub repo: String,
    /// Full commit message, capped at 600 chars.
    pub full_message: String,
    /// Files touched, capped at 10 entries.
    pub files_changed: Vec<String>,
    pub additions: u64,
    pub deletions: u64,
}

/// All activity after dedup, bot-filter, truncation, and token-budget enforcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizedData {
    pub commits: Vec<SanitizedCommit>,
    /// MR/PR titles with state, e.g. "[merged] Add retry logic"
    pub mrs_and_prs: Vec<String>,
    /// Issue titles with state, e.g. "[closed] Fix crash on empty input"
    pub issues: Vec<String>,
}

impl DataPreprocessor {
    /// Deductive phase step 1 — sanitize raw collected data.
    ///
    /// Normalizes commits from all sources into `SanitizedCommit`, deduplicates
    /// by hash, removes bot authors, truncates long messages/file lists, and
    /// drops the oldest commits when the estimated token count exceeds `budget`.
    pub fn sanitize(&self, data: &CollectedData, budget: usize) -> SanitizedData {
        let mut commits = self.collect_all_commits(data);

        // Dedup by hash; if hash is empty use "title:author" as fallback key.
        let mut seen: HashSet<String> = HashSet::new();
        commits.retain(|c| {
            let key = if c.hash.is_empty() {
                format!("{}:{}", c.title, c.author)
            } else {
                c.hash.clone()
            };
            seen.insert(key)
        });

        // Drop bot-authored commits.
        commits.retain(|c| !Self::is_bot_author(&c.author));

        // Collect MR/PR and issue titles (simple text form for context).
        let mrs_and_prs: Vec<String> = data
            .gitlab
            .merge_requests
            .iter()
            .chain(data.github.pull_requests.iter())
            .filter_map(|v| {
                let title = v.get("title").and_then(|t| t.as_str())?;
                let state = v.get("state").and_then(|s| s.as_str()).unwrap_or("?");
                Some(format!("[{}] {}", state, title))
            })
            .collect();

        let issues: Vec<String> = data
            .gitlab
            .issues
            .iter()
            .chain(data.github.issues.iter())
            .filter_map(|v| {
                let title = v.get("title").and_then(|t| t.as_str())?;
                let state = v.get("state").and_then(|s| s.as_str()).unwrap_or("?");
                Some(format!("[{}] {}", state, title))
            })
            .collect();

        // Token budget: estimate ~4 chars per token.
        // Drop commits from the back (oldest/least-signal) until within budget.
        loop {
            if commits.is_empty() {
                break;
            }
            let commit_chars: usize = commits
                .iter()
                .map(|c| c.title.len() + c.full_message.len() + c.files_changed.join(",").len() + 40)
                .sum();
            let context_chars: usize =
                mrs_and_prs.iter().map(|s| s.len()).sum::<usize>()
                    + issues.iter().map(|s| s.len()).sum::<usize>();
            if (commit_chars + context_chars) / 4 <= budget {
                break;
            }
            commits.pop();
        }

        SanitizedData { commits, mrs_and_prs, issues }
    }

    /// Collect and normalize commits from all three sources into `SanitizedCommit`.
    fn collect_all_commits(&self, data: &CollectedData) -> Vec<SanitizedCommit> {
        let mut result: Vec<SanitizedCommit> = Vec::new();

        // GitLab + GitHub commits share the same JSON shape.
        for commit in data.gitlab.commits.iter().chain(data.github.commits.iter()) {
            let summary = self.extract_commit_summary(commit);
            let repo = self.extract_repo_name(commit);
            let files: Vec<String> = commit
                .get("files_changed")
                .and_then(|f| f.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|f| f.as_str().map(|s| s.to_string()))
                        .take(10)
                        .collect()
                })
                .unwrap_or_default();

            result.push(SanitizedCommit {
                hash: summary.id,
                title: summary.title,
                author: summary.author,
                repo,
                full_message: summary.full_message.chars().take(600).collect(),
                files_changed: files,
                additions: summary.additions,
                deletions: summary.deletions,
            });
        }

        // Local git commits have a typed struct.
        for c in &data.git_commits {
            let hash = c.hash.chars().take(8).collect();
            let title = c.message.lines().next().unwrap_or("").to_string();
            let full_message: String = c.message.chars().take(600).collect();
            let repo = c
                .repo_path
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or(&c.repo_path)
                .to_string();

            result.push(SanitizedCommit {
                hash,
                title,
                author: c.author_name.clone(),
                repo,
                full_message,
                files_changed: c.files_changed.iter().take(10).cloned().collect(),
                additions: 0,
                deletions: 0,
            });
        }

        result
    }

    fn is_bot_author(author: &str) -> bool {
        let lower = author.to_lowercase();
        lower.contains("[bot]")
            || lower.contains("dependabot")
            || lower.contains("renovate")
            || lower.contains("github-actions")
            || lower.contains("gitlab-bot")
    }
}

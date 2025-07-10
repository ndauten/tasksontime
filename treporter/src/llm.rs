use crate::config::LlmConfig;
use crate::types::CollectedData;
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::env;

pub struct LlmClient {
    client: Client,
    config: LlmConfig,
    api_key: Option<String>,
    fallback_mode: bool,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Result<Self> {
        let (api_key, fallback_mode) = match env::var(&config.api_key_env) {
            Ok(key) if !key.is_empty() => (Some(key), false),
            _ => {
                eprintln!("⚠️  API key not found in environment variable: {}", config.api_key_env);
                eprintln!("🔄 Running in fallback mode - will generate template-based reports");
                (None, true)
            }
        };

        Ok(Self {
            client: Client::new(),
            config,
            api_key,
            fallback_mode,
        })
    }

    pub async fn generate_report(&self, data: &CollectedData, template: &str, report_type: &str) -> Result<String> {
        if self.fallback_mode {
            return self.generate_fallback_report(data, template, report_type);
        }

        let system_prompt = self.create_system_prompt(report_type);
        let user_prompt = self.create_user_prompt(data, template);

        match self.config.provider.as_str() {
            "openai" => self.call_openai(&system_prompt, &user_prompt).await,
            "anthropic" => self.call_anthropic(&system_prompt, &user_prompt).await,
            _ => Err(anyhow!("Unsupported LLM provider: {}", self.config.provider)),
        }
    }

    fn generate_fallback_report(&self, data: &CollectedData, template: &str, _report_type: &str) -> Result<String> {
        let mut report = template.to_string();
        
        // Replace template variables with actual data
        report = report.replace("{{project_name}}", "SPEAR Project");
        report = report.replace("{{report_period}}", &format!("{} to {}", 
            data.metadata.date_range_start.format("%B %Y"),
            data.metadata.date_range_end.format("%B %Y")));
        
        // Generate comprehensive quantitative summaries
        let total_items = data.gitlab_events.len() + data.github_events.len() + 
                         data.local_files.len() + data.git_commits.len();
        
        // Count different types of GitLab activities
        let mut gitlab_commits = 0;
        let mut gitlab_issues = 0;
        let mut gitlab_mrs = 0;
        let mut gitlab_other = 0;
        
        for event in &data.gitlab_events {
            match event.action_name.as_str() {
                "committed" if event.target_type.as_deref() == Some("Commit") => gitlab_commits += 1,
                "opened" | "closed" if event.target_type.as_deref() == Some("Issue") => gitlab_issues += 1,
                "opened" | "closed" | "merged" if event.target_type.as_deref() == Some("MergeRequest") => gitlab_mrs += 1,
                _ => gitlab_other += 1,
            }
        }
        
        // Calculate git commit statistics
        let total_insertions: usize = data.git_commits.iter().map(|c| c.insertions).sum();
        let total_deletions: usize = data.git_commits.iter().map(|c| c.deletions).sum();
        let total_files_changed: usize = data.git_commits.iter().map(|c| c.files_changed.len()).sum();
        
        // Count unique repositories
        let mut unique_repos = std::collections::HashSet::new();
        for commit in &data.git_commits {
            unique_repos.insert(&commit.repository);
        }
        
        let activity_summary = format!(
            "**Quantitative Summary for Reporting Period:**\n\n\
            **Total Activities Tracked: {}**\n\
            - 📝 {} local file updates\n\
            - 🔗 {} local git commits across {} repositories\n\
            - 🦊 {} GitLab commits (remote repositories)\n\
            - 🦊 {} other GitLab events ({}I/{}MR/{}O)\n\
            - 🐙 {} GitHub events\n\n\
            **Development Metrics:**\n\
            - **Total Commits:** {} (local + remote)\n\
            - **Lines of Code (local):** +{} insertions, -{} deletions\n\
            - **Files Modified (local):** {} files\n\
            - **Active Local Repositories:** {}\n\
            - **Daily Average:** {:.1} activities per day",
            total_items,
            data.local_files.len(),
            data.git_commits.len(),
            unique_repos.len(),
            gitlab_commits,
            data.gitlab_events.len() - gitlab_commits,
            gitlab_issues, gitlab_mrs, gitlab_other,
            data.github_events.len(),
            data.git_commits.len() + gitlab_commits,
            total_insertions,
            total_deletions,
            total_files_changed,
            unique_repos.len(),
            total_items as f64 / ((data.metadata.date_range_end - data.metadata.date_range_start).num_days() as f64 + 1.0)
        );
        
        report = report.replace("{{activity_summary}}", &activity_summary);
        
        // Add recent commits with enhanced details (combining local git commits and GitLab commits)
        let mut all_commits = Vec::new();
        
        // Add local git commits
        for commit in &data.git_commits {
            all_commits.push(format!("- **{}** [{}]: {} (+{} -{} lines in {} files) [Local Git]", 
                commit.date.format("%Y-%m-%d"),
                commit.author,
                commit.message.lines().next().unwrap_or("No message").trim(),
                commit.insertions,
                commit.deletions,
                commit.files_changed.len()));
        }
        
        // Add GitLab commits
        for event in &data.gitlab_events {
            if event.action_name == "committed" && event.target_type.as_deref() == Some("Commit") {
                let commit_hash = event.commit_hash.as_deref()
                    .map(|h| &h[..8.min(h.len())]) // Take first 8 chars
                    .unwrap_or("Unknown");
                let project_name = event.project_name.as_deref().unwrap_or("Unknown");
                
                all_commits.push(format!("- **{}** [{}] ({}): {} [GitLab:{}]", 
                    event.created_at.format("%Y-%m-%d"),
                    event.author_name.as_deref().unwrap_or("Unknown"),
                    commit_hash,
                    event.target_title.as_deref().unwrap_or("No title"),
                    project_name));
            }
        }
        
        // Sort commits by date (newest first)
        all_commits.sort_by(|a, b| {
            let date_a = &a[3..13]; // Extract date from "- **YYYY-MM-DD**"
            let date_b = &b[3..13];
            date_b.cmp(date_a) // Reverse order for newest first
        });
        
        if !all_commits.is_empty() {
            let recent_commits = all_commits.into_iter()
                .take(10) // Show more commits since we're combining sources
                .collect::<Vec<_>>()
                .join("\n");
            report = report.replace("{{recent_commits}}", &recent_commits);
        } else {
            report = report.replace("{{recent_commits}}", "No commits found in the specified period");
        }
        
        // Add file updates summary with enhanced details
        if !data.local_files.is_empty() {
            let mut file_updates = String::new();
            for file in data.local_files.iter().take(10) {
                file_updates.push_str(&format!("- **{}** ({} entries)\n", 
                    file.file_name, 
                    file.time_based_entries.len() + file.content_snippets.len()));
            }
            report = report.replace("{{file_updates}}", &file_updates);
        } else {
            report = report.replace("{{file_updates}}", "No file updates found in the specified period");
        }
        
        // Add quantitative placeholder replacements
        report = report.replace("{{gitlab_events_count}}", &data.gitlab_events.len().to_string());
        report = report.replace("{{github_events_count}}", &data.github_events.len().to_string());
        report = report.replace("{{git_commits_count}}", &data.git_commits.len().to_string());
        report = report.replace("{{local_files_count}}", &data.local_files.len().to_string());
        report = report.replace("{{total_insertions}}", &total_insertions.to_string());
        report = report.replace("{{total_deletions}}", &total_deletions.to_string());
        report = report.replace("{{files_changed_count}}", &total_files_changed.to_string());
        report = report.replace("{{unique_repos_count}}", &unique_repos.len().to_string());
        report = report.replace("{{generation_date}}", &chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string());
        
        Ok(report)
    }

    fn create_system_prompt(&self, report_type: &str) -> String {
        match report_type {
            "monthly_report" => {
                "You are an expert technical project manager tasked with creating a comprehensive monthly project report. 
                 Based on the provided data from various sources (GitLab, GitHub, local files, git commits), 
                 create a well-structured, professional report that highlights accomplishments, progress, 
                 challenges, and next steps. Focus on meaningful insights rather than just listing activities.
                 
                 **CRITICAL: Include explicit quantitative metrics and analysis throughout the report:**
                 - Count and summarize different types of activities (commits, issues, MRs, file updates)
                 - Calculate development velocity metrics (lines of code, files changed, daily averages)
                 - Identify trends and patterns in the data
                 - Provide numerical context for all major accomplishments
                 - Include a dedicated 'Progress Metrics' section with specific numbers
                 
                 The report should be suitable for stakeholders and project sponsors. Use clear, concise language
                 and organize information logically. Always lead with quantitative summaries before diving into
                 qualitative analysis. Make the data actionable by highlighting what the numbers mean for project progress.".to_string()
            },
            "group_slides" => {
                "You are creating slides for a group meeting based on project activity data.
                 Create content suitable for presentation slides that highlights key accomplishments,
                 progress made, current blockers, and next steps. Be concise and focus on the most
                 important information that would be relevant to a team meeting.
                 
                 **CRITICAL: Lead with quantitative metrics on every slide:**
                 - Start with a 'By the Numbers' slide showing key metrics
                 - Include specific counts for commits, issues, MRs, files changed
                 - Show development velocity and activity trends
                 - Use numbers to support every major point
                 - Include progress indicators and completion rates where applicable
                 
                 Format the output as Marp markdown with appropriate slide breaks and formatting.
                 Use bullet points and clear headings. Keep text minimal and impactful but always
                 include the supporting numbers.".to_string()
            },
            _ => "Create a summary report based on the provided project activity data. Focus on quantitative metrics and measurable progress indicators.".to_string(),
        }
    }

    fn create_user_prompt(&self, data: &CollectedData, template: &str) -> String {
        let mut prompt = format!("Please create a report based on the following template and data:\n\n");
        
        prompt.push_str("TEMPLATE:\n");
        prompt.push_str(template);
        prompt.push_str("\n\n");
        
        prompt.push_str("PROJECT ACTIVITY DATA:\n\n");
        
        // Add metadata with quantitative summary
        prompt.push_str(&format!(
            "**REPORTING PERIOD:** {} to {} ({} days)\n",
            data.metadata.date_range_start.format("%Y-%m-%d"),
            data.metadata.date_range_end.format("%Y-%m-%d"),
            (data.metadata.date_range_end - data.metadata.date_range_start).num_days() + 1
        ));
        prompt.push_str(&format!("**DATA SOURCES:** {}\n\n", data.metadata.sources_used.join(", ")));
        
        // Add quantitative overview
        let total_items = data.gitlab_events.len() + data.github_events.len() + 
                         data.local_files.len() + data.git_commits.len();
        
        // Calculate git commit statistics
        let total_insertions: usize = data.git_commits.iter().map(|c| c.insertions).sum();
        let total_deletions: usize = data.git_commits.iter().map(|c| c.deletions).sum();
        let total_files_changed: usize = data.git_commits.iter().map(|c| c.files_changed.len()).sum();
        
        // Count unique repositories
        let mut unique_repos = std::collections::HashSet::new();
        for commit in &data.git_commits {
            unique_repos.insert(&commit.repository);
        }
        
        // Count different types of GitLab activities
        let mut gitlab_commits = 0;
        let mut gitlab_issues = 0;
        let mut gitlab_mrs = 0;
        let mut gitlab_other = 0;
        
        for event in &data.gitlab_events {
            match event.action_name.as_str() {
                "committed" if event.target_type.as_deref() == Some("Commit") => gitlab_commits += 1,
                "opened" | "closed" if event.target_type.as_deref() == Some("Issue") => gitlab_issues += 1,
                "opened" | "closed" | "merged" if event.target_type.as_deref() == Some("MergeRequest") => gitlab_mrs += 1,
                _ => gitlab_other += 1,
            }
        }
        
        // Count GitLab commits separately for better statistics
        let gitlab_commit_count = data.gitlab_events.iter()
            .filter(|e| e.action_name == "committed" && e.target_type.as_deref() == Some("Commit"))
            .count();
        
        prompt.push_str(&format!(
            "**QUANTITATIVE SUMMARY:**\n\
            - Total Activities: {}\n\
            - Local Git Commits: {} (across {} repos)\n\
            - GitLab Commits: {} (remote repositories)\n\
            - Total Commits: {} (local + remote)\n\
            - Other GitLab Events: {} ({}I/{}MR/{}O)\n\
            - GitHub Events: {}\n\
            - Local Files: {}\n\
            - Lines Changed (local): +{} -{}\n\
            - Files Modified (local): {}\n\
            - Daily Average: {:.1} activities/day\n\n",
            total_items,
            data.git_commits.len(),
            unique_repos.len(),
            gitlab_commit_count,
            data.git_commits.len() + gitlab_commit_count,
            data.gitlab_events.len() - gitlab_commit_count,
            gitlab_issues, gitlab_mrs, gitlab_other,
            data.github_events.len(),
            data.local_files.len(),
            total_insertions,
            total_deletions,
            total_files_changed,
            total_items as f64 / ((data.metadata.date_range_end - data.metadata.date_range_start).num_days() as f64 + 1.0)
        ));
        
        // Add GitLab events summary with special emphasis on commits
        if !data.gitlab_events.is_empty() {
            let gitlab_commits: Vec<_> = data.gitlab_events.iter()
                .filter(|e| e.action_name == "committed" && e.target_type.as_deref() == Some("Commit"))
                .collect();
            
            if !gitlab_commits.is_empty() {
                prompt.push_str("**GITLAB COMMITS:**\n");
                for event in gitlab_commits {
                    let commit_hash = event.commit_hash.as_deref()
                        .map(|h| &h[..8.min(h.len())]) // Take first 8 chars
                        .unwrap_or("Unknown");
                    let project_name = event.project_name.as_deref().unwrap_or("Unknown");
                    
                    prompt.push_str(&format!(
                        "- {} [{}] ({}): {} [GitLab:{}]\n",
                        event.created_at.format("%Y-%m-%d"),
                        event.author_name.as_deref().unwrap_or("Unknown"),
                        commit_hash,
                        event.target_title.as_deref().unwrap_or("No title"),
                        project_name
                    ));
                }
                prompt.push_str("\n");
            }
            
            let other_events: Vec<_> = data.gitlab_events.iter()
                .filter(|e| {
                    // Only include opened/closed issues and merge requests, exclude comments and other noise
                    match e.action_name.as_str() {
                        "opened" | "closed" | "merged" => {
                            matches!(e.target_type.as_deref(), Some("Issue") | Some("MergeRequest"))
                        },
                        _ => false
                    }
                })
                .collect();
            
            if !other_events.is_empty() {
                prompt.push_str("**OTHER GITLAB ACTIVITY:**\n");
                for event in other_events {
                    prompt.push_str(&format!(
                        "- {} ({}): {} [{}]\n",
                        event.created_at.format("%Y-%m-%d"),
                        event.action_name,
                        event.details.as_deref().unwrap_or("No details"),
                        event.project_name.as_deref().unwrap_or("Unknown project")
                    ));
                }
                prompt.push_str("\n");
            }
        }
        
        // Add GitHub events summary
        if !data.github_events.is_empty() {
            prompt.push_str("**GITHUB ACTIVITY DETAILS:**\n");
            for event in &data.github_events {
                prompt.push_str(&format!(
                    "- {} ({}): {} [{}]\n",
                    event.created_at.format("%Y-%m-%d"),
                    event.event_type,
                    event.details.as_deref().unwrap_or("No details"),
                    event.repo_name.as_deref().unwrap_or("Unknown repo")
                ));
            }
            prompt.push_str("\n");
        }
        
        // Add Git commits summary
        if !data.git_commits.is_empty() {
            prompt.push_str("**GIT COMMITS DETAILS:**\n");
            for commit in &data.git_commits {
                prompt.push_str(&format!(
                    "- {} [{}]: {} (+{} -{} changes in {} files)\n",
                    commit.date.format("%Y-%m-%d"),
                    commit.repository,
                    commit.message.lines().next().unwrap_or("No message"),
                    commit.insertions,
                    commit.deletions,
                    commit.files_changed.len()
                ));
            }
            prompt.push_str("\n");
        }
        
        // Add local files summary
        if !data.local_files.is_empty() {
            prompt.push_str("**LOCAL FILES & NOTES DETAILS:**\n");
            for file in &data.local_files {
                prompt.push_str(&format!("File: {} ({} time-based entries, {} content snippets)\n", 
                    file.file_name, 
                    file.time_based_entries.len(), 
                    file.content_snippets.len()));
                
                for entry in &file.time_based_entries {
                    prompt.push_str(&format!(
                        "  - {} ({}): {}\n",
                        entry.date.format("%Y-%m-%d"),
                        entry.entry_type,
                        entry.content.lines().next().unwrap_or("").trim()
                    ));
                }
                
                if !file.content_snippets.is_empty() && file.time_based_entries.is_empty() {
                    for snippet in file.content_snippets.iter().take(3) {
                        prompt.push_str(&format!("  - {}\n", snippet.content.trim()));
                    }
                }
                prompt.push_str("\n");
            }
        }
        
        prompt.push_str("\nPlease create a comprehensive report based on this data and the provided template.");
        prompt
    }

    async fn call_openai(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let payload = json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user", 
                    "content": user_prompt
                }
            ],
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature
        });

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key.as_ref().ok_or_else(|| anyhow!("API key not available"))?))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let response_json: Value = response.json().await?;
        
        if let Some(error) = response_json.get("error") {
            return Err(anyhow!("OpenAI API error: {}", error));
        }

        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid response format from OpenAI"))?;

        Ok(content.to_string())
    }

    async fn call_anthropic(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let payload = json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
            "system": system_prompt,
            "messages": [
                {
                    "role": "user",
                    "content": user_prompt
                }
            ]
        });

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("Authorization", format!("Bearer {}", self.api_key.as_ref().ok_or_else(|| anyhow!("API key not available"))?))
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&payload)
            .send()
            .await?;

        let response_json: Value = response.json().await?;
        
        if let Some(error) = response_json.get("error") {
            return Err(anyhow!("Anthropic API error: {}", error));
        }

        let content = response_json["content"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid response format from Anthropic"))?;

        Ok(content.to_string())
    }
}

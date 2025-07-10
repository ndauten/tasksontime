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
            Ok(key) => (Some(key), false),
            Err(_) => {
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
        report = report.replace("{{report_period}}", "January 2025");
        
        // Generate basic summaries
        let total_items = data.gitlab_events.len() + data.github_events.len() + 
                         data.local_files.len() + data.git_commits.len();
        
        let activity_summary = format!(
            "During this period, we tracked {} total activities:\n\
            - {} local file updates\n\
            - {} git commits\n\
            - {} GitLab events\n\
            - {} GitHub events",
            total_items,
            data.local_files.len(),
            data.git_commits.len(),
            data.gitlab_events.len(),
            data.github_events.len()
        );
        
        report = report.replace("{{activity_summary}}", &activity_summary);
        
        // Add recent commits if available
        if !data.git_commits.is_empty() {
            let recent_commits = data.git_commits.iter()
                .take(5)
                .map(|c| format!("- {} ({})", c.message.trim(), c.author))
                .collect::<Vec<_>>()
                .join("\n");
            report = report.replace("{{recent_commits}}", &recent_commits);
        } else {
            report = report.replace("{{recent_commits}}", "No commits found in the specified period");
        }
        
        // Add file updates summary
        if !data.local_files.is_empty() {
            let file_updates = data.local_files.iter()
                .take(10)
                .map(|f| format!("- {}", f.file_name))
                .collect::<Vec<_>>()
                .join("\n");
            report = report.replace("{{file_updates}}", &file_updates);
        } else {
            report = report.replace("{{file_updates}}", "No file updates found in the specified period");
        }
        
        Ok(report)
    }

    fn create_system_prompt(&self, report_type: &str) -> String {
        match report_type {
            "monthly_report" => {
                "You are an expert technical project manager tasked with creating a comprehensive monthly project report. 
                 Based on the provided data from various sources (GitLab, GitHub, local files, git commits), 
                 create a well-structured, professional report that highlights accomplishments, progress, 
                 challenges, and next steps. Focus on meaningful insights rather than just listing activities.
                 
                 The report should be suitable for stakeholders and project sponsors. Use clear, concise language
                 and organize information logically. Include metrics where relevant and provide context for
                 technical activities.".to_string()
            },
            "group_slides" => {
                "You are creating slides for a group meeting based on project activity data.
                 Create content suitable for presentation slides that highlights key accomplishments,
                 progress made, current blockers, and next steps. Be concise and focus on the most
                 important information that would be relevant to a team meeting.
                 
                 Format the output as Marp markdown with appropriate slide breaks and formatting.
                 Use bullet points and clear headings. Keep text minimal and impactful.".to_string()
            },
            _ => "Create a summary report based on the provided project activity data.".to_string(),
        }
    }

    fn create_user_prompt(&self, data: &CollectedData, template: &str) -> String {
        let mut prompt = format!("Please create a report based on the following template and data:\n\n");
        
        prompt.push_str("TEMPLATE:\n");
        prompt.push_str(template);
        prompt.push_str("\n\n");
        
        prompt.push_str("PROJECT ACTIVITY DATA:\n\n");
        
        // Add metadata
        prompt.push_str(&format!(
            "Report Period: {} to {}\n",
            data.metadata.date_range_start.format("%Y-%m-%d"),
            data.metadata.date_range_end.format("%Y-%m-%d")
        ));
        prompt.push_str(&format!("Data Sources: {}\n\n", data.metadata.sources_used.join(", ")));
        
        // Add GitLab events summary
        if !data.gitlab_events.is_empty() {
            prompt.push_str("GITLAB ACTIVITY:\n");
            for event in &data.gitlab_events {
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
        
        // Add GitHub events summary
        if !data.github_events.is_empty() {
            prompt.push_str("GITHUB ACTIVITY:\n");
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
            prompt.push_str("GIT COMMITS:\n");
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
            prompt.push_str("LOCAL FILES & NOTES:\n");
            for file in &data.local_files {
                prompt.push_str(&format!("File: {}\n", file.file_name));
                
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

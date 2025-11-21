use crate::config::Config;
use crate::types::CollectedData;
use anyhow::Result;
use reqwest::Client;
use serde_json::json;

pub struct LLMClient {
    client: Client,
    config: Config,
}

impl LLMClient {
    pub fn new(config: Config) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    pub async fn generate_monthly_report(&self, data: &CollectedData) -> Result<String> {
        // Read the template
        let template = std::fs::read_to_string("templates/monthly_report.md")
            .unwrap_or_else(|_| self.default_monthly_template());

        // Prepare data for LLM
        let data_summary = self.prepare_data_summary(data);
        
        let prompt = format!(
            "You are a project manager creating a monthly technical report for the SPEAR project. \
            Based on the following data, generate a comprehensive report using the template provided. \
            Include quantitative metrics, progress summaries, and detailed commit information.\n\n\
            IMPORTANT NOTES:\n\
            - The data may contain duplicate commits from different sources (local git, GitLab, GitHub)\n\
            - When analyzing commits, use the commit hash/SHA to identify duplicates and count each unique commit only once\n\
            - Focus on unique contributions and avoid double-counting activities\n\
            - Provide accurate metrics by deduplicating based on commit hashes, issue IDs, or other unique identifiers\n\n\
            DATA SUMMARY:\n{}\n\n\
            TEMPLATE:\n{}\n\n\
            Generate a complete report with all placeholders filled in, including specific metrics and progress indicators. \
            Ensure all counts and metrics reflect unique activities (no duplicates).",
            data_summary, template
        );

        // Call LLM API
        match self.call_llm_api(&prompt).await {
            Ok(response) => Ok(response),
            Err(e) => {
                println!("⚠️  LLM API failed: {}", e);
                println!("🔄 Falling back to template-based report generation...");
                let fallback_report = self.generate_fallback_report(data, &template);
                println!("📝 Generated fallback report with {} characters", fallback_report.len());
                Ok(fallback_report)
            }
        }
    }

    pub async fn generate_group_slides(&self, data: &CollectedData) -> Result<String> {
        // Read the template
        let template = std::fs::read_to_string("templates/group_slides.md")
            .unwrap_or_else(|_| self.default_slides_template());

        // Prepare data for LLM
        let data_summary = self.prepare_data_summary(data);
        
        let prompt = format!(
            "You are preparing slides for a group presentation on the SPEAR project. \
            Based on the following data, generate presentation slides using the template provided. \
            Focus on progress metrics, key achievements, and quantitative summaries.\n\n\
            IMPORTANT NOTES:\n\
            - The data may contain duplicate commits from different sources (local git, GitLab, GitHub)\n\
            - When analyzing commits, use the commit hash/SHA to identify duplicates and count each unique commit only once\n\
            - Focus on unique contributions and avoid double-counting activities\n\
            - Provide accurate metrics by deduplicating based on commit hashes, issue IDs, or other unique identifiers\n\n\
            DATA SUMMARY:\n{}\n\n\
            TEMPLATE:\n{}\n\n\
            Generate complete slides with all placeholders filled in, emphasizing visual and quantitative metrics. \
            Ensure all counts and metrics reflect unique activities (no duplicates).",
            data_summary, template
        );

        // Call LLM API
        match self.call_llm_api(&prompt).await {
            Ok(response) => Ok(response),
            Err(e) => {
                println!("⚠️  LLM API failed: {}", e);
                println!("🔄 Falling back to template-based slides generation...");
                Ok(self.generate_fallback_slides(data, &template))
            }
        }
    }

    fn prepare_data_summary(&self, data: &CollectedData) -> String {
        let mut summary = String::new();
        
        // Basic metadata
        summary.push_str(&format!(
            "## Collection Metadata\n\
            - Collection period: {} to {}\n\
            - Data sources: {}\n\n",
            data.metadata.date_range_start.format("%Y-%m-%d"),
            data.metadata.date_range_end.format("%Y-%m-%d"),
            data.metadata.sources_used.join(", ")
        ));

        // GitLab raw data - pass the actual JSON
        if !data.gitlab_raw.is_empty() {
            summary.push_str("## GitLab Raw Data\n");
            summary.push_str(&format!("Total items: {}\n\n", data.gitlab_raw.len()));
            for (i, item) in data.gitlab_raw.iter().enumerate() {
                summary.push_str(&format!("GitLab Item {}:\n", i + 1));
                summary.push_str(&serde_json::to_string_pretty(item).unwrap_or_else(|_| "Invalid JSON".to_string()));
                summary.push_str("\n\n");
            }
        }

        // GitHub raw data - pass the actual JSON
        if !data.github_raw.is_empty() {
            summary.push_str("## GitHub Raw Data\n");
            summary.push_str(&format!("Total items: {}\n\n", data.github_raw.len()));
            for (i, item) in data.github_raw.iter().enumerate() {
                summary.push_str(&format!("GitHub Item {}:\n", i + 1));
                summary.push_str(&serde_json::to_string_pretty(item).unwrap_or_else(|_| "Invalid JSON".to_string()));
                summary.push_str("\n\n");
            }
        }

        // Git commit details
        if !data.git_commits.is_empty() {
            summary.push_str("## Git Commits\n");
            for commit in &data.git_commits {
                summary.push_str(&format!(
                    "- {} ({}): {} by {} on {}\n",
                    &commit.hash[0..8],
                    commit.repo_path,
                    commit.message.lines().next().unwrap_or("No message"),
                    commit.author_name,
                    commit.timestamp.format("%Y-%m-%d")
                ));
            }
            summary.push('\n');
        }

        // Local files summary
        if !data.local_files.is_empty() {
            summary.push_str("## Local Files\n");
            summary.push_str(&format!("Total files: {}\n", data.local_files.len()));
            for file in &data.local_files {
                summary.push_str(&format!(
                    "- {} (modified: {})\n",
                    file.file_name,
                    file.last_modified.format("%Y-%m-%d")
                ));
            }
            summary.push('\n');
        }

        summary
    }

    fn generate_fallback_report(&self, data: &CollectedData, template: &str) -> String {
        let mut report = template.to_string();
        
        // Parse GitLab raw data to extract detailed metrics
        let mut gitlab_commits = 0;
        let mut gitlab_issues = 0;
        let mut gitlab_merge_requests = 0;
        let mut gitlab_commit_details = Vec::new();
        
        for item in &data.gitlab_raw {
            if let Some(action) = item.get("action_name").and_then(|v| v.as_str()) {
                match action {
                    "pushed" => {
                        gitlab_commits += 1;
                        if let (Some(author), Some(message), Some(date)) = (
                            item.get("author_name").and_then(|v| v.as_str()),
                            item.get("push_data").and_then(|p| p.get("commit_title")).and_then(|v| v.as_str()),
                            item.get("created_at").and_then(|v| v.as_str()),
                        ) {
                            gitlab_commit_details.push(format!("- {} by {} on {}", message, author, date));
                        }
                    }
                    "opened" | "closed" | "reopened" => {
                        if item.get("target_type").and_then(|v| v.as_str()) == Some("Issue") {
                            gitlab_issues += 1;
                        } else if item.get("target_type").and_then(|v| v.as_str()) == Some("MergeRequest") {
                            gitlab_merge_requests += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
        
        // Basic replacements
        report = report.replace("{{project_name}}", "SPEAR Project");
        report = report.replace("{{report_period}}", &format!("{} to {}", 
            data.metadata.date_range_start.format("%B %Y"),
            data.metadata.date_range_end.format("%B %Y")));
        
        // Quantitative summary with actual GitLab data
        let total_items = data.gitlab_raw.len() + data.github_raw.len() + 
                         data.local_files.len() + data.git_commits.len();
        
        let activity_summary = format!(
            "**Progress Metrics for Reporting Period:**\n\n\
            **Total Activities Tracked: {}**\n\
            - 📝 {} local file updates\n\
            - 🔗 {} local git commits\n\
            - 🦊 {} GitLab activities ({} commits, {} issues, {} MRs)\n\
            - 🐙 {} GitHub activities\n\n\
            **Development Summary:**\n\
            - **Total GitLab Commits:** {}\n\
            - **Total Local Commits:** {}\n\
            - **Issues Activity:** {}\n\
            - **Merge Requests:** {}\n\
            - **Active Data Sources:** {}\n\
            - **Collection Period:** {} to {}\n\n",
            total_items,
            data.local_files.len(),
            data.git_commits.len(),
            data.gitlab_raw.len(),
            gitlab_commits,
            gitlab_issues,
            gitlab_merge_requests,
            data.github_raw.len(),
            gitlab_commits,
            data.git_commits.len(),
            gitlab_issues,
            gitlab_merge_requests,
            data.metadata.sources_used.len(),
            data.metadata.date_range_start.format("%Y-%m-%d"),
            data.metadata.date_range_end.format("%Y-%m-%d")
        );

        report = report.replace("{{activity_summary}}", &activity_summary);
        
        // Detailed commit information
        let mut commit_details = String::new();
        
        // Add GitLab commits
        if !gitlab_commit_details.is_empty() {
            commit_details.push_str("## GitLab Commits\n\n");
            for detail in &gitlab_commit_details {
                commit_details.push_str(&format!("{}\n", detail));
            }
            commit_details.push('\n');
        }
        
        // Add local Git commits
        if !data.git_commits.is_empty() {
            commit_details.push_str("## Local Git Commits\n\n");
            for commit in &data.git_commits {
                commit_details.push_str(&format!(
                    "**{}** ({})\n\
                    - Author: {}\n\
                    - Date: {}\n\
                    - Repository: {}\n\
                    - Files changed: {}\n\
                    - Message: {}\n\n",
                    &commit.hash[0..8],
                    commit.hash,
                    commit.author_name,
                    commit.timestamp.format("%Y-%m-%d"),
                    commit.repo_path,
                    commit.files_changed.len(),
                    commit.message.lines().next().unwrap_or("No message")
                ));
            }
        }

        report = report.replace("{{commit_details}}", &commit_details);
        
        // Replace remaining placeholders
        report = report.replace("{{gitlab_events_count}}", &data.gitlab_raw.len().to_string());
        report = report.replace("{{github_events_count}}", &data.github_raw.len().to_string());
        report = report.replace("{{git_commits_count}}", &data.git_commits.len().to_string());
        report = report.replace("{{local_files_count}}", &data.local_files.len().to_string());
        report = report.replace("{{total_items}}", &total_items.to_string());
        report = report.replace("{{total_insertions}}", "0"); // Raw data doesn't have this
        report = report.replace("{{total_deletions}}", "0"); // Raw data doesn't have this
        report = report.replace("{{files_changed_count}}", &data.local_files.len().to_string());
        report = report.replace("{{unique_repos_count}}", &data.metadata.sources_used.len().to_string());
        
        report
    }

    fn generate_fallback_slides(&self, data: &CollectedData, template: &str) -> String {
        let mut slides = template.to_string();
        
        // Basic replacements
        slides = slides.replace("{{project_name}}", "SPEAR Project");
        slides = slides.replace("{{report_period}}", &format!("{} to {}", 
            data.metadata.date_range_start.format("%B %Y"),
            data.metadata.date_range_end.format("%B %Y")));
        
        // Quantitative summary for slides
        let total_items = data.gitlab_raw.len() + data.github_raw.len() + 
                         data.local_files.len() + data.git_commits.len();
        
        let metrics_summary = format!(
            "## Progress Metrics\n\n\
            - **Total Activities:** {}\n\
            - **Git Commits:** {}\n\
            - **GitLab Activities:** {}\n\
            - **GitHub Activities:** {}\n\
            - **Local Files:** {}\n\
            - **Data Sources:** {}\n\n",
            total_items,
            data.git_commits.len(),
            data.gitlab_raw.len(),
            data.github_raw.len(),
            data.local_files.len(),
            data.metadata.sources_used.len()
        );

        slides = slides.replace("{{metrics_summary}}", &metrics_summary);
        
        // Replace any remaining placeholders
        slides = slides.replace("{{gitlab_events_count}}", &data.gitlab_raw.len().to_string());
        slides = slides.replace("{{github_events_count}}", &data.github_raw.len().to_string());
        slides = slides.replace("{{git_commits_count}}", &data.git_commits.len().to_string());
        slides = slides.replace("{{local_files_count}}", &data.local_files.len().to_string());
        slides = slides.replace("{{total_items}}", &total_items.to_string());
        
        slides
    }

    async fn call_llm_api(&self, prompt: &str) -> Result<String> {
        println!("🔍 Attempting to call LLM API...");
        
        // Check API key
        let api_key = match std::env::var(&self.config.llm.api_key_env) {
            Ok(key) => {
                if key.is_empty() {
                    return Err(anyhow::anyhow!(
                        "❌ API key environment variable '{}' is set but empty. Please set a valid API key.",
                        self.config.llm.api_key_env
                    ));
                }
                println!("✅ API key found: {}...", &key[0..std::cmp::min(10, key.len())]);
                key
            },
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "❌ API key not found. Please set the '{}' environment variable with your {} API key.",
                    self.config.llm.api_key_env,
                    self.config.llm.provider.to_uppercase()
                ));
            }
        };
        
        let payload = match self.config.llm.provider.as_str() {
            "openai" => {
                json!({
                    "model": self.config.llm.model,
                    "messages": [
                        {"role": "system", "content": "You are a helpful assistant that generates technical reports."},
                        {"role": "user", "content": prompt}
                    ],
                    "max_tokens": 4000,
                    "temperature": 0.7
                })
            },
            "anthropic" => {
                json!({
                    "model": self.config.llm.model,
                    "max_tokens": 4000,
                    "messages": [
                        {"role": "user", "content": prompt}
                    ]
                })
            },
            _ => return Err(anyhow::anyhow!("❌ Unsupported LLM provider: '{}'. Supported providers: openai, anthropic", self.config.llm.provider)),
        };

        let url = match self.config.llm.provider.as_str() {
            "openai" => "https://api.openai.com/v1/chat/completions",
            "anthropic" => "https://api.anthropic.com/v1/messages",
            _ => return Err(anyhow::anyhow!("❌ Unsupported LLM provider: {}", self.config.llm.provider)),
        };

        let auth_header = match self.config.llm.provider.as_str() {
            "openai" => format!("Bearer {}", api_key),
            "anthropic" => api_key,
            _ => return Err(anyhow::anyhow!("❌ Unsupported LLM provider: {}", self.config.llm.provider)),
        };

        let mut request = self.client.post(url)
            .header("Content-Type", "application/json")
            .json(&payload);

        request = match self.config.llm.provider.as_str() {
            "openai" => request.header("Authorization", auth_header),
            "anthropic" => request
                .header("x-api-key", auth_header)
                .header("anthropic-version", "2023-06-01"),
            _ => request,
        };

        println!("🌐 Sending request to {} API...", self.config.llm.provider);
        let response = match request.send().await {
            Ok(resp) => {
                let status = resp.status();
                if !status.is_success() {
                    let error_text = resp.text().await.unwrap_or_else(|_| "Unable to read error response".to_string());
                    return Err(anyhow::anyhow!(
                        "❌ {} API request failed with status {}: {}",
                        self.config.llm.provider.to_uppercase(),
                        status,
                        error_text
                    ));
                }
                resp
            },
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "❌ Failed to connect to {} API: {}. Check your internet connection.",
                    self.config.llm.provider.to_uppercase(),
                    e
                ));
            }
        };

        let response_json: serde_json::Value = match response.json().await {
            Ok(json) => json,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "❌ Failed to parse {} API response as JSON: {}",
                    self.config.llm.provider.to_uppercase(),
                    e
                ));
            }
        };
        
        // Check for API errors in response
        if let Some(error) = response_json.get("error") {
            let error_message = error.get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            let error_type = error.get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown");
            
            return Err(anyhow::anyhow!(
                "❌ {} API error ({}): {}",
                self.config.llm.provider.to_uppercase(),
                error_type,
                error_message
            ));
        }

        println!("✅ {} API request successful", self.config.llm.provider.to_uppercase());

        let content = match self.config.llm.provider.as_str() {
            "openai" => {
                response_json["choices"][0]["message"]["content"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("❌ Invalid OpenAI API response: missing content field"))?
                    .to_string()
            },
            "anthropic" => {
                response_json["content"][0]["text"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("❌ Invalid Anthropic API response: missing content field"))?
                    .to_string()
            },
            _ => return Err(anyhow::anyhow!("❌ Unsupported LLM provider: {}", self.config.llm.provider)),
        };

        if content.trim().is_empty() {
            return Err(anyhow::anyhow!("❌ {} API returned empty content", self.config.llm.provider.to_uppercase()));
        }

        println!("📝 Generated report with {} characters", content.len());
        Ok(content)
    }

    fn default_monthly_template(&self) -> String {
        "# {{project_name}} Monthly Report\n\n\
        **Report Period:** {{report_period}}\n\n\
        ## Progress Summary\n\n\
        {{activity_summary}}\n\n\
        ## Detailed Activities\n\n\
        {{commit_details}}\n\n\
        ## Quantitative Summary\n\n\
        - Total GitLab events: {{gitlab_events_count}}\n\
        - Total GitHub events: {{github_events_count}}\n\
        - Total Git commits: {{git_commits_count}}\n\
        - Total local files: {{local_files_count}}\n\
        - **Total items tracked:** {{total_items}}\n".to_string()
    }

    fn default_slides_template(&self) -> String {
        "# {{project_name}} Progress Update\n\n\
        **Period:** {{report_period}}\n\n\
        ---\n\n\
        ## Progress Metrics\n\n\
        {{metrics_summary}}\n\n\
        ---\n\n\
        ## Summary\n\n\
        - Total activities tracked: {{total_items}}\n\
        - Active development across {{git_commits_count}} commits\n\
        - Data collected from GitLab, GitHub, and local sources\n".to_string()
    }
}

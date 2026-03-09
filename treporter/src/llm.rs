use crate::config::Config;
use crate::types::CollectedData;
use crate::preprocessor::DataPreprocessor;
use crate::ollama::OllamaClient;
use crate::structured_extractor::StructuredData;
use crate::retry_loop::{RetryLoop, validate_min_length};
use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use tokio::fs;
use chrono;

pub struct LLMClient {
    client: Client,
    config: Config,
    ollama_client: Option<OllamaClient>,
    // Dynamic configuration for chunking based on model capabilities
    max_context_tokens: usize,
    target_chunk_tokens: usize,
    max_items_per_chunk: usize,
    // Rate limiting
    requests_per_minute: u32,
    #[allow(dead_code)]
    tokens_per_minute: u32,
    use_ollama: bool,
}

impl LLMClient {
    pub fn new(config: Config) -> Self {
        // Check if we should use Ollama
        let use_ollama = config.llm.provider == "ollama" || 
                        std::env::var("USE_OLLAMA").is_ok() ||
                        std::env::var("OPENAI_API_KEY").is_err();
        
        // Initialize Ollama client if needed
        let ollama_client = if use_ollama {
            Some(OllamaClient::new(None, Some(config.llm.model.clone())))
        } else {
            None
        };
        
        // Model-specific configurations
        let (max_context_tokens, requests_per_minute, tokens_per_minute) = if use_ollama {
            // Configure based on specific Ollama model
            match config.llm.model.as_str() {
                "deepseek-coder:33b-instruct" => (32_768, 100, 50_000), // Large model with more context
                "deepseek-coder:6.7b-instruct" => (8_192, 500, 80_000), // Medium model
                "codellama:34b-instruct" => (16_384, 100, 40_000), // Large CodeLlama
                "codellama:13b-instruct" => (8_192, 300, 60_000), // Medium CodeLlama
                "llama3.1:8b" => (8_192, 1000, 100_000), // General model
                _ => (8_192, 500, 80_000), // Conservative defaults for unknown models
            }
        } else {
            match config.llm.model.as_str() {
                "gpt-4-turbo" => (128_000, 500, 150_000), // 128K context, rate limits for gpt-4-turbo
                "gpt-4" => (8_192, 200, 40_000),         // 8K context, standard gpt-4 limits
                "gpt-3.5-turbo" => (16_385, 3500, 90_000), // 16K context, higher rate limits
                "claude-3-opus" => (200_000, 1000, 400_000), // 200K context, Anthropic limits
                "claude-3-sonnet" => (200_000, 1000, 400_000),
                "claude-3-haiku" => (200_000, 1000, 400_000),
                _ => (8_192, 200, 40_000), // Conservative defaults
            }
        };

        // Calculate optimal chunk size: use 60% of context for data, 40% for response/system prompt
        let target_chunk_tokens = (max_context_tokens as f64 * 0.6) as usize;
        
        // Adjust max items per chunk based on model capability
        let max_items_per_chunk = match config.llm.model.as_str() {
            "deepseek-coder:33b-instruct" => 25, // Large model can handle more items
            "codellama:34b-instruct" => 20,      // Large CodeLlama
            "deepseek-coder:6.7b-instruct" => 15, // Medium model
            "codellama:13b-instruct" => 15,      // Medium CodeLlama
            _ => 10, // Conservative for smaller/unknown models
        };
        
        Self {
            client: Client::new(),
            config,
            ollama_client,
            max_context_tokens,
            target_chunk_tokens,
            max_items_per_chunk,
            requests_per_minute,
            tokens_per_minute,
            use_ollama,
        }
    }

    pub async fn generate_monthly_report(&self, data: &CollectedData, template: &str) -> Result<String> {
        println!("🧠 Starting intelligent LLM-powered report generation with preprocessing...");
        
        // Use the provided template instead of hardcoded one
        let template_content = template.to_string();

        // Step 1: Preprocess data locally to reduce size and improve relevance
        let preprocessor = DataPreprocessor::new();
        let structured_content = preprocessor.preprocess_for_llm(data)?;
        
        println!("📊 Preprocessed data to {} characters (estimated {} tokens)", 
                structured_content.len(), 
                self.estimate_tokens(&structured_content));
        
        // Step 2: Check if we can send this as a single request
        let content_tokens = self.estimate_tokens(&structured_content);
        
        if content_tokens <= self.max_context_tokens {
            println!("✅ Content fits in single request, using template-based processing...");
            return self.generate_template_based_report(&structured_content, &template_content).await;
        }
        
        // Step 3: If still too large, use intelligent chunking on the preprocessed data
        println!("📝 Content still large, using intelligent chunking...");
        self.generate_chunked_report(&structured_content, &template).await
    }

    #[allow(dead_code)]
    async fn generate_single_report(&self, content: &str, template: &str) -> Result<String> {
        let prompt = format!(
            "You are an expert technical writer creating a project status report.\n\n\
             Using the following structured project data, generate a comprehensive report \
             following this template format:\n\n{}\n\n\
             PROJECT DATA:\n{}\n\n\
             Generate the complete report with specific details from the data.",
            template, content
        );

        let response = if self.use_ollama {
            if let Some(ollama_client) = &self.ollama_client {
                match ollama_client.generate(&prompt).await {
                    Ok(response) => response,
                    Err(e) => {
                        println!("⚠️  Ollama failed: {}", e);
                        return Ok(self.generate_fallback_report_from_content(content));
                    }
                }
            } else {
                println!("⚠️  Ollama client not initialized");
                return Ok(self.generate_fallback_report_from_content(content));
            }
        } else {
            match self.call_llm_api_with_retry(&prompt, 3).await {
                Ok(response) => response,
                Err(e) => {
                    println!("⚠️  LLM API failed: {}", e);
                    return Ok(self.generate_fallback_report_from_content(content));
                }
            }
        };

        Ok(response)
    }

    async fn generate_chunked_report(&self, content: &str, template: &str) -> Result<String> {
        // Split preprocessed content into logical sections
        let sections: Vec<&str> = content.split("\n\n---\n\n").collect();
        let mut section_summaries = Vec::new();
        
        println!("📊 Processing {} content sections...", sections.len());
        
        for (i, section) in sections.iter().enumerate() {
            let section_tokens = self.estimate_tokens(section);
            println!("  📝 Processing section {}/{} (~{} tokens)", 
                    i + 1, sections.len(), section_tokens);
                    
            if section_tokens > self.max_context_tokens {
                println!("    ⚠️  Section too large, creating summary fallback");
                section_summaries.push(format!("Section {}: Content too large for processing", i + 1));
                continue;
            }
            
            let section_prompt = format!(
                "Summarize the key accomplishments, features, and technical progress from this project data section:\n\n{}",
                section
            );
            
            let summary_result = if self.use_ollama {
                if let Some(ollama_client) = &self.ollama_client {
                    ollama_client.generate(&section_prompt).await
                } else {
                    Err(anyhow::anyhow!("Ollama client not initialized"))
                }
            } else {
                self.call_llm_api_with_retry(&section_prompt, 3).await
            };
            
            match summary_result {
                Ok(summary) => {
                    section_summaries.push(summary);
                    // Small delay between sections
                    sleep(Duration::from_millis(100)).await;
                }
                Err(e) => {
                    println!("    ⚠️  Failed to process section: {}", e);
                    section_summaries.push(format!("Section {}: Processing failed - {}", i + 1, e));
                }
            }
        }
        
        // Combine summaries and generate final report
        let combined_summaries = section_summaries.join("\n\n");
        let final_prompt = format!(
            "Using these section summaries, generate a comprehensive project report following this template:\n\n{}\n\n\
             SECTION SUMMARIES:\n{}\n\n\
             Create a cohesive, detailed report.",
            template, combined_summaries
        );
        
        let final_result = if self.use_ollama {
            if let Some(ollama_client) = &self.ollama_client {
                ollama_client.generate(&final_prompt).await
            } else {
                Err(anyhow::anyhow!("Ollama client not initialized"))
            }
        } else {
            self.call_llm_api_with_retry(&final_prompt, 3).await
        };
        
        match final_result {
            Ok(response) => Ok(response),
            Err(e) => {
                println!("⚠️  Final report generation failed: {}", e);
                Ok(self.generate_fallback_report_from_content(&combined_summaries))
            }
        }
    }

    fn generate_fallback_report_from_content(&self, content: &str) -> String {
        format!(
            "# Project Status Report\n\n\
             **Note:** This report was generated using fallback processing due to LLM API limitations.\n\n\
             ## Summary\n\n\
             Based on the collected project data, here is a structured overview:\n\n\
             {}\n\n\
             ## Data Overview\n\n\
             - Total content processed: {} characters\n\
             - Report generated: {}\n\n\
             *For a more detailed analysis, please review the source data or try again when API limits are restored.*",
            content,
            content.len(),
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        )
    }

    pub async fn generate_group_slides(&self, data: &CollectedData) -> Result<String> {
        println!("🧠 Starting intelligent LLM-powered slides generation with chunking...");
        
        // Read the template
        let template = std::fs::read_to_string("templates/group_slides.md")
            .unwrap_or_else(|_| self.default_slides_template());

        // Step 1: Generate summaries for each data type using chunking
        let summaries = self.generate_data_summaries(data).await?;
        
        // Step 2: Combine summaries into final slides
        let final_slides = self.synthesize_final_slides(&template, &summaries, data).await?;
        
        Ok(final_slides)
    }

    /// Generate summaries for each data type using intelligent chunking
    #[allow(dead_code)]
    fn generate_fallback_report(&self, data: &CollectedData, template: &str) -> String {
        let mut report = template.to_string();
        
        // Parse GitLab organized data to extract detailed metrics
        let gitlab_commits = data.gitlab.commits.len();
        let gitlab_issues = data.gitlab.issues.len();
        let gitlab_merge_requests = data.gitlab.merge_requests.len();
        let gitlab_comments = data.gitlab.comments.len();
        
        let github_commits = data.github.commits.len();
        let github_issues = data.github.issues.len();
        let github_pull_requests = data.github.pull_requests.len();
        let github_comments = data.github.comments.len();
        
        let mut gitlab_commit_details = Vec::new();
        
        for item in &data.gitlab.commits {
            if let (Some(author), Some(message), Some(date)) = (
                item.get("author_name").and_then(|v| v.as_str()),
                item.get("message").and_then(|v| v.as_str()),
                item.get("created_at").and_then(|v| v.as_str()),
            ) {
                gitlab_commit_details.push(format!("- {} by {} on {}", message, author, date));
            }
        }
        
        // Basic replacements
        report = report.replace("{{project_name}}", &self.config.project.name);
        report = report.replace("{{report_period}}", &format!("{} to {}", 
            data.metadata.date_range_start.format("%B %Y"),
            data.metadata.date_range_end.format("%B %Y")));
        
        // Quantitative summary with organized data
        let total_items = data.total_items();
        
        let activity_summary = format!(
            "**Progress Metrics for Reporting Period:**\n\n\
            **Total Activities Tracked: {}**\n\
            - 📝 {} local file updates\n\
            - 🔗 {} local git commits\n\
            - 🦊 {} GitLab activities ({} commits, {} issues, {} MRs, {} comments)\n\
            - 🐙 {} GitHub activities ({} commits, {} issues, {} PRs, {} comments)\n\n\
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
            data.gitlab.total_items(),
            gitlab_commits,
            gitlab_issues,
            gitlab_merge_requests,
            gitlab_comments,
            data.github.total_items(),
            github_commits,
            github_issues,
            github_pull_requests,
            github_comments,
            gitlab_commits,
            data.git_commits.len(),
            gitlab_issues,
            gitlab_merge_requests,
            data.metadata.sources_used.join(", "),
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
        report = report.replace("{{gitlab_events_count}}", &data.gitlab.total_items().to_string());
        report = report.replace("{{github_events_count}}", &data.github.total_items().to_string());
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
        slides = slides.replace("{{project_name}}", &self.config.project.name);
        slides = slides.replace("{{report_period}}", &format!("{} to {}", 
            data.metadata.date_range_start.format("%B %Y"),
            data.metadata.date_range_end.format("%B %Y")));
        
        // Quantitative summary for slides
        let total_items = data.total_items();
        
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
            data.gitlab.total_items(),
            data.github.total_items(),
            data.local_files.len(),
            data.metadata.sources_used.len()
        );

        slides = slides.replace("{{metrics_summary}}", &metrics_summary);
        
        // Replace any remaining placeholders
        slides = slides.replace("{{gitlab_events_count}}", &data.gitlab.total_items().to_string());
        slides = slides.replace("{{github_events_count}}", &data.github.total_items().to_string());
        slides = slides.replace("{{git_commits_count}}", &data.git_commits.len().to_string());
        slides = slides.replace("{{local_files_count}}", &data.local_files.len().to_string());
        slides = slides.replace("{{total_items}}", &total_items.to_string());
        
        slides
    }

    /// Generate summaries for each data type using intelligent chunking
    async fn generate_data_summaries(&self, data: &CollectedData) -> Result<HashMap<String, String>> {
        let mut summaries = HashMap::new();
        
        // Process GitLab data
        if data.gitlab.total_items() > 0 {
            println!("📊 Processing GitLab data ({} items)...", data.gitlab.total_items());
            
            if !data.gitlab.commits.is_empty() {
                let summary = self.process_data_chunks("gitlab_commits", &data.gitlab.commits).await?;
                summaries.insert("gitlab_commits".to_string(), summary);
            }
            
            if !data.gitlab.merge_requests.is_empty() {
                let summary = self.process_data_chunks("gitlab_merge_requests", &data.gitlab.merge_requests).await?;
                summaries.insert("gitlab_merge_requests".to_string(), summary);
            }
            
            if !data.gitlab.issues.is_empty() {
                let summary = self.process_data_chunks("gitlab_issues", &data.gitlab.issues).await?;
                summaries.insert("gitlab_issues".to_string(), summary);
            }
            
            if !data.gitlab.comments.is_empty() {
                let summary = self.process_data_chunks("gitlab_comments", &data.gitlab.comments).await?;
                summaries.insert("gitlab_comments".to_string(), summary);
            }
        }
        
        // Process GitHub data
        if data.github.total_items() > 0 {
            println!("📊 Processing GitHub data ({} items)...", data.github.total_items());
            
            if !data.github.commits.is_empty() {
                let summary = self.process_data_chunks("github_commits", &data.github.commits).await?;
                summaries.insert("github_commits".to_string(), summary);
            }
            
            if !data.github.pull_requests.is_empty() {
                let summary = self.process_data_chunks("github_pull_requests", &data.github.pull_requests).await?;
                summaries.insert("github_pull_requests".to_string(), summary);
            }
            
            if !data.github.issues.is_empty() {
                let summary = self.process_data_chunks("github_issues", &data.github.issues).await?;
                summaries.insert("github_issues".to_string(), summary);
            }
            
            if !data.github.comments.is_empty() {
                let summary = self.process_data_chunks("github_comments", &data.github.comments).await?;
                summaries.insert("github_comments".to_string(), summary);
            }
        }
        
        // Process local Git commits
        if !data.git_commits.is_empty() {
            println!("📊 Processing local Git commits ({} items)...", data.git_commits.len());
            let git_commits_json: Vec<Value> = data.git_commits.iter()
                .map(|commit| json!({
                    "hash": commit.hash,
                    "message": commit.message,
                    "author": commit.author_name,
                    "email": commit.author_email,
                    "date": commit.timestamp,
                    "files_changed": commit.files_changed,
                    "repo_path": commit.repo_path
                }))
                .collect();
            
            let summary = self.process_data_chunks("local_git_commits", &git_commits_json).await?;
            summaries.insert("local_git_commits".to_string(), summary);
        }
        
        Ok(summaries)
    }

    /// Process data chunks for a specific data type with caching and smart rate limiting
    async fn process_data_chunks(&self, data_type: &str, items: &[Value]) -> Result<String> {
        let chunks = self.create_chunks(items);
        let mut chunk_summaries = Vec::new();
        
        let total_estimated_tokens: usize = chunks.iter()
            .map(|chunk| {
                let chunk_json = serde_json::to_string(chunk).unwrap_or_default();
                self.estimate_tokens(&chunk_json)
            })
            .sum();
        
        println!("  🔄 Processing {} chunks for {} (estimated {} tokens)", 
                chunks.len(), data_type, total_estimated_tokens);
        
        // Check for cached results first
        let cache_dir = "cache/chunk_summaries";
        if let Err(_) = fs::create_dir_all(&cache_dir).await {
            println!("⚠️  Failed to create cache directory, proceeding without cache");
        }
        
        let mut successful_requests = 0;
        let mut failed_requests = 0;
        let start_time = std::time::Instant::now();
        
        for (i, chunk) in chunks.iter().enumerate() {
            let chunk_hash = self.calculate_chunk_hash(chunk);
            let cache_file = format!("{}/{}_{}.json", cache_dir, data_type, chunk_hash);
            
            // Try to load from cache first
            if let Ok(cached_content) = fs::read_to_string(&cache_file).await {
                if let Ok(cached_summary) = serde_json::from_str::<String>(&cached_content) {
                    println!("    💾 Using cached result for chunk {}/{}", i + 1, chunks.len());
                    chunk_summaries.push(cached_summary);
                    continue;
                }
            }
            
            let chunk_json = serde_json::to_string(chunk).unwrap_or_default();
            let chunk_tokens = self.estimate_tokens(&chunk_json);
            
            println!("    📝 Analyzing chunk {}/{} ({} items, ~{} tokens)", 
                    i + 1, chunks.len(), chunk.len(), chunk_tokens);
            
            if chunk_tokens > self.max_context_tokens {
                println!("    ⚠️  Chunk too large ({} tokens > {} limit), using fallback", 
                        chunk_tokens, self.max_context_tokens);
                let fallback = self.create_fallback_chunk_summary(data_type, chunk);
                chunk_summaries.push(fallback.clone());
                
                // Cache the fallback result too
                self.cache_chunk_result(&cache_file, &fallback).await;
                continue;
            }
            
            // Smart rate limiting: only delay if we've been making requests frequently
            if successful_requests > 0 {
                let delay = self.calculate_smart_delay(successful_requests, failed_requests, start_time);
                if delay > 0 {
                    println!("    ⏱️  Smart delay: {}ms (success rate: {}/{})", 
                            delay, successful_requests, successful_requests + failed_requests);
                    sleep(Duration::from_millis(delay)).await;
                }
            }
            
            match self.analyze_chunk(data_type, chunk).await {
                Ok(summary) => {
                    chunk_summaries.push(summary.clone());
                    successful_requests += 1;
                    
                    // Cache the successful result
                    self.cache_chunk_result(&cache_file, &summary).await;
                }
                Err(e) => {
                    println!("    ⚠️  Failed to analyze chunk {}: {}", i + 1, e);
                    failed_requests += 1;
                    
                    // Create and cache fallback summary
                    let fallback = self.create_fallback_chunk_summary(data_type, chunk);
                    chunk_summaries.push(fallback.clone());
                    self.cache_chunk_result(&cache_file, &fallback).await;
                }
            }
        }
        
        let success_rate = if successful_requests + failed_requests > 0 {
            (successful_requests as f64 / (successful_requests + failed_requests) as f64) * 100.0
        } else {
            100.0
        };
        
        println!("  ✅ Completed {} with {:.1}% success rate ({}/{} successful)", 
                data_type, success_rate, successful_requests, successful_requests + failed_requests);
        
        // Combine chunk summaries
        let combined_summary = format!(
            "# {} Summary\n\n**Total Items:** {}\n**Chunks Processed:** {}\n**Estimated Tokens:** {}\n**Success Rate:** {:.1}%\n\n## Analysis:\n\n{}", 
            data_type, 
            items.len(), 
            chunks.len(),
            total_estimated_tokens,
            success_rate,
            chunk_summaries.join("\n\n---\n\n")
        );
        
        Ok(combined_summary)
    }

    fn calculate_chunk_hash(&self, chunk: &[Value]) -> String {
        // Simple hash based on content - for caching purposes
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        let chunk_json = serde_json::to_string(chunk).unwrap_or_default();
        chunk_json.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    async fn cache_chunk_result(&self, cache_file: &str, result: &str) {
        if let Ok(json_result) = serde_json::to_string(result) {
            if let Err(_) = fs::write(cache_file, json_result).await {
                // Silently fail if we can't cache - not critical
            }
        }
    }

    fn calculate_smart_delay(&self, successful_requests: u32, failed_requests: u32, start_time: std::time::Instant) -> u64 {
        // Smart rate limiting based on actual performance
        let elapsed_ms = start_time.elapsed().as_millis() as u64;
        let total_requests = successful_requests + failed_requests;
        
        if total_requests == 0 {
            return 0;
        }
        
        // Calculate current request rate (requests per minute)
        let current_rpm = if elapsed_ms > 0 {
            (total_requests as u64 * 60_000) / elapsed_ms
        } else {
            0
        };
        
        // If we're well below our limit, don't delay much
        let target_rpm = (self.requests_per_minute as f64 * 0.8) as u64; // 80% of limit for safety
        
        if current_rpm < target_rpm {
            // We're safe, minimal delay
            return 50; // Just 50ms to be polite
        }
        
        // If we have failures, be more conservative
        if failed_requests > 0 {
            let failure_rate = failed_requests as f64 / total_requests as f64;
            if failure_rate > 0.2 {
                // High failure rate, slow down significantly
                return 5000; // 5 seconds
            } else if failure_rate > 0.1 {
                // Moderate failure rate
                return 2000; // 2 seconds
            }
        }
        
        // Calculate delay needed to stay at target rate
        let ms_per_request = 60_000 / target_rpm;
        ms_per_request
    }

    /// Create chunks from data items based on estimated token count
    fn create_chunks(&self, items: &[Value]) -> Vec<Vec<Value>> {
        let mut chunks = Vec::new();
        let mut current_chunk = Vec::new();
        let mut current_tokens = 0;
        
        for item in items {
            let item_json = serde_json::to_string(item).unwrap_or_default();
            let estimated_tokens = self.estimate_tokens(&item_json);
            
            // Check if adding this item would exceed our token or item limits
            if current_chunk.len() >= self.max_items_per_chunk || 
               current_tokens + estimated_tokens > self.target_chunk_tokens {
                
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk.clone());
                    current_chunk.clear();
                    current_tokens = 0;
                }
            }
            
            current_chunk.push(item.clone());
            current_tokens += estimated_tokens;
        }
        
        // Add the last chunk if it has items
        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }
        
        if chunks.is_empty() && !items.is_empty() {
            // Fallback: if even a single item is too large, create single-item chunks
            println!("⚠️  Items too large for target chunk size, creating single-item chunks");
            chunks = items.iter().map(|item| vec![item.clone()]).collect();
        }
        
        chunks
    }

    /// Estimate token count for text (rough approximation: 1 token ≈ 4 characters for English)
    fn estimate_tokens(&self, text: &str) -> usize {
        // Improved token estimation:
        // - Regular text: ~4 chars per token
        // - Code/JSON: ~3 chars per token (more dense)
        // - Add some padding for safety
        let base_estimate = if text.contains('{') || text.contains('<') || text.contains("---") {
            text.len() / 3  // Code/structured data is more token-dense
        } else {
            text.len() / 4  // Regular text
        };
        
        // Add 20% padding for safety
        (base_estimate as f64 * 1.2) as usize
    }

    /// Analyze a single chunk using LLM
    async fn analyze_chunk(&self, data_type: &str, chunk: &[Value]) -> Result<String> {
        let chunk_template = std::fs::read_to_string("templates/chunk_summary.md")
            .unwrap_or_else(|_| self.default_chunk_template());
        
        let data_content = chunk.iter()
            .enumerate()
            .map(|(i, item)| format!("Item {}:\n{}", i + 1, serde_json::to_string_pretty(item).unwrap_or_default()))
            .collect::<Vec<_>>()
            .join("\n\n");
        
        let prompt = chunk_template
            .replace("{{chunk_type}}", data_type)
            .replace("{{item_count}}", &chunk.len().to_string())
            .replace("{{date_range}}", "2025-07-01 to 2025-07-11")
            .replace("{{data_content}}", &data_content);
        
        let prompt_tokens = self.estimate_tokens(&prompt);
        
        if prompt_tokens > self.max_context_tokens * 3 / 4 {
            return Err(anyhow::anyhow!(
                "Prompt too large: {} tokens (max context: {})", 
                prompt_tokens, self.max_context_tokens
            ));
        }
        
        self.call_llm_api(&prompt).await
    }

    /// Create a fallback summary for a chunk when LLM fails
    fn create_fallback_chunk_summary(&self, data_type: &str, chunk: &[Value]) -> String {
        format!(
            "## Fallback Summary for {}\n\n**Items:** {}\n\n**Note:** LLM analysis failed, showing basic statistics only.\n\n**Items in this chunk:** {}", 
            data_type, 
            chunk.len(),
            chunk.iter()
                .enumerate()
                .map(|(i, item)| {
                    // Extract key fields based on data type
                    match data_type {
                        dt if dt.contains("commit") => {
                            format!("{}. Commit: {}", i + 1, 
                                item.get("title").or(item.get("message")).and_then(|v| v.as_str()).unwrap_or("No title"))
                        }
                        dt if dt.contains("issue") => {
                            format!("{}. Issue: {}", i + 1, 
                                item.get("title").and_then(|v| v.as_str()).unwrap_or("No title"))
                        }
                        dt if dt.contains("merge_request") || dt.contains("pull_request") => {
                            format!("{}. MR/PR: {}", i + 1, 
                                item.get("title").and_then(|v| v.as_str()).unwrap_or("No title"))
                        }
                        _ => format!("{}. Item: {}", i + 1, 
                            item.get("title").or(item.get("body")).and_then(|v| v.as_str()).unwrap_or("No description"))
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    /// Synthesize the final report from all summaries
    #[allow(dead_code)]
    async fn synthesize_final_report(&self, template: &str, summaries: &HashMap<String, String>, data: &CollectedData) -> Result<String> {
        println!("🎯 Synthesizing final report from {} summaries...", summaries.len());
        
        let summaries_text = summaries.iter()
            .map(|(key, summary)| format!("## {}\n\n{}", key, summary))
            .collect::<Vec<_>>()
            .join("\n\n");
        
        let prompt = format!(
            "You are a project manager creating a comprehensive monthly technical report for the {project} project. \
            Based on the following data summaries, generate a complete report using the template provided.\n\n\
            IMPORTANT INSTRUCTIONS:\n\
            - The data summaries below contain analyzed information from different sources (GitLab, GitHub, local Git)\n\
            - When counting commits, identify duplicates across sources using commit hashes/IDs and count each unique commit only once\n\
            - Focus on unique contributions and avoid double-counting activities\n\
            - Provide accurate quantitative metrics by deduplicating based on commit hashes, issue IDs, or other unique identifiers\n\
            - Fill in ALL template placeholders with specific, accurate information\n\
            - Include concrete metrics, progress indicators, and detailed technical insights\n\n\
            METADATA:\n\
            - Collection Period: {} to {}\n\
            - Data Sources: {}\n\
            - Total Items Collected: {}\n\n\
            DATA SUMMARIES:\n{}\n\n\
            TEMPLATE TO FILL:\n{}\n\n\
            Generate a complete, professional report with all placeholders filled in, including specific metrics and progress indicators. \
            Ensure all counts and metrics reflect unique activities (no duplicates across sources).",
            data.metadata.date_range_start.format("%Y-%m-%d"),
            data.metadata.date_range_end.format("%Y-%m-%d"),
            data.metadata.sources_used.join(", "),
            data.total_items(),
            summaries_text,
            template,
            project = &self.config.project.name
        );
        
        match self.call_llm_api(&prompt).await {
            Ok(response) => Ok(response),
            Err(e) => {
                println!("⚠️  Final report synthesis failed: {}", e);
                println!("🔄 Falling back to template-based report generation...");
                Ok(self.generate_fallback_report(data, template))
            }
        }
    }

    /// Synthesize the final slides from all summaries
    async fn synthesize_final_slides(&self, template: &str, summaries: &HashMap<String, String>, data: &CollectedData) -> Result<String> {
        println!("🎯 Synthesizing final slides from {} summaries...", summaries.len());
        
        let summaries_text = summaries.iter()
            .map(|(key, summary)| format!("## {}\n\n{}", key, summary))
            .collect::<Vec<_>>()
            .join("\n\n");
        
        let prompt = format!(
            "You are preparing slides for a group presentation on the {project} project. \
            Based on the following data summaries, generate presentation slides using the template provided. \
            Focus on progress metrics, key achievements, and quantitative summaries.\n\n\
            IMPORTANT INSTRUCTIONS:\n\
            - The data summaries below contain analyzed information from different sources (GitLab, GitHub, local Git)\n\
            - When counting commits, identify duplicates across sources using commit hashes/IDs and count each unique commit only once\n\
            - Focus on unique contributions and avoid double-counting activities\n\
            - Provide accurate quantitative metrics by deduplicating based on commit hashes, issue IDs, or other unique identifiers\n\
            - Fill in ALL template placeholders with specific, accurate information\n\
            - Include concrete metrics, progress indicators, and visual elements for presentations\n\n\
            METADATA:\n\
            - Collection Period: {} to {}\n\
            - Data Sources: {}\n\
            - Total Items Collected: {}\n\n\
            DATA SUMMARIES:\n{}\n\n\
            TEMPLATE TO FILL:\n{}\n\n\
            Generate complete slides with all placeholders filled in, emphasizing visual and quantitative metrics. \
            Ensure all counts and metrics reflect unique activities (no duplicates across sources).",
            data.metadata.date_range_start.format("%Y-%m-%d"),
            data.metadata.date_range_end.format("%Y-%m-%d"),
            data.metadata.sources_used.join(", "),
            data.total_items(),
            summaries_text,
            template,
            project = &self.config.project.name
        );
        
        match self.call_llm_api(&prompt).await {
            Ok(response) => Ok(response),
            Err(e) => {
                println!("⚠️  Final slides synthesis failed: {}", e);
                println!("🔄 Falling back to template-based slides generation...");
                Ok(self.generate_fallback_slides(data, template))
            }
        }
    }

    /// Generate comprehensive architectural analysis reports
    pub async fn generate_architectural_analysis(&self, data: &CollectedData) -> Result<String> {
        println!("🏗️  Starting multi-stage architectural analysis...");
        
        // Read the architectural analysis template
        let template = match std::fs::read_to_string("templates/architectural-analysis.md") {
            Ok(content) => content,
            Err(_) => {
                println!("⚠️  Architectural analysis template not found, using default");
                self.default_architectural_template()
            }
        };

        // SEND TECHNICAL CONTENT TO LLM - extract actual commit diffs and technical details
        let mut technical_content = String::new();
        
        // Extract GitLab commit technical content
        technical_content.push_str("# GitLab Technical Changes\n\n");
        for commit in &data.gitlab.commits {
            if let Some(title) = commit.get("title").and_then(|v| v.as_str()) {
                technical_content.push_str(&format!("## Commit: {}\n", title));
            }
            if let Some(id) = commit.get("id").and_then(|v| v.as_str()) {
                technical_content.push_str(&format!("ID: {}\n", id));
            }
            if let Some(author) = commit.get("author_name").and_then(|v| v.as_str()) {
                technical_content.push_str(&format!("Author: {}\n", author));
            }
            if let Some(message) = commit.get("message").and_then(|v| v.as_str()) {
                technical_content.push_str(&format!("Message: {}\n", message));
            }
            if let Some(diff) = commit.get("_diff").and_then(|v| v.as_str()) {
                technical_content.push_str(&format!("Diff:\n```\n{}\n```\n", diff));
            }
            technical_content.push_str("\n---\n\n");
        }
        
        // Extract GitHub commit technical content
        technical_content.push_str("# GitHub Technical Changes\n\n");
        for commit in &data.github.commits {
            if let Some(title) = commit.get("title").and_then(|v| v.as_str()) {
                technical_content.push_str(&format!("## Commit: {}\n", title));
            }
            if let Some(id) = commit.get("id").and_then(|v| v.as_str()) {
                technical_content.push_str(&format!("ID: {}\n", id));
            }
            if let Some(author) = commit.get("author_name").and_then(|v| v.as_str()) {
                technical_content.push_str(&format!("Author: {}\n", author));
            }
            if let Some(message) = commit.get("message").and_then(|v| v.as_str()) {
                technical_content.push_str(&format!("Message: {}\n", message));
            }
            if let Some(diff) = commit.get("_diff").and_then(|v| v.as_str()) {
                technical_content.push_str(&format!("Diff:\n```\n{}\n```\n", diff));
            }
            technical_content.push_str("\n---\n\n");
        }
        
        // Extract Local Git commit technical content 
        technical_content.push_str("# Local Git Technical Changes\n\n");
        for commit in &data.git_commits {
            technical_content.push_str(&format!("## Commit: {}\n", commit.message));
            technical_content.push_str(&format!("ID: {}\n", commit.hash));
            technical_content.push_str(&format!("Author: {} <{}>\n", commit.author_name, commit.author_email));
            technical_content.push_str(&format!("Files Changed: {:?}\n", commit.files_changed));
            technical_content.push_str("\n---\n\n");
        }
        
        // Add metadata
        technical_content.push_str(&format!(
            "# Analysis Metadata\n\
            - Analysis Period: {} to {}\n\
            - Total Items: {}\n\
            - Total Content Length: {} characters\n",
            data.metadata.date_range_start.format("%Y-%m-%d"),
            data.metadata.date_range_end.format("%Y-%m-%d"),
            data.total_items(),
            technical_content.len()
        ));
        
        let structured_content = technical_content;
        
        println!("📊 Architectural analysis - preprocessed data: {} characters", structured_content.len());
        
        // Stage 1: Extract technical facts and categorize changes
        let technical_analysis = self.extract_technical_facts(&structured_content).await?;
        
        // Stage 2: Identify architectural patterns and themes
        let architectural_themes = self.identify_architectural_themes(&technical_analysis, &structured_content).await?;
        
        // Stage 3: Synthesize high-level insights
        let synthesis = self.synthesize_architectural_insights(&architectural_themes, &technical_analysis).await?;
        
        // Stage 4: Generate the final report using the template
        self.generate_final_architectural_report(&template, &synthesis, data).await
    }

    /// Stage 1: Extract technical facts from commits and changes
    async fn extract_technical_facts(&self, content: &str) -> Result<String> {
        println!("🔍 Stage 1: Extracting technical facts from development activity...");
        
        // Load the sophisticated prompts from the TOML file
        let prompt_config = match std::fs::read_to_string("prompts/architectural-analysis.toml") {
            Ok(config_content) => {
                match toml::from_str::<toml::Value>(&config_content) {
                    Ok(config) => config,
                    Err(e) => {
                        println!("⚠️  Failed to parse prompt config: {}", e);
                        return Ok("Failed to load prompt configuration".to_string());
                    }
                }
            }
            Err(e) => {
                println!("⚠️  Failed to load prompt config: {}", e);
                return Ok("Failed to load prompt configuration".to_string());
            }
        };
        
        let base_context = prompt_config
            .get("architectural_analysis")
            .and_then(|section| section.get("base_context"))
            .and_then(|v| v.as_str())
            .unwrap_or("You are an expert software architect analyzing development activity.");
            
        let technical_foundation_prompt = prompt_config
            .get("architectural_analysis")
            .and_then(|section| section.get("technical_foundation_prompt"))
            .and_then(|v| v.as_str())
            .unwrap_or("Analyze the technical foundation of this project.");
        
        let prompt = format!(
            "{}\n\n{}\n\nDATA TO ANALYZE:\n{}\n\n\
            Please provide a detailed technical analysis focusing on concrete facts from the development activity.",
            base_context,
            technical_foundation_prompt,
            content
        );
        
        let response = if self.use_ollama {
            if let Some(ollama_client) = &self.ollama_client {
                ollama_client.generate(&prompt).await?
            } else {
                "Technical fact extraction unavailable.".to_string()
            }
        } else {
            self.call_llm_api_with_retry(&prompt, 3).await?
        };
        
        println!("✅ Stage 1 complete: Technical facts extracted");
        Ok(response)
    }

    /// Stage 2: Identify architectural themes and patterns
    async fn identify_architectural_themes(&self, technical_facts: &str, _raw_data: &str) -> Result<String> {
        println!("🎯 Stage 2: Identifying architectural themes and patterns...");
        
        // Load the sophisticated prompts from the TOML file
        let prompt_config = match std::fs::read_to_string("prompts/architectural-analysis.toml") {
            Ok(config_content) => {
                match toml::from_str::<toml::Value>(&config_content) {
                    Ok(config) => config,
                    Err(e) => {
                        println!("⚠️  Failed to parse prompt config: {}", e);
                        return Ok("Failed to load prompt configuration".to_string());
                    }
                }
            }
            Err(e) => {
                println!("⚠️  Failed to load prompt config: {}", e);
                return Ok("Failed to load prompt configuration".to_string());
            }
        };
        
        let base_context = prompt_config
            .get("architectural_analysis")
            .and_then(|section| section.get("base_context"))
            .and_then(|v| v.as_str())
            .unwrap_or("You are an expert software architect analyzing development activity.");
            
        let architecture_patterns_prompt = prompt_config
            .get("architectural_analysis")
            .and_then(|section| section.get("architecture_patterns_prompt"))
            .and_then(|v| v.as_str())
            .unwrap_or("Identify and analyze architectural patterns and design decisions.");
        
        let prompt = format!(
            "{}\n\n{}\n\nTECHNICAL FACTS:\n{}\n\n\
            Please analyze the technical facts to identify architectural patterns and themes.",
            base_context,
            architecture_patterns_prompt,
            technical_facts
        );
        
        let response = if self.use_ollama {
            if let Some(ollama_client) = &self.ollama_client {
                ollama_client.generate(&prompt).await?
            } else {
                "Architectural theme identification unavailable.".to_string()
            }
        } else {
            self.call_llm_api_with_retry(&prompt, 3).await?
        };
        
        println!("✅ Stage 2 complete: Architectural themes identified");
        Ok(response)
    }

    /// Stage 3: Synthesizes high-level architectural insights
    async fn synthesize_architectural_insights(&self, themes: &str, technical_facts: &str) -> Result<String> {
        println!("🧠 Stage 3: Synthesizing high-level architectural insights...");
        
        // Load the sophisticated prompts from the TOML file
        let prompt_config = match std::fs::read_to_string("prompts/architectural-analysis.toml") {
            Ok(config_content) => {
                match toml::from_str::<toml::Value>(&config_content) {
                    Ok(config) => config,
                    Err(e) => {
                        println!("⚠️  Failed to parse prompt config: {}", e);
                        return Ok("Failed to load prompt configuration".to_string());
                    }
                }
            }
            Err(e) => {
                println!("⚠️  Failed to load prompt config: {}", e);
                return Ok("Failed to load prompt configuration".to_string());
            }
        };
        
        let base_context = prompt_config
            .get("architectural_analysis")
            .and_then(|section| section.get("base_context"))
            .and_then(|v| v.as_str())
            .unwrap_or("You are an expert software architect analyzing development activity.");
            
        let executive_summary_prompt = prompt_config
            .get("architectural_analysis")
            .and_then(|section| section.get("executive_summary_prompt"))
            .and_then(|v| v.as_str())
            .unwrap_or("Analyze this project data to create a comprehensive executive summary.");
        
        let prompt = format!(
            "{}\n\n{}\n\nARCHITECTURAL THEMES:\n{}\n\nTECHNICAL FACTS:\n{}\n\n\
            Please provide a comprehensive executive summary synthesizing the architectural insights.",
            base_context,
            executive_summary_prompt,
            themes,
            technical_facts
        );
        
        let response = if self.use_ollama {
            if let Some(ollama_client) = &self.ollama_client {
                ollama_client.generate(&prompt).await?
            } else {
                "Architectural synthesis unavailable.".to_string()
            }
        } else {
            self.call_llm_api_with_retry(&prompt, 3).await?
        };
        
        println!("✅ Stage 3 complete: Architectural insights synthesized");
        Ok(response)
    }

    /// Stage 4: Generate final architectural report using template
    async fn generate_final_architectural_report(&self, _template: &str, synthesis: &str, data: &CollectedData) -> Result<String> {
        println!("📝 Stage 4: Generating final architectural report using synthesized insights...");
        
        // Load the sophisticated prompts from the TOML file
        let prompt_config = match std::fs::read_to_string("prompts/architectural-analysis.toml") {
            Ok(config_content) => {
                match toml::from_str::<toml::Value>(&config_content) {
                    Ok(config) => config,
                    Err(e) => {
                        println!("⚠️  Failed to parse prompt config: {}", e);
                        return Ok("Failed to load prompt configuration".to_string());
                    }
                }
            }
            Err(e) => {
                println!("⚠️  Failed to load prompt config: {}", e);
                return Ok("Failed to load prompt configuration".to_string());
            }
        };
        
        let base_context = prompt_config
            .get("architectural_analysis")
            .and_then(|section| section.get("base_context"))
            .and_then(|v| v.as_str())
            .unwrap_or("You are an expert software architect analyzing development activity.");
            
        let final_report_prompt = prompt_config
            .get("architectural_analysis")
            .and_then(|section| section.get("final_report_prompt"))
            .and_then(|v| v.as_str())
            .unwrap_or("Create a comprehensive final architectural report.");
        
        // Create a comprehensive prompt using the sophisticated TOML prompts
        let final_prompt = format!(
            "{}\n\n{}\n\nARCHITECTURAL SYNTHESIS:\n{}\n\n\
             Please provide a comprehensive final architectural report.",
            base_context,
            final_report_prompt,
            synthesis
        );
        
        let response = if self.use_ollama {
            if let Some(ollama_client) = &self.ollama_client {
                match ollama_client.generate(&final_prompt).await {
                    Ok(response) => response,
                    Err(e) => {
                        println!("    ⚠️  Ollama failed for final report generation: {}", e);
                        return Err(e);
                    }
                }
            } else {
                return Err(anyhow::anyhow!("Ollama client not available"));
            }
        } else {
            match self.call_llm_api_with_retry(&final_prompt, 3).await {
                Ok(response) => response,
                Err(e) => {
                    println!("    ⚠️  LLM API failed for final report generation: {}", e);
                    return Err(e);
                }
            }
        };
        
        // Add metadata footer
        let date_range_start = data.metadata.date_range_start.format("%Y-%m-%d").to_string();
        let date_range_end = data.metadata.date_range_end.format("%Y-%m-%d").to_string();
        let sources_used = data.metadata.sources_used.join(", ");
        
        let final_report = format!(
            "{}\n\n---\n\n## Report Metadata\n\n- **Analysis Period**: {} to {}\n- **Data Sources**: {}\n- **Generated**: {}\n- **Analysis Method**: 4-Stage Architectural Synthesis\n",
            response, date_range_start, date_range_end, sources_used, 
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );
        
        println!("✅ Stage 4 complete: Final architectural report generated using synthesis");
        Ok(final_report)
    }

    // ...existing code...
    async fn call_llm_api(&self, prompt: &str) -> Result<String> {
        self.call_llm_api_with_retry(prompt, 3).await
    }

    async fn call_llm_api_with_retry(&self, prompt: &str, max_retries: u32) -> Result<String> {
        let mut attempt = 0;
        let mut last_error = None;

        while attempt < max_retries {
            match self.call_llm_api_once(prompt).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    let error_str = e.to_string().to_lowercase();
                    
                    // Check for rate limiting
                    if error_str.contains("rate limit") || error_str.contains("429") {
                        let delay = self.calculate_backoff_delay(attempt);
                        println!("⏱️  Rate limit hit, waiting {} seconds before retry {}/{}...", 
                                delay, attempt + 1, max_retries);
                        sleep(Duration::from_secs(delay)).await;
                    }
                    // Check for temporary server errors
                    else if error_str.contains("500") || error_str.contains("502") || error_str.contains("503") {
                        let delay = 2_u64.pow(attempt);
                        println!("🔄 Server error, retrying in {} seconds (attempt {}/{})...", 
                                delay, attempt + 1, max_retries);
                        sleep(Duration::from_secs(delay)).await;
                    }
                    // For other errors, don't retry
                    else {
                        return Err(e);
                    }
                    
                    last_error = Some(e);
                    attempt += 1;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Max retries exceeded")))
    }

    fn calculate_backoff_delay(&self, attempt: u32) -> u64 {
        // Exponential backoff with jitter for rate limits
        // Base delay of 60 seconds for rate limits, exponentially increasing
        let base_delay = 60_u64;
        let exponential_delay = base_delay * 2_u64.pow(attempt);
        
        // Add some jitter (±25%) to prevent thundering herd
        let jitter = (exponential_delay / 4) as u64;
        let min_delay = exponential_delay.saturating_sub(jitter);
        let max_delay = exponential_delay + jitter;
        
        // Use current time as simple random source
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos() as u64;
        min_delay + (seed % (max_delay - min_delay + 1))
    }

    async fn call_llm_api_once(&self, prompt: &str) -> Result<String> {
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

    async fn generate_template_based_report(&self, content: &str, template: &str) -> Result<String> {
        println!("🎯 Processing template-based report with LLM_PROMPT sections...");
        
        // Parse template to find LLM_PROMPT sections
        let llm_prompts = self.extract_llm_prompts(template);
        println!("📝 Found {} LLM_PROMPT sections to process", llm_prompts.len());
        
        let mut filled_template = template.to_string();
        
        // Process each LLM_PROMPT section
        for (i, (placeholder, prompt)) in llm_prompts.iter().enumerate() {
            println!("  📋 Processing prompt {}/{}: {}", i + 1, llm_prompts.len(), 
                    &prompt[..prompt.len().min(60)]);
            
            let full_prompt = format!(
                "You are an expert technical writer creating a project status report for the {project} project.\n\n\
                 Your task: {}\n\n\
                 Based on the following project activity data, provide a detailed response:\n\n\
                 === PROJECT ACTIVITY DATA ===\n{}\n\n\
                 === RESPONSE REQUIREMENTS ===\n\
                 - Be specific and concrete\n\
                 - Reference actual commits, projects, or accomplishments from the data\n\
                 - Use professional technical language\n\
                 - Focus on significant technical contributions\n\
                 - Include relevant details like project names, authors, dates when appropriate\n\n\
                 Your response:",
                prompt, content,
                project = &self.config.project.name
            );
            
            let response = if self.use_ollama {
                if let Some(ollama_client) = &self.ollama_client {
                    match ollama_client.generate(&full_prompt).await {
                        Ok(response) => response,
                        Err(e) => {
                            println!("    ⚠️  Ollama failed for prompt {}: {}", i + 1, e);
                            format!("Unable to generate response for this section due to processing error.")
                        }
                    }
                } else {
                    println!("    ⚠️  Ollama client not initialized");
                    format!("LLM processing unavailable.")
                }
            } else {
                match self.call_llm_api_with_retry(&full_prompt, 3).await {
                    Ok(response) => response,
                    Err(e) => {
                        println!("    ⚠️  LLM API failed for prompt {}: {}", i + 1, e);
                        format!("Unable to generate response for this section due to API error.")
                    }
                }
            };
            
            // Replace the placeholder with the generated response
            filled_template = filled_template.replace(placeholder, &response);
            
            // Small delay between prompts
            sleep(Duration::from_millis(500)).await;
        }
        
        Ok(filled_template)
    }

    fn extract_llm_prompts(&self, template: &str) -> Vec<(String, String)> {
        let mut prompts = Vec::new();
        
        println!("🔍 Template length: {} characters", template.len());
        println!("📝 Template snippet: {}", &template[..template.len().min(200)]);
        
        // Use a much simpler syntax: {LLM: prompt text here}
        let re = regex::Regex::new(r"(?s)\{LLM:\s*(.*?)\}").unwrap();
        
        println!("📊 Regex pattern: (?s)\\{{LLM:\\s*(.*?)\\}}");
        
        for cap in re.captures_iter(template) {
            let full_match = cap.get(0).unwrap().as_str();
            let prompt_text = cap.get(1).unwrap().as_str().trim();
            
            println!("  🔍 Found LLM prompt: {}", &prompt_text[..prompt_text.len().min(80)]);
            prompts.push((full_match.to_string(), prompt_text.to_string()));
        }
        
        if prompts.is_empty() {
            println!("⚠️  No LLM prompts found! Let's try the old [[LLM_PROMPT: syntax...");
            // Fallback to old syntax
            let re_old = regex::Regex::new(r"(?s)\[\[LLM_PROMPT:\s*(.*?)\]\]").unwrap();
            for cap in re_old.captures_iter(template) {
                let full_match = cap.get(0).unwrap().as_str();
                let prompt_text = cap.get(1).unwrap().as_str().trim();
                
                let cleaned_prompt = prompt_text.replace("\n", " ")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                
                println!("  🔍 Found old-style LLM_PROMPT: {}", &cleaned_prompt[..cleaned_prompt.len().min(80)]);
                prompts.push((full_match.to_string(), cleaned_prompt));
            }
        }
        
        prompts
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

    fn default_chunk_template(&self) -> String {
        "# Data Analysis\n\nAnalyze the following {{chunk_type}} data ({{item_count}} items):\n\n{{data_content}}\n\nProvide a summary with key metrics and insights.".to_string()
    }

    fn default_architectural_template(&self) -> String {
        r#"# Comprehensive Architectural Analysis Report

## Executive Summary

{LLM: Analyze the provided project data and create a comprehensive executive summary that includes: overall project health and momentum, key architectural decisions made during this period, major technical milestones achieved, critical challenges identified and addressed, and strategic direction and focus areas. Provide specific metrics and concrete examples from the data.}

## Technical Foundation Analysis

{LLM: Analyze the technical foundation of the project based on commit patterns, file changes, and development activity. Address: core technologies and frameworks being used, build system and infrastructure choices, key architectural components, integration patterns and data flows, and development workflow and tooling. Reference specific files, commits, or code changes from the data.}

## Codebase Evolution

{LLM: Examine the codebase evolution during this period by analyzing: new modules or components added, existing components enhanced or refactored, deprecated or removed functionality, code quality improvements, performance optimizations, and security enhancements. Provide specific examples with file paths, commit messages, and impact assessment.}

## Architecture Patterns and Decisions

{LLM: Identify and analyze key architectural patterns and decisions evident in the development activity: design patterns implemented, system architecture choices, data structures and algorithms selected, API design and interface patterns, error handling and resilience strategies, and configuration management approaches. Support each pattern with specific code examples or implementation details from the data.}

## Development Progress Metrics

{LLM: Provide quantitative analysis of development progress: lines of code added/modified/deleted, number of files created/modified, commit frequency and distribution, issue resolution rates, feature completion metrics, code review statistics, and testing coverage improvements. Present metrics in a clear, analytical format.}

## Technical Challenges and Solutions

{LLM: Identify and analyze major technical challenges faced and how they were addressed: complex problems encountered, research and investigation approaches, alternative solutions considered, implementation strategies chosen, trade-offs and compromises made, and validation and testing methods. Provide detailed technical analysis with specific examples.}

## Lessons Learned and Best Practices

{LLM: Extract key lessons learned and best practices from this development period: effective development practices observed, pitfalls avoided or encountered, process improvements implemented, tool and technology insights, team collaboration insights, and documentation and knowledge sharing improvements. Focus on actionable insights for future development.}

## Recommendations for Next Period

{LLM: Based on the architectural analysis, provide specific recommendations for the next development period: priority areas for improvement, technical debt to address, new features or capabilities to implement, process optimizations to consider, tools or technologies to evaluate, and team skill development needs. Provide prioritized, actionable recommendations.}

---

**Report Generation Metadata:**
- Analysis Period: {{date_range_start}} to {{date_range_end}}
- Data Sources: {{sources_used}}
- Total Activities Analyzed: {{total_items}}
- Report Generated: {{generation_timestamp}}
"#.to_string()
    }

    // ------------------------------------------------------------------ //
    //  Inductive phase: structured-data → monthly report (Phase 3)       //
    // ------------------------------------------------------------------ //

    /// Generate a monthly report from pre-analysed `StructuredData`.
    ///
    /// This is the inductive step in the de-inductive architecture: the LLM
    /// only receives deterministically clustered evidence and must write prose
    /// that labels and narrates those clusters.  A `RetryLoop` (max 3 tries)
    /// ensures format compliance; the pure-Rust fallback is always available.
    pub async fn synthesize_with_structured_data(
        &self,
        structured: &StructuredData,
        date_range: &str,
        prior_plan_context: Option<&str>,
    ) -> Result<String> {
        let project_name = self.config.project.name.clone();

        // Build the condensed activity text from StructuredData.
        let clusters_text = Self::format_clusters_for_prompt(structured);
        let milestones_text = if structured.milestone_markers.is_empty() {
            String::new()
        } else {
            let lines: Vec<String> = structured
                .milestone_markers
                .iter()
                .map(|m| format!("- [{}] {}", m.status_hint, m.text))
                .collect();
            format!("\nCARRY-FORWARD FROM LAST MONTH:\n{}\n", lines.join("\n"))
        };

        let prior_context = prior_plan_context
            .map(|p| format!("\nPRIOR MONTH PLAN EXCERPT:\n{}\n", &p[..p.len().min(1500)]))
            .unwrap_or_default();

        let metrics = &structured.summary_metrics;
        let repos_str = structured.metadata.repos.join(", ");

        let base_prompt = format!(
            "You are writing a professional monthly engineering report.\n\
             Project: {project}\n\
             Period: {period}\n\
             Repos: {repos}\n\n\
             METRICS:\n\
             - Unique commits: {commits}\n\
             - Lines added/removed: +{adds}/-{dels}\n\
             - Open MRs/PRs: {prs} | Issues: {issues}\n\
             {milestones}{prior}\n\
             COMMIT CLUSTERS (tf-idf analysis - each cluster = a distinct work area):\n\
             {clusters}\n\
             INSTRUCTIONS:\n\
             - Write a COMPLETE monthly report. Use these sections:\n\
               1. Executive Summary (3-5 sentences, key numbers)\n\
               2. Technical Achievements (one subsection per commit cluster; name the \
                  subsection after the cluster's theme)\n\
               3. Progress on Prior Goals (reference CARRY-FORWARD items if present)\n\
               4. Risks / Blockers (if any are evident)\n\
               5. Next Steps\n\
             - Reference actual commit titles and repository names.\n\
             - Do not invent data not present above.\n\
             - Use markdown formatting.",
            project = project_name,
            period = date_range,
            repos = repos_str,
            commits = structured.metadata.total_commits_sanitized,
            adds = metrics.total_additions,
            dels = metrics.total_deletions,
            prs = metrics.total_mrs_and_prs,
            issues = metrics.total_issues,
            milestones = milestones_text,
            prior = prior_context,
            clusters = clusters_text,
        );

        let retry = RetryLoop::new(3);
        let fallback = Self::structured_fallback_report(structured, &project_name, date_range);

        let result = retry
            .run_str(
                |hint| {
                    let prompt = match hint {
                        None => base_prompt.clone(),
                        Some(h) => format!(
                            "{base}\n\n---\nPREVIOUS ATTEMPT FEEDBACK: {hint}\n\
                             Please revise your response to address this feedback.",
                            base = base_prompt,
                            hint = h
                        ),
                    };
                    let ollama = self.ollama_client.as_ref().map(|o| {
                        // Clone the model + url so we can move into the async block.
                        // OllamaClient isn't Clone, so we reconstruct from config fields.
                        (self.config.llm.model.clone(), String::new())
                    });
                    let use_ollama = self.use_ollama;
                    let api_base = std::env::var("OPENAI_API_BASE")
                        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
                    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
                    let model = self.config.llm.model.clone();
                    let client = self.client.clone();
                    async move {
                        if use_ollama {
                            let oc = OllamaClient::new(None, Some(model));
                            oc.generate(&prompt).await
                        } else {
                            // Minimal OpenAI-compatible call
                            let payload = serde_json::json!({
                                "model": model,
                                "messages": [{"role": "user", "content": prompt}],
                                "max_tokens": 2048
                            });
                            let resp: serde_json::Value = client
                                .post(format!("{}/chat/completions", api_base))
                                .bearer_auth(&api_key)
                                .json(&payload)
                                .send()
                                .await?
                                .json()
                                .await?;
                            let text = resp["choices"][0]["message"]["content"]
                                .as_str()
                                .unwrap_or("")
                                .to_string();
                            Ok(text)
                        }
                    }
                },
                validate_min_length(300),
                fallback,
            )
            .await;

        Ok(result)
    }

    /// Format commit clusters into a compact prompt-friendly representation.
    fn format_clusters_for_prompt(structured: &StructuredData) -> String {
        structured
            .commit_clusters
            .iter()
            .enumerate()
            .map(|(i, cluster)| {
                let sample_titles: Vec<&str> = cluster
                    .commits
                    .iter()
                    .take(4)
                    .map(|c| c.title.as_str())
                    .collect();
                let files_summary = if cluster.file_paths.is_empty() {
                    String::new()
                } else {
                    format!("\n  Files: {}", cluster.file_paths.join(", "))
                };
                let stats: u64 = cluster
                    .commits
                    .iter()
                    .map(|c| c.additions + c.deletions)
                    .sum();
                format!(
                    "Cluster {} - terms: [{}] ({} commits, ~{} lines changed)\n\
                     Sample commits:\n{}{}\n",
                    i + 1,
                    cluster.terms.join(", "),
                    cluster.commits.len(),
                    stats,
                    sample_titles
                        .iter()
                        .map(|t| format!("  - {}", t))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    files_summary,
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Pure-Rust fallback report — uses only StructuredData, no LLM.
    fn structured_fallback_report(
        structured: &StructuredData,
        project_name: &str,
        date_range: &str,
    ) -> String {
        let metrics = &structured.summary_metrics;
        let mut out = format!(
            "# Monthly Engineering Report\n\
             **Project:** {project}\n\
             **Period:** {period}\n\n\
             ## Executive Summary\n\n\
             This period saw {commits} commits across {repos} repositories, \
             with {adds} lines added and {dels} lines removed.\n\n\
             ## Technical Activity by Area\n\n",
            project = project_name,
            period = date_range,
            commits = structured.metadata.total_commits_sanitized,
            repos = structured.metadata.total_repos,
            adds = metrics.total_additions,
            dels = metrics.total_deletions,
        );

        for (i, cluster) in structured.commit_clusters.iter().enumerate() {
            out.push_str(&format!(
                "### Area {}: {} ({} commits)\n\n",
                i + 1,
                cluster.terms.first().cloned().unwrap_or_else(|| "misc".to_string()),
                cluster.commits.len()
            ));
            for commit in cluster.commits.iter().take(5) {
                out.push_str(&format!("- {} (`{}`)\n", commit.title, commit.id));
            }
            out.push('\n');
        }

        if !structured.milestone_markers.is_empty() {
            out.push_str("## Prior Goals Status\n\n");
            for m in &structured.milestone_markers {
                out.push_str(&format!("- [{}] {}\n", m.status_hint, m.text));
            }
            out.push('\n');
        }

        out.push_str("## Next Steps\n\n*(Derived from cluster analysis - review and expand)*\n");
        out
    }
}

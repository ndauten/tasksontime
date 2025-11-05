use anyhow::Result;
use crate::types::CollectedData;
use crate::ollama::OllamaClient;
use std::fs;
use tokio::fs as async_fs;
use chrono::Utc;
use reqwest::Client;
use serde_json::json;

pub struct SimplePipeline {
    ollama_client: OllamaClient,
    openai_client: Client,
    verbose: bool,
    debug_dir: String,
    use_chatgpt: bool,
}

impl SimplePipeline {
    pub fn new(verbose: bool) -> Self {
        let ollama_client = OllamaClient::new(None, Some("llama3.1:70b".to_string()));
        let openai_client = Client::new();
        let debug_dir = format!("debug_{}", Utc::now().format("%Y%m%d_%H%M%S"));
        
        // Check if we should use ChatGPT (if OPENAI_API_KEY is available)
        let use_chatgpt = std::env::var("OPENAI_API_KEY").is_ok();
        
        if verbose {
            println!("🔧 Creating simple pipeline with debug directory: {}", debug_dir);
            if use_chatgpt {
                println!("🚀 ChatGPT API key found - using GPT-4 for superior analysis");
            } else {
                println!("🦙 Using local Llama 3.1 70B model");
            }
            if let Err(e) = fs::create_dir_all(&debug_dir) {
                println!("⚠️  Failed to create debug directory: {}", e);
            }
        }
        
        Self {
            ollama_client,
            openai_client,
            verbose,
            debug_dir,
            use_chatgpt,
        }
    }
    
    pub async fn generate_simple_report(&self, data: &CollectedData) -> Result<String> {
        println!("🚀 Starting SIMPLE architectural analysis pipeline...");
        
        // Step 1: Extract raw technical content (NO PREPROCESSING)
        let raw_content = self.extract_raw_content(data);
        
        if self.verbose {
            println!("📊 Raw content extracted: {} characters", raw_content.len());
            self.save_debug_file("01_raw_content.txt", &raw_content).await;
        }
        
        // Step 2: Load sophisticated prompts
        let prompts = self.load_sophisticated_prompts()?;
        
        // Step 3: Check content size and determine if we need chunking
        let estimated_tokens = self.estimate_tokens(&raw_content);
        let max_tokens = 128_000; // Llama 3.1 8B has 128K context window
        
        if estimated_tokens > max_tokens {
            println!("⚠️  Content too large ({} tokens > {} limit)", estimated_tokens, max_tokens);
            println!("� Using weekly chunking strategy for large content...");
            return self.generate_chunked_report(data, &prompts).await;
        }
        
        // Step 4: Generate report directly with sophisticated prompts
        let final_prompt = format!(
            "{}\n\n{}\n\nDEVELOPMENT ACTIVITY DATA:\n{}\n\nProvide a comprehensive architectural analysis following the structure and depth outlined in the prompt above. Focus on concrete technical insights, architectural patterns, and strategic recommendations based on the actual code changes and development activity shown in the data.",
            prompts.base_context,
            prompts.executive_summary_prompt,
            raw_content
        );
        
        if self.verbose {
            println!("📝 Final prompt: {} characters", final_prompt.len());
            self.save_debug_file("02_final_prompt.txt", &final_prompt).await;
        }
        
        // Step 5: Send to LLM
        let response = self.call_llm(&final_prompt).await?;
        
        if self.verbose {
            println!("✅ LLM response: {} characters", response.len());
            self.save_debug_file("03_llm_response.txt", &response).await;
        }
        
        // Step 6: Add metadata
        let final_report = format!(
            "{}\n\n---\n\n## Report Metadata\n\n- **Analysis Method**: Simple Direct Pipeline\n- **Generated**: {}\n- **Content Size**: {} characters\n- **Estimated Tokens**: {}\n",
            response,
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            raw_content.len(),
            estimated_tokens
        );
        
        if self.verbose {
            self.save_debug_file("04_final_report.txt", &final_report).await;
            println!("🎉 Debug files saved to: {}", self.debug_dir);
        }
        
        Ok(final_report)
    }
    
    pub async fn generate_report(&self, data: &CollectedData, output: Option<&str>) -> Result<String> {
        // Generate the report content
        let report_content = self.generate_simple_report(data).await?;
        
        // Determine output path
        let output_path = output.unwrap_or("simple_architectural_analysis.md");
        
        // Write to file
        async_fs::write(&output_path, &report_content).await?;
        
        if self.verbose {
            println!("📝 Report written to: {}", output_path);
        }
        
        Ok(output_path.to_string())
    }
    
    fn extract_raw_content(&self, data: &CollectedData) -> String {
        let mut content = String::new();
        
        // Extract ALL commit data as-is
        content.push_str("# GitLab Commits\n\n");
        for commit in &data.gitlab.commits {
            content.push_str(&format!("## Commit\n"));
            content.push_str(&format!("Raw JSON: {}\n\n", serde_json::to_string_pretty(commit).unwrap_or_default()));
        }
        
        content.push_str("# GitHub Commits\n\n");
        for commit in &data.github.commits {
            content.push_str(&format!("## Commit\n"));
            content.push_str(&format!("Raw JSON: {}\n\n", serde_json::to_string_pretty(commit).unwrap_or_default()));
        }
        
        content.push_str("# Local Git Commits\n\n");
        for commit in &data.git_commits {
            content.push_str(&format!("## Commit: {}\n", commit.message));
            content.push_str(&format!("Hash: {}\n", commit.hash));
            content.push_str(&format!("Author: {} <{}>\n", commit.author_name, commit.author_email));
            content.push_str(&format!("Date: {}\n", commit.timestamp));
            content.push_str(&format!("Files: {:?}\n\n", commit.files_changed));
        }
        
        content
    }
    
    fn load_sophisticated_prompts(&self) -> Result<SophisticatedPrompts> {
        let toml_content = fs::read_to_string("prompts/architectural-analysis.toml")?;
        let config: toml::Value = toml::from_str(&toml_content)?;
        
        let base_context = config
            .get("architectural_analysis")
            .and_then(|section| section.get("base_context"))
            .and_then(|v| v.as_str())
            .unwrap_or("You are an expert software architect.")
            .to_string();
            
        let technical_foundation_prompt = config
            .get("architectural_analysis")
            .and_then(|section| section.get("technical_foundation_prompt"))
            .and_then(|v| v.as_str())
            .unwrap_or("Analyze the technical foundation.")
            .to_string();

        let codebase_evolution_prompt = config
            .get("architectural_analysis")
            .and_then(|section| section.get("codebase_evolution_prompt"))
            .and_then(|v| v.as_str())
            .unwrap_or("Analyze the codebase evolution.")
            .to_string();

        let architecture_patterns_prompt = config
            .get("architectural_analysis")
            .and_then(|section| section.get("architecture_patterns_prompt"))
            .and_then(|v| v.as_str())
            .unwrap_or("Analyze architectural patterns.")
            .to_string();

        let executive_summary_prompt = config
            .get("architectural_analysis")
            .and_then(|section| section.get("executive_summary_prompt"))
            .and_then(|v| v.as_str())
            .unwrap_or("Provide executive summary.")
            .to_string();
        
        if self.verbose {
            println!("📋 Loaded sophisticated prompts:");
            println!("  - Base context: {} chars", base_context.len());
            println!("  - Technical foundation: {} chars", technical_foundation_prompt.len());
            println!("  - Codebase evolution: {} chars", codebase_evolution_prompt.len());
            println!("  - Architecture patterns: {} chars", architecture_patterns_prompt.len());
            println!("  - Executive summary: {} chars", executive_summary_prompt.len());
        }
        
        Ok(SophisticatedPrompts {
            base_context,
            technical_foundation_prompt,
            codebase_evolution_prompt,
            architecture_patterns_prompt,
            executive_summary_prompt,
        })
    }
    
    async fn call_llm(&self, prompt: &str) -> Result<String> {
        if self.use_chatgpt {
            println!("🤖 Calling ChatGPT GPT-4 for superior analysis...");
            self.call_openai_api(prompt).await
        } else {
            println!("🤖 Calling Llama 3.1 70B (128K context)...");
            let response = self.ollama_client.generate(prompt).await?;
            println!("✅ LLM call successful");
            Ok(response)
        }
    }
    
    async fn call_openai_api(&self, prompt: &str) -> Result<String> {
        let api_key = std::env::var("OPENAI_API_KEY")?;
        
        let payload = json!({
            "model": "gpt-4-turbo",
            "messages": [
                {
                    "role": "system",
                    "content": "You are an expert software architect and technical analyst. Provide deep, insightful analysis of software development activity with concrete technical insights, architectural patterns, and strategic recommendations."
                },
                {
                    "role": "user", 
                    "content": prompt
                }
            ],
            "max_tokens": 4000,
            "temperature": 0.3
        });
        
        let response = self.openai_client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("OpenAI API error: {}", error_text));
        }
        
        let response_json: serde_json::Value = response.json().await?;
        
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No content in OpenAI response"))?;
            
        println!("✅ ChatGPT call successful");
        Ok(content.to_string())
    }
    
    async fn save_debug_file(&self, filename: &str, content: &str) {
        let filepath = format!("{}/{}", self.debug_dir, filename);
        if let Err(e) = async_fs::write(&filepath, content).await {
            println!("⚠️  Failed to save debug file {}: {}", filepath, e);
        } else {
            println!("💾 Debug file saved: {}", filepath);
        }
    }
    
    fn estimate_tokens(&self, text: &str) -> usize {
        // Simple token estimation: ~3.5 chars per token for code/structured data
        text.len() / 3
    }
    
    async fn generate_chunked_report(&self, data: &CollectedData, prompts: &SophisticatedPrompts) -> Result<String> {
        println!("📅 Starting weekly chunk analysis...");
        
        // Step 1: Group data by week
        let weekly_chunks = self.group_data_by_week(data);
        
        if self.verbose {
            println!("📊 Created {} weekly chunks", weekly_chunks.len());
        }
        
        // Step 2: Analyze each week separately
        let mut weekly_analyses = Vec::new();
        
        for (week_num, (week_start, week_data)) in weekly_chunks.iter().enumerate() {
            println!("🔍 Analyzing week {} ({})...", week_num + 1, week_start.format("%Y-%m-%d"));
            
            // Get raw JSON data for this week - NO PREPROCESSING
            let raw_week_json = serde_json::to_string_pretty(week_data)?;
            let week_tokens = self.estimate_tokens(&raw_week_json);
            
            if self.verbose {
                println!("  📊 Week {} raw data: {} chars, ~{} tokens", week_num + 1, raw_week_json.len(), week_tokens);
            }
            
            // Save raw data for debugging
            self.save_debug_file(&format!("week_{}_raw_data.json", week_start.format("%Y%m%d")), &raw_week_json).await;
            
            // If a single week is still too large, skip it with a summary
            if week_tokens > 25_000 {
                println!("  ⚠️  Week {} too large ({} tokens), creating summary", week_num + 1, week_tokens);
                let summary = self.create_week_summary(week_data, week_start);
                weekly_analyses.push(summary);
                continue;
            }
            
            // Analyze this week with raw data
            let week_analysis = self.analyze_week_chunk(&raw_week_json, week_start, prompts).await?;
            weekly_analyses.push(week_analysis);
            
            // Small delay between weeks
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        
        // Step 3: Synthesize weekly analyses into final report
        println!("🧠 Synthesizing {} weekly analyses into final report...", weekly_analyses.len());
        let final_report = self.synthesize_weekly_analyses(&weekly_analyses, data, prompts).await?;
        
        if self.verbose {
            self.save_debug_file("final_synthesized_report.txt", &final_report).await;
        }
        
        Ok(final_report)
    }

    fn group_data_by_week(&self, data: &CollectedData) -> Vec<(chrono::NaiveDate, CollectedData)> {
        use chrono::Duration;
        
        
        let mut weekly_chunks = Vec::new();
        let start_date = data.metadata.date_range_start.date_naive();
        let end_date = data.metadata.date_range_end.date_naive();
        
        // Create weekly buckets
        let mut current_date = start_date;
        while current_date <= end_date {
            let week_end = current_date + Duration::days(6);
            let week_end = if week_end > end_date { end_date } else { week_end };
            
            // Create filtered data for this week
            let mut week_data = CollectedData {
                metadata: data.metadata.clone(),
                gitlab: crate::types::GitLabData {
                    commits: Vec::new(),
                    issues: Vec::new(),
                    merge_requests: Vec::new(),
                    comments: Vec::new(),
                },
                github: crate::types::GitHubData {
                    commits: Vec::new(),
                    issues: Vec::new(),
                    pull_requests: Vec::new(),
                    comments: Vec::new(),
                },
                git_commits: Vec::new(),
                local_files: Vec::new(),
            };
            
            // Filter GitLab commits for this week
            for commit in &data.gitlab.commits {
                if let Some(date_str) = commit.get("created_at").and_then(|v| v.as_str()) {
                    if let Ok(commit_date) = chrono::DateTime::parse_from_rfc3339(date_str) {
                        let commit_date = commit_date.date_naive();
                        if commit_date >= current_date && commit_date <= week_end {
                            week_data.gitlab.commits.push(commit.clone());
                        }
                    }
                }
            }
            
            // Filter GitHub commits for this week
            for commit in &data.github.commits {
                if let Some(date_str) = commit.get("created_at").and_then(|v| v.as_str()) {
                    if let Ok(commit_date) = chrono::DateTime::parse_from_rfc3339(date_str) {
                        let commit_date = commit_date.date_naive();
                        if commit_date >= current_date && commit_date <= week_end {
                            week_data.github.commits.push(commit.clone());
                        }
                    }
                }
            }
            
            // Filter local git commits for this week
            for commit in &data.git_commits {
                let commit_date = commit.timestamp.date_naive();
                if commit_date >= current_date && commit_date <= week_end {
                    week_data.git_commits.push(commit.clone());
                }
            }
            
            // Filter other data types similarly (issues, MRs, etc.)
            for issue in &data.gitlab.issues {
                if let Some(date_str) = issue.get("created_at").and_then(|v| v.as_str()) {
                    if let Ok(issue_date) = chrono::DateTime::parse_from_rfc3339(date_str) {
                        let issue_date = issue_date.date_naive();
                        if issue_date >= current_date && issue_date <= week_end {
                            week_data.gitlab.issues.push(issue.clone());
                        }
                    }
                }
            }
            
            for mr in &data.gitlab.merge_requests {
                if let Some(date_str) = mr.get("created_at").and_then(|v| v.as_str()) {
                    if let Ok(mr_date) = chrono::DateTime::parse_from_rfc3339(date_str) {
                        let mr_date = mr_date.date_naive();
                        if mr_date >= current_date && mr_date <= week_end {
                            week_data.gitlab.merge_requests.push(mr.clone());
                        }
                    }
                }
            }
            
            // Only add weeks that have some data
            if week_data.total_items() > 0 {
                weekly_chunks.push((current_date, week_data));
            }
            
            current_date = week_end + Duration::days(1);
        }
        
        weekly_chunks
    }

    fn create_week_summary(&self, week_data: &CollectedData, week_start: &chrono::NaiveDate) -> String {
        format!(
            "## Week of {}\n\n\
            **Summary**: Week too large for detailed analysis\n\
            - GitLab Commits: {}\n\
            - GitHub Commits: {}\n\
            - Local Git Commits: {}\n\
            - GitLab Issues: {}\n\
            - GitLab MRs: {}\n\
            - Total Items: {}\n\n\
            *Note: This week had too much activity for detailed analysis. Consider running analysis on smaller date ranges.*\n",
            week_start.format("%Y-%m-%d"),
            week_data.gitlab.commits.len(),
            week_data.github.commits.len(),
            week_data.git_commits.len(),
            week_data.gitlab.issues.len(),
            week_data.gitlab.merge_requests.len(),
            week_data.total_items()
        )
    }

    async fn analyze_week_chunk(&self, week_content: &str, week_start: &chrono::NaiveDate, prompts: &SophisticatedPrompts) -> Result<String> {
        // Create sophisticated prompt using the actual TOML prompts
        let prompt = format!(
            "{}\n\n{}\n\nWEEK OF {} DEVELOPMENT ACTIVITY:\n\n{}\n\nAnalyze this week's development activity following the detailed analysis framework above. Focus on specific technical changes, architectural decisions, and development patterns evident in the data.",
            prompts.base_context,
            prompts.codebase_evolution_prompt,
            week_start.format("%Y-%m-%d"),
            week_content
        );
        
        // Debug output
        if self.verbose {
            println!("📝 Week {} prompt: {} chars", week_start.format("%Y-%m-%d"), prompt.len());
        }
        self.save_debug_file(&format!("week_{}_prompt.txt", week_start.format("%Y%m%d")), &prompt).await;
        
        // Send to LLM
        let response = self.call_llm(&prompt).await?;
        
        // Debug output
        if self.verbose {
            println!("� Week {} response: {} chars", week_start.format("%Y-%m-%d"), response.len());
        }
        self.save_debug_file(&format!("week_{}_response.txt", week_start.format("%Y%m%d")), &response).await;
        
        Ok(response)
    }

    async fn synthesize_weekly_analyses(&self, weekly_analyses: &[String], data: &CollectedData, prompts: &SophisticatedPrompts) -> Result<String> {
        let combined_analyses = weekly_analyses.join("\n\n---\n\n");
        
        // Simple synthesis prompt: base_context + executive_summary_prompt + weekly results
        let synthesis_prompt = format!(
            "{}\n\n{}\n\n**WEEKLY ANALYSES TO SYNTHESIZE**:\n\n{}\n\n**SYNTHESIZE THE ABOVE ANALYSES**",
            prompts.base_context,
            prompts.executive_summary_prompt,
            combined_analyses
        );
        
        // Debug output
        if self.verbose {
            println!("� Synthesis prompt: {} chars", synthesis_prompt.len());
        }
        self.save_debug_file("synthesis_prompt.txt", &synthesis_prompt).await;
        
        // Send to LLM
        let response = self.call_llm(&synthesis_prompt).await?;
        
        // Debug output  
        if self.verbose {
            println!("📝 Synthesis response: {} chars", response.len());
        }
        self.save_debug_file("synthesis_response.txt", &response).await;
        
        // Add simple metadata
        let final_report = format!(
            "{}\n\n---\n\n**Report Metadata**\n- Analysis Period: {} to {}\n- Weeks: {}\n- Items: {}\n- Generated: {}\n",
            response,
            data.metadata.date_range_start.format("%Y-%m-%d"),
            data.metadata.date_range_end.format("%Y-%m-%d"),
            weekly_analyses.len(),
            data.total_items(),
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );
        
        Ok(final_report)
    }

    // ...existing code...
}

struct SophisticatedPrompts {
    base_context: String,
    #[allow(dead_code)]
    technical_foundation_prompt: String,
    codebase_evolution_prompt: String,
    #[allow(dead_code)]
    architecture_patterns_prompt: String,
    executive_summary_prompt: String,
}

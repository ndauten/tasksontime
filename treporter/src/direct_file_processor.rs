use anyhow::Result;
use std::fs;
use tokio::fs as async_fs;
use chrono::Utc;
use reqwest::Client;
use serde_json::json;
use crate::ollama::OllamaClient;

pub struct DirectFileProcessor {
    ollama_client: OllamaClient,
    openai_client: Client,
    use_chatgpt: bool,
    verbose: bool,
}

impl DirectFileProcessor {
    pub fn new(verbose: bool) -> Self {
        let ollama_client = OllamaClient::new(None, Some("llama3.1:70b".to_string()));
        let openai_client = Client::new();
        let use_chatgpt = false; // Default to Ollama
        
        if verbose {
            println!("🦙 Using Llama 3.1 70B for direct file analysis");
        }
        
        Self {
            ollama_client,
            openai_client,
            use_chatgpt,
            verbose,
        }
    }
    
    pub async fn process_file(&self, file_path: &str, output_path: Option<&str>) -> Result<String> {
        // Read the file directly
        let file_content = fs::read_to_string(file_path)?;
        
        if self.verbose {
            println!("📄 Read file: {} ({} chars)", file_path, file_content.len());
        }
        
        // Read prompt file directly as string - NO PARSING!
        let prompt_content = fs::read_to_string("prompts/architectural-analysis.toml")?;
        
        if self.verbose {
            println!("📋 Read prompt file: {} chars", prompt_content.len());
        }
        
        // Create direct prompt: raw prompt file + file content
        let direct_prompt = format!(
            "{}\n\nFILE TO ANALYZE:\n\n{}\n\nAnalyze this file according to the framework above.",
            prompt_content,
            file_content
        );
        
        if self.verbose {
            println!("📝 Direct prompt created: {} chars", direct_prompt.len());
            println!("🔍 DEBUG - First 500 chars of prompt:");
            println!("{}", &direct_prompt[..500.min(direct_prompt.len())]);
        }
        
        // Send directly to LLM
        let response = self.call_llm(&direct_prompt).await?;
        
        if self.verbose {
            println!("✅ LLM response: {} chars", response.len());
        }
        
        // Add minimal metadata
        let final_report = format!(
            "{}\n\n---\n\n**Analysis Metadata**\n- Source File: {}\n- Generated: {}\n- Model: {}\n",
            response,
            file_path,
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            if self.use_chatgpt { "ChatGPT GPT-4" } else { "Llama 3.1 70B" }
        );
        
        // Write to output file
        let output_file = output_path.unwrap_or("direct_analysis.md");
        async_fs::write(&output_file, &final_report).await?;
        
        if self.verbose {
            println!("📝 Analysis written to: {}", output_file);
        }
        
        Ok(output_file.to_string())
    }
    
    async fn call_llm(&self, prompt: &str) -> Result<String> {
        if self.use_chatgpt {
            self.call_chatgpt(prompt).await
        } else {
            self.call_ollama(prompt).await
        }
    }
    
    async fn call_chatgpt(&self, prompt: &str) -> Result<String> {
        let api_key = std::env::var("OPENAI_API_KEY")?;
        
        let payload = json!({
            "model": "gpt-4-turbo",
            "messages": [
                {
                    "role": "system",
                    "content": "You are an expert software architect and technical analyst. Provide deep, insightful analysis with concrete technical insights, architectural patterns, and strategic recommendations."
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
            
        println!("✅ ChatGPT analysis complete");
        Ok(content.to_string())
    }
    
    async fn call_ollama(&self, prompt: &str) -> Result<String> {
        let response = self.ollama_client.generate(prompt).await?;
        println!("✅ Ollama analysis complete");
        Ok(response)
    }
}

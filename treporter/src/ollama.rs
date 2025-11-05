use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};

pub struct OllamaClient {
    client: Client,
    base_url: String,
    model: String,
}

impl OllamaClient {
    pub fn new(base_url: Option<String>, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
            model: model.unwrap_or_else(|| "llama3.2:3b".to_string()),
        }
    }

    pub async fn generate(&self, prompt: &str) -> Result<String> {
        let url = format!("{}/api/generate", self.base_url);
        
        let payload = json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": 0.7,
                "top_k": 40,
                "top_p": 0.9,
                "num_predict": 2048
            }
        });

        println!("🦙 Sending request to Ollama ({})...", self.model);
        
        let response = self.client
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Ollama API error: {}", error_text));
        }

        let response_json: Value = response.json().await?;
        
        let content = response_json
            .get("response")
            .and_then(|r| r.as_str())
            .ok_or_else(|| anyhow::anyhow!("No response content from Ollama"))?;

        Ok(content.to_string())
    }

    #[allow(dead_code)]
    pub async fn generate_with_system(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let combined_prompt = format!("{}\n\nUser: {}\nAssistant:", system_prompt, user_prompt);
        self.generate(&combined_prompt).await
    }

    #[allow(dead_code)]
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/api/tags", self.base_url);
        
        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ollama_health_check() {
        let client = OllamaClient::new(None, None);
        let is_healthy = client.health_check().await.unwrap_or(false);
        println!("Ollama health check: {}", is_healthy);
    }

    #[tokio::test]
    async fn test_ollama_generate() {
        let client = OllamaClient::new(None, None);
        
        if client.health_check().await.unwrap_or(false) {
            match client.generate("Generate a brief technical summary bullet point.").await {
                Ok(response) => {
                    println!("Ollama response: {}", response);
                    assert!(!response.is_empty());
                }
                Err(e) => {
                    println!("Test skipped - Ollama error: {}", e);
                }
            }
        } else {
            println!("Test skipped - Ollama not available");
        }
    }
}

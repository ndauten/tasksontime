use crate::config::GitLabConfig;
use crate::types::GitLabEvent;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use serde_json::Value;
use std::env;

pub struct GitLabCollector {
    client: Client,
    token: String,
    username: String,
    base_url: String,
    config: GitLabConfig,
}

impl GitLabCollector {
    pub fn new(config: &GitLabConfig) -> Result<Self> {
        let token = env::var(&config.token_env)
            .map_err(|_| anyhow!("GitLab token not found in environment variable: {}", config.token_env))?;
        
        let username = env::var(&config.username_env)
            .map_err(|_| anyhow!("GitLab username not found in environment variable: {}", config.username_env))?;

        Ok(Self {
            client: Client::new(),
            token,
            username,
            base_url: config.base_url.clone(),
            config: config.clone(),
        })
    }

    pub fn collect(&self, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<GitLabEvent>> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }

        println!("🔍 Collecting GitLab events from {} to {}", start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d"));
        
        let user_id = self.get_user_id()?;
        let events = self.get_user_events(user_id, start_date, end_date)?;
        
        println!("✅ Collected {} GitLab events", events.len());
        Ok(events)
    }

    fn get_user_id(&self) -> Result<u64> {
        let url = format!("{}/api/v4/users?username={}", self.base_url, self.username);
        let resp = self.client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()?
            .json::<Vec<Value>>()?;
        
        resp.first()
            .and_then(|user| user["id"].as_u64())
            .ok_or_else(|| anyhow!("User not found: {}", self.username))
    }

    fn get_user_events(&self, user_id: u64, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<GitLabEvent>> {
        let mut events = Vec::new();
        let mut page = 1;
        let since = start_date.format("%Y-%m-%d").to_string();

        loop {
            let url = format!(
                "{}/api/v4/users/{}/events?page={}&per_page=100&after={}",
                self.base_url, user_id, page, since
            );
            
            let resp = self.client
                .get(&url)
                .header("PRIVATE-TOKEN", &self.token)
                .send()?
                .json::<Vec<Value>>()?;

            if resp.is_empty() {
                break;
            }

            for event_json in resp {
                if let Ok(event) = self.parse_event(event_json) {
                    // Filter by date range
                    if event.created_at >= start_date && event.created_at <= end_date {
                        events.push(event);
                    }
                }
            }

            page += 1;
            if page > 10 { // Safety limit
                break;
            }
        }

        Ok(events)
    }

    fn parse_event(&self, event_json: Value) -> Result<GitLabEvent> {
        let created_at_str = event_json["created_at"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing created_at field"))?;
        
        let created_at = DateTime::parse_from_rfc3339(created_at_str)?
            .with_timezone(&Utc);

        let event = GitLabEvent {
            id: event_json["id"].as_u64().unwrap_or(0),
            action_name: event_json["action_name"].as_str().unwrap_or("").to_string(),
            target_type: event_json["target_type"].as_str().map(|s| s.to_string()),
            target_title: event_json["target_title"].as_str().map(|s| s.to_string()),
            project_id: event_json["project_id"].as_u64(),
            project_name: event_json["project"]["name"].as_str().map(|s| s.to_string()),
            created_at,
            details: self.get_event_details(&event_json),
            author_name: event_json["author"]["name"].as_str().map(|s| s.to_string()),
        };

        Ok(event)
    }

    fn get_event_details(&self, event_json: &Value) -> Option<String> {
        let action = event_json["action_name"].as_str()?;
        let target_type = event_json["target_type"].as_str()?;
        let target_title = event_json["target_title"].as_str().unwrap_or("Untitled");

        match (action, target_type) {
            ("created", "MergeRequest") => Some(format!("Created merge request: {}", target_title)),
            ("merged", "MergeRequest") => Some(format!("Merged merge request: {}", target_title)),
            ("opened", "Issue") => Some(format!("Opened issue: {}", target_title)),
            ("closed", "Issue") => Some(format!("Closed issue: {}", target_title)),
            ("pushed to", _) => Some(format!("Pushed commits to repository")),
            _ => Some(format!("{} {}: {}", action, target_type, target_title)),
        }
    }
}

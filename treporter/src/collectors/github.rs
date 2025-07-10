use crate::config::GitHubConfig;
use crate::types::GitHubEvent;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use serde_json::Value;
use std::env;

pub struct GitHubCollector {
    client: Client,
    token: String,
    username: String,
    config: GitHubConfig,
}

impl GitHubCollector {
    pub fn new(config: &GitHubConfig) -> Result<Self> {
        let token = env::var(&config.token_env)
            .map_err(|_| anyhow!("GitHub token not found in environment variable: {}", config.token_env))?;
        
        let username = env::var(&config.username_env)
            .map_err(|_| anyhow!("GitHub username not found in environment variable: {}", config.username_env))?;

        Ok(Self {
            client: Client::new(),
            token,
            username,
            config: config.clone(),
        })
    }

    pub fn collect(&self, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<GitHubEvent>> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }

        println!("🔍 Collecting GitHub events from {} to {}", start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d"));
        
        let events = self.get_user_events(start_date, end_date)?;
        
        println!("✅ Collected {} GitHub events", events.len());
        Ok(events)
    }

    fn get_user_events(&self, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<GitHubEvent>> {
        let mut events = Vec::new();
        let mut page = 1;

        loop {
            let url = format!("https://api.github.com/users/{}/events?page={}&per_page=100", self.username, page);
            
            let resp = self.client
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.token))
                .header("User-Agent", "treporter/1.0")
                .header("Accept", "application/vnd.github+json")
                .send()?;

            if resp.status() == reqwest::StatusCode::FORBIDDEN {
                println!("⚠️  GitHub API rate limit reached");
                break;
            }

            let event_list = resp.json::<Vec<Value>>()?;

            if event_list.is_empty() {
                break;
            }

            for event_json in event_list {
                if let Ok(event) = self.parse_event(event_json) {
                    // Filter by date range
                    if event.created_at >= start_date && event.created_at <= end_date {
                        events.push(event);
                    } else if event.created_at < start_date {
                        // Events are returned in chronological order (newest first)
                        // If we hit an event older than our start date, we can stop
                        return Ok(events);
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

    fn parse_event(&self, event_json: Value) -> Result<GitHubEvent> {
        let created_at_str = event_json["created_at"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing created_at field"))?;
        
        let created_at = DateTime::parse_from_rfc3339(created_at_str)?
            .with_timezone(&Utc);

        let event_type = event_json["type"].as_str().unwrap_or("").to_string();
        let repo_name = event_json["repo"]["name"].as_str().map(|s| s.to_string());
        let actor = event_json["actor"]["login"].as_str().map(|s| s.to_string());

        let event = GitHubEvent {
            id: event_json["id"].as_str().unwrap_or("").to_string(),
            event_type: event_type.clone(),
            repo_name: repo_name.clone(),
            created_at,
            actor,
            payload: Some(event_json["payload"].clone()),
            details: self.get_event_details(&event_type, &event_json["payload"], &repo_name),
        };

        Ok(event)
    }

    fn get_event_details(&self, event_type: &str, payload: &Value, repo_name: &Option<String>) -> Option<String> {
        let repo = repo_name.as_deref().unwrap_or("unknown");
        
        match event_type {
            "PushEvent" => {
                let commit_count = payload["size"].as_u64().unwrap_or(0);
                Some(format!("Pushed {} commit(s) to {}", commit_count, repo))
            },
            "CreateEvent" => {
                let ref_type = payload["ref_type"].as_str().unwrap_or("unknown");
                let ref_name = payload["ref"].as_str().unwrap_or("");
                Some(format!("Created {} {} in {}", ref_type, ref_name, repo))
            },
            "IssuesEvent" => {
                let action = payload["action"].as_str().unwrap_or("unknown");
                let issue_title = payload["issue"]["title"].as_str().unwrap_or("Untitled");
                Some(format!("{} issue '{}' in {}", action, issue_title, repo))
            },
            "PullRequestEvent" => {
                let action = payload["action"].as_str().unwrap_or("unknown");
                let pr_title = payload["pull_request"]["title"].as_str().unwrap_or("Untitled");
                Some(format!("{} pull request '{}' in {}", action, pr_title, repo))
            },
            "IssueCommentEvent" => {
                let issue_title = payload["issue"]["title"].as_str().unwrap_or("Untitled");
                Some(format!("Commented on issue '{}' in {}", issue_title, repo))
            },
            "ForkEvent" => {
                Some(format!("Forked repository {}", repo))
            },
            "WatchEvent" => {
                Some(format!("Starred repository {}", repo))
            },
            _ => Some(format!("{} in {}", event_type, repo)),
        }
    }
}

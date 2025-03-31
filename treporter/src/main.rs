use chrono::{Duration, Utc};
use reqwest::blocking::Client;
use serde_json::{Map, Value};
use std::{collections::HashMap, env, fs::File, io::Write, process};

fn load_env_or_exit() -> (String, String) {
    dotenvy::dotenv().ok(); // Loads from .env file if present

    let token = env::var("GITLAB_TOKEN").unwrap_or_else(|_| {
        eprintln!("❌ Error: GITLAB_TOKEN not found in .env or environment.");
        process::exit(1);
    });

    let username = env::var("GITLAB_USERNAME").unwrap_or_else(|_| {
        eprintln!("❌ Error: GITLAB_USERNAME not found in .env or environment.");
        process::exit(1);
    });

    (token, username)
}

const GITLAB_URL: &str = "https://gitlab.com";
const DAYS_BACK: i64 = 30;

fn get_user_id(client: &Client, token: &str, username: &str) -> Result<u64, reqwest::Error> {
    let url = format!("{}/api/v4/users?username={}", GITLAB_URL, username);
    let resp = client
        .get(&url)
        .header("PRIVATE-TOKEN", token)
        .send()?
        .json::<Vec<Value>>()?;
    Ok(resp[0]["id"].as_u64().unwrap())
}

fn get_user_events(client: &Client, token: &str, user_id: u64, since: &str) -> Result<Vec<Value>, reqwest::Error> {
    let mut events = vec![];
    let mut page = 1;

    loop {
        let url = format!(
            "{}/api/v4/users/{}/events?page={}&per_page=100&after={}",
            GITLAB_URL, user_id, page, since
        );
        let resp = client
            .get(&url)
            .header("PRIVATE-TOKEN", token)
            .send()?
            .json::<Vec<Value>>()?;

        if resp.is_empty() {
            break;
        }

        events.extend(resp);
        page += 1;
    }

    println!("Fetched {} events for user ID {}", events.len(), user_id);
    Ok(events)
}

fn populate_event_details(client: &Client, token: &str, event: &mut Value) -> Result<(), reqwest::Error> {
    if let Some(project_id) = event["project_id"].as_u64() {
        if let Some(target_type) = event["target_type"].as_str() {
            match target_type {
                "Commit" => {
                    let url = format!(
                        "{}/api/v4/projects/{}/repository/commits/{}",
                        GITLAB_URL,
                        project_id,
                        event["action_name"].as_str().unwrap_or("")
                    );
                    let resp = client
                        .get(&url)
                        .header("PRIVATE-TOKEN", token)
                        .send()?
                        .json::<Value>()?;
                    event["details"] = Value::String(format!(
                        "Commit: {} by {}",
                        resp["message"].as_str().unwrap_or(""),
                        resp["author_name"].as_str().unwrap_or("")
                    ));
                }
                "MergeRequest" => {
                    let url = format!(
                        "{}/api/v4/projects/{}/merge_requests/{}",
                        GITLAB_URL,
                        project_id,
                        event["action_name"].as_str().unwrap_or("")
                    );
                    let resp = client
                        .get(&url)
                        .header("PRIVATE-TOKEN", token)
                        .send()?
                        .json::<Value>()?;
                    event["details"] = Value::String(format!(
                        "Merge Request: {} by {}",
                        resp["title"].as_str().unwrap_or(""),
                        resp["author"]["name"].as_str().unwrap_or("")
                    ));
                }
                "Issue" => {
                    let url = format!(
                        "{}/api/v4/projects/{}/issues/{}",
                        GITLAB_URL,
                        project_id,
                        event["action_name"].as_str().unwrap_or("")
                    );
                    let resp = client
                        .get(&url)
                        .header("PRIVATE-TOKEN", token)
                        .send()?
                        .json::<Value>()?;
                    event["details"] = Value::String(format!(
                        "Issue: {} by {}",
                        resp["title"].as_str().unwrap_or(""),
                        resp["author"]["name"].as_str().unwrap_or("")
                    ));
                }
                _ => {
                    event["details"] = Value::String("No additional details available".to_string());
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (token, username) = load_env_or_exit();
    let client = Client::new();
    let since = (Utc::now() - Duration::days(DAYS_BACK)).date_naive().to_string();

    let user_id = get_user_id(&client, &token, &username)?;
    let mut events = get_user_events(&client, &token, user_id, &since)?;

    for event in &mut events {
        populate_event_details(&client, &token, event)?;
    }

    // Serialize events to JSON and save to a file
    let file_path = "gitlab_activity.json";
    let mut file = File::create(file_path)?;
    let json_data = serde_json::to_string_pretty(&events)?;
    file.write_all(json_data.as_bytes())?;

    println!("✅ Saved {} events to {}", events.len(), file_path);
    Ok(())
}
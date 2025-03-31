use chrono::{Duration, Utc};
use reqwest::blocking::Client;
use serde::Deserialize;
use std::{env, fs::File, process};

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

#[derive(Debug, Deserialize, serde::Serialize)]
struct Event {
    action_name: String,
    project_id: Option<u64>,
    target_type: Option<String>,
    created_at: String,
}

const GITLAB_URL: &str = "https://gitlab.com";
const DAYS_BACK: i64 = 90;

fn get_user_id(client: &Client, token: &str, username: &str) -> Result<u64, reqwest::Error> {
    let url = format!("{}/api/v4/users?username={}", GITLAB_URL, username);
    let resp = client
        .get(&url)
        .header("PRIVATE-TOKEN", token)
        .send()?
        .json::<Vec<serde_json::Value>>()?;
    Ok(resp[0]["id"].as_u64().unwrap())
}

fn get_user_events(client: &Client, token: &str, user_id: u64, since: &str) -> Result<Vec<Event>, reqwest::Error> {
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
            .json::<Vec<Event>>()?;

        if resp.is_empty() {
            break;
        }

        events.extend(resp);
        page += 1;
    }

    Ok(events)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (token, username) = load_env_or_exit();
    let client = Client::new();
    let since = (Utc::now() - Duration::days(DAYS_BACK)).date_naive().to_string();

    let user_id = get_user_id(&client, &token, &username)?;
    let events = get_user_events(&client, &token, user_id, &since)?;

    let mut wtr = csv::Writer::from_writer(File::create("gitlab_activity.csv")?);
    for event in &events {
        wtr.serialize(event)?;
    }

    println!("✅ Saved {} events to gitlab_activity.csv", events.len());
    Ok(())
}


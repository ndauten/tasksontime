use chrono::{Datelike, NaiveDate, Utc};
use reqwest::blocking::Client;
use serde_json::Value;
use std::{env, fs::File, io::Write, process};

fn load_env_or_exit() -> (String, String, String, String) {
    dotenvy::dotenv().ok(); // Loads from .env file if present

    let gitlab_token = env::var("GITLAB_TOKEN").unwrap_or_else(|_| {
        eprintln!("❌ Error: GITLAB_TOKEN not found in .env or environment.");
        print_help();
        process::exit(1);
    });
    
    let gitlab_username = env::var("GITLAB_USERNAME").unwrap_or_else(|_| {
        eprintln!("❌ Error: USERNAME not found in .env or environment.");
        print_help();
        process::exit(1);
    });

    let github_token = env::var("GITHUB_TOKEN").unwrap_or_else(|_| {
        eprintln!("❌ Error: GITHUB_TOKEN not found in .env or environment.");
        print_help();
        process::exit(1);
    });

    let github_username = env::var("GITLAB_USERNAME").unwrap_or_else(|_| {
        eprintln!("❌ Error: USERNAME not found in .env or environment.");
        print_help();
        process::exit(1);
    });

    (gitlab_token, gitlab_username, github_token, github_username)
}

const GITLAB_URL: &str = "https://gitlab.com";

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

fn get_github_events(client: &Client, token: &str, username: &str) -> Result<Vec<Value>, reqwest::Error> {
    let url = format!("https://api.github.com/users/{}/events", username);
    let resp = client
        .get(&url)
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", "gitlab_activity_fetcher") // GitHub requires a User-Agent header
        .send()?
        .json::<Vec<Value>>()?;

    println!("Fetched {} events for GitHub user {}", resp.len(), username);
    Ok(resp)
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

fn print_help() {
    println!(
        "Usage: gitlab_activity [OPTIONS]

Options:
  --help, -h          Print this help message
  --since <DATE>      Start date for fetching events (format: YYYY-MM-DD)
  --until <DATE>      End date for fetching events (format: YYYY-MM-DD)

Description:
  This program fetches GitLab activity events for a user and saves them to a JSON file.
  By default, it fetches events from the start of the current month to the current date.

Examples:
  Fetch events for the current month:
    gitlab_activity

  Fetch events for a specific date range:
    gitlab_activity --since 2025-03-01 --until 2025-03-31
"
    );
}

fn parse_args() -> (Option<String>, Option<String>) {
    let args: Vec<String> = env::args().collect();
    let mut since = None;
    let mut until = None;

    eprintln!("Error: Missing value for --since");
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_help();
                process::exit(0);
            }
            "--since" => {
                if i + 1 < args.len() {
                    since = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    eprintln!("Error: Missing value for --since");
                    process::exit(1);
                }
            }
            "--until" => {
                if i + 1 < args.len() {
                    until = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    eprintln!("Error: Missing value for --until");
                    process::exit(1);
                }
            }
            _ => {
                eprintln!("Error: Unknown argument '{}'", args[i]);
                process::exit(1);
            }
        }
        i += 1;
    }

    (since, until)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments
    let (since_arg, until_arg) = parse_args();

    let (gitlab_token, gitlab_username, github_token, github_username) = load_env_or_exit();
    let client = Client::new();

    // Determine date range
    let (since, until) = if let (Some(since), Some(until)) = (since_arg, until_arg) {
        (since, until)
    } else {
        // Default to the current month to date
        let now = Utc::now().naive_utc();
        let date = now.date(); // Extract the NaiveDate
        let start_of_month = NaiveDate::from_ymd_opt(date.year(), date.month(), 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let today = date.and_hms_opt(23, 59, 59).unwrap();

        (start_of_month.to_string(), today.to_string())
    };

    println!("Fetching events from {} to {}", since, until);

    // Fetch GitLab events
    let user_id = get_user_id(&client, &gitlab_token, &gitlab_username)?;
    let mut gitlab_events = get_user_events(&client, &gitlab_token, user_id, &since)?;

    for event in &mut gitlab_events {
        populate_event_details(&client, &gitlab_token, event)?;
    }

    // Fetch GitHub events
    let github_events = get_github_events(&client, &github_token, &github_username)?;

    // Combine events from both platforms
    let mut all_events = gitlab_events;
    all_events.extend(github_events);

    // Serialize events to JSON and save to a file
    let file_path = "all_activity.json";
    let mut file = File::create(file_path)?;
    let json_data = serde_json::to_string_pretty(&all_events)?;
    file.write_all(json_data.as_bytes())?;

    println!("✅ Saved {} events to {}", all_events.len(), file_path);
    Ok(())
}

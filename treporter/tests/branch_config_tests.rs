use chronopulse::config::{Config, RepositoryConfig, BranchConfig};
use std::fs;
use tempfile::TempDir;

/// Helper to create a minimal test config
fn create_test_config(repo: RepositoryConfig) -> Config {
    Config {
        project: chronopulse::config::ProjectConfig {
            name: "Test".to_string(),
            description: "Test".to_string(),
        },
        date_range: chronopulse::config::DateRangeConfig {
            default_period: "2025-01".to_string(),
        },
        collection: chronopulse::config::CollectionConfig {
            include_diffs: false,
            max_diff_size: 10000,
        },
        data_sources: chronopulse::config::DataSourcesConfig {
            gitlab: None,
            github: None,
            local_files: None,
        },
        repositories: vec![repo],
        repository_discovery: None,
        llm: chronopulse::config::LlmConfig {
            provider: "ollama".to_string(),
            model: "test".to_string(),
            api_key_env: "TEST_KEY".to_string(),
            max_tokens: 1000,
            temperature: 0.7,
        },
        templates: chronopulse::config::TemplatesConfig {
            monthly_report: chronopulse::config::TemplateConfig {
                path: "templates/monthly_report.md".to_string(),
                output_format: "markdown".to_string(),
                sections: vec![],
            },
            group_slides: chronopulse::config::TemplateConfig {
                path: "templates/group_slides.md".to_string(),
                output_format: "markdown".to_string(),
                sections: vec![],
            },
        },
        output: chronopulse::config::OutputConfig {
            base_directory: "./reports".to_string(),
            date_format: "%Y-%m".to_string(),
            filename_template: "test".to_string(),
        },
        reporting: None,
    }
}

#[test]
fn test_branch_config_default() {
    let repo = RepositoryConfig {
        name: "test-repo".to_string(),
        platform: "local".to_string(),
        group: None,
        project_id: None,
        path: Some("/tmp/test".to_string()),
        include_issues: None,
        include_merge_requests: None,
        include_commits: Some(true),
        include_wiki: None,
        branches: Some(BranchConfig::Default),
    };
    
    let config = create_test_config(repo);
    assert_eq!(config.repositories.len(), 1);
    
    match &config.repositories[0].branches {
        Some(BranchConfig::Default) => (),
        _ => panic!("Expected BranchConfig::Default"),
    }
}

#[test]
fn test_branch_config_all() {
    let repo = RepositoryConfig {
        name: "test-repo".to_string(),
        platform: "local".to_string(),
        group: None,
        project_id: None,
        path: Some("/tmp/test".to_string()),
        include_issues: None,
        include_merge_requests: None,
        include_commits: Some(true),
        include_wiki: None,
        branches: Some(BranchConfig::All),
    };
    
    let config = create_test_config(repo);
    
    match &config.repositories[0].branches {
        Some(BranchConfig::All) => (),
        _ => panic!("Expected BranchConfig::All"),
    }
}

#[test]
fn test_branch_config_list() {
    let branches = vec!["main".to_string(), "develop".to_string(), "feature/test".to_string()];
    
    let repo = RepositoryConfig {
        name: "test-repo".to_string(),
        platform: "local".to_string(),
        group: None,
        project_id: None,
        path: Some("/tmp/test".to_string()),
        include_issues: None,
        include_merge_requests: None,
        include_commits: Some(true),
        include_wiki: None,
        branches: Some(BranchConfig::List(branches.clone())),
    };
    
    let config = create_test_config(repo);
    
    match &config.repositories[0].branches {
        Some(BranchConfig::List(list)) => {
            assert_eq!(list.len(), 3);
            assert_eq!(list[0], "main");
            assert_eq!(list[1], "develop");
            assert_eq!(list[2], "feature/test");
        },
        _ => panic!("Expected BranchConfig::List"),
    }
}

#[test]
fn test_branch_config_none_defaults_to_default() {
    let repo = RepositoryConfig {
        name: "test-repo".to_string(),
        platform: "local".to_string(),
        group: None,
        project_id: None,
        path: Some("/tmp/test".to_string()),
        include_issues: None,
        include_merge_requests: None,
        include_commits: Some(true),
        include_wiki: None,
        branches: None,
    };
    
    let config = create_test_config(repo);
    
    // When branches is None, it should default to collecting from default branch
    match &config.repositories[0].branches {
        None => (),
        _ => panic!("Expected None for branches"),
    }
}

#[test]
fn test_branch_config_serialization() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    
    // Create a config with different branch configurations
    let config_content = r#"
[project]
name = "test-project"
description = "Test"

[date_range]
default_period = "2025-01"

[collection]
include_diffs = false
max_diff_size = 10000

[data_sources]

[[repositories]]
name = "repo-default"
platform = "local"
path = "/tmp/test1"
include_commits = true
branches = "default"

[[repositories]]
name = "repo-all"
platform = "local"
path = "/tmp/test2"
include_commits = true
branches = "all"

[[repositories]]
name = "repo-list"
platform = "local"
path = "/tmp/test3"
include_commits = true
branches = { list = ["main", "develop"] }

[llm]
provider = "ollama"
model = "test"
api_key_env = "TEST_KEY"
max_tokens = 1000
temperature = 0.7

[templates]

[templates.monthly_report]
path = "templates/monthly_report.md"
output_format = "markdown"
sections = ["summary"]

[templates.group_slides]
path = "templates/group_slides.md"
output_format = "markdown"
sections = ["highlights"]

[output]
base_directory = "./reports"
date_format = "%Y-%m"
filename_template = "test"
"#;
    
    fs::write(&config_path, config_content).unwrap();
    
    // Load and verify
    let config = Config::load(config_path.to_str().unwrap()).unwrap();
    
    assert_eq!(config.repositories.len(), 3);
    
    // Check first repo (default)
    match &config.repositories[0].branches {
        Some(BranchConfig::Default) => (),
        _ => panic!("Expected BranchConfig::Default for repo-default"),
    }
    
    // Check second repo (all)
    match &config.repositories[1].branches {
        Some(BranchConfig::All) => (),
        _ => panic!("Expected BranchConfig::All for repo-all"),
    }
    
    // Check third repo (list)
    match &config.repositories[2].branches {
        Some(BranchConfig::List(list)) => {
            assert_eq!(list.len(), 2);
            assert_eq!(list[0], "main");
            assert_eq!(list[1], "develop");
        },
        _ => panic!("Expected BranchConfig::List for repo-list"),
    }
}

#[test]
fn test_multiple_repos_different_branch_configs() {
    let repos = vec![
        RepositoryConfig {
            name: "repo1".to_string(),
            platform: "local".to_string(),
            group: None,
            project_id: None,
            path: Some("/tmp/repo1".to_string()),
            include_issues: None,
            include_merge_requests: None,
            include_commits: Some(true),
            include_wiki: None,
            branches: Some(BranchConfig::Default),
        },
        RepositoryConfig {
            name: "repo2".to_string(),
            platform: "local".to_string(),
            group: None,
            project_id: None,
            path: Some("/tmp/repo2".to_string()),
            include_issues: None,
            include_merge_requests: None,
            include_commits: Some(true),
            include_wiki: None,
            branches: Some(BranchConfig::All),
        },
        RepositoryConfig {
            name: "repo3".to_string(),
            platform: "local".to_string(),
            group: None,
            project_id: None,
            path: Some("/tmp/repo3".to_string()),
            include_issues: None,
            include_merge_requests: None,
            include_commits: Some(true),
            include_wiki: None,
            branches: Some(BranchConfig::List(vec!["main".to_string(), "staging".to_string()])),
        },
    ];
    
    let mut config = create_test_config(repos[0].clone());
    config.repositories = repos;
    
    assert_eq!(config.repositories.len(), 3);
    
    // Verify each repo has its correct branch config
    match &config.repositories[0].branches {
        Some(BranchConfig::Default) => (),
        _ => panic!("repo1 should have Default"),
    }
    
    match &config.repositories[1].branches {
        Some(BranchConfig::All) => (),
        _ => panic!("repo2 should have All"),
    }
    
    match &config.repositories[2].branches {
        Some(BranchConfig::List(list)) => {
            assert_eq!(list.len(), 2);
            assert_eq!(list[0], "main");
            assert_eq!(list[1], "staging");
        },
        _ => panic!("repo3 should have List"),
    }
}

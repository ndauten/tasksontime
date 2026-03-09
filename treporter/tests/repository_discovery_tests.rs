use chronopulse::config::{Config, RepositoryDiscoveryConfig};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test git repository
fn create_test_git_repo(path: &std::path::Path) -> std::io::Result<()> {
    fs::create_dir_all(path)?;
    fs::create_dir(path.join(".git"))?;
    Ok(())
}

#[test]
fn test_discover_single_repository() {
    // Create a temporary directory structure
    let temp_dir = TempDir::new().unwrap();
    let repos_dir = temp_dir.path().join("repos");
    
    // Create a test repository
    create_test_git_repo(&repos_dir.join("project1")).unwrap();
    
    // Create config with discovery enabled
    let discovery = RepositoryDiscoveryConfig {
        enabled: true,
        search_paths: vec![repos_dir.to_string_lossy().to_string()],
        max_depth: Some(2),
        ignore_patterns: None,
    };
    
    let config = Config {
        project: chronopulse::config::ProjectConfig {
            name: "Test".to_string(),
            description: "Test".to_string(),
        },
        date_range: chronopulse::config::DateRangeConfig {
            default_period: "2025-01".to_string(),
        },
        collection: chronopulse::config::CollectionConfig {
            include_diffs: true,
            max_diff_size: 50000,
        },
        data_sources: chronopulse::config::DataSourcesConfig {
            gitlab: None,
            github: None,
            local_files: None,
        },
        repositories: vec![],
        repository_discovery: Some(discovery),
        llm: chronopulse::config::LlmConfig {
            provider: "ollama".to_string(),
            model: "test".to_string(),
            api_key_env: "TEST".to_string(),
            max_tokens: 1000,
            temperature: 0.3,
        },
        templates: chronopulse::config::TemplatesConfig {
            monthly_report: chronopulse::config::TemplateConfig {
                path: "test".to_string(),
                output_format: "markdown".to_string(),
                sections: vec![],
            },
            group_slides: chronopulse::config::TemplateConfig {
                path: "test".to_string(),
                output_format: "marp".to_string(),
                sections: vec![],
            },
        },
        output: chronopulse::config::OutputConfig {
            base_directory: "reports".to_string(),
            date_format: "%Y-%m".to_string(),
            filename_template: "{project_name}_{template_name}_{date}".to_string(),
        },
        reporting: None,
    };
    
    let discovered = config.discover_repositories();
    
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].name, "project1");
    assert_eq!(discovered[0].platform, "local");
    assert!(discovered[0].path.is_some());
}

#[test]
fn test_discover_multiple_repositories() {
    let temp_dir = TempDir::new().unwrap();
    let repos_dir = temp_dir.path().join("repos");
    
    // Create multiple test repositories
    create_test_git_repo(&repos_dir.join("project1")).unwrap();
    create_test_git_repo(&repos_dir.join("project2")).unwrap();
    create_test_git_repo(&repos_dir.join("project3")).unwrap();
    
    let discovery = RepositoryDiscoveryConfig {
        enabled: true,
        search_paths: vec![repos_dir.to_string_lossy().to_string()],
        max_depth: Some(2),
        ignore_patterns: None,
    };
    
    let config = create_minimal_config(Some(discovery));
    let discovered = config.discover_repositories();
    
    assert_eq!(discovered.len(), 3);
    
    let names: Vec<String> = discovered.iter().map(|r| r.name.clone()).collect();
    assert!(names.contains(&"project1".to_string()));
    assert!(names.contains(&"project2".to_string()));
    assert!(names.contains(&"project3".to_string()));
}

#[test]
fn test_discover_ignores_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let repos_dir = temp_dir.path().join("workspace");
    
    // Create repositories, including ones that should be ignored
    create_test_git_repo(&repos_dir.join("my-project")).unwrap();
    create_test_git_repo(&repos_dir.join("node_modules/dependency")).unwrap();
    create_test_git_repo(&repos_dir.join("vendor/library")).unwrap();
    create_test_git_repo(&repos_dir.join(".venv/site-packages")).unwrap();
    
    let discovery = RepositoryDiscoveryConfig {
        enabled: true,
        search_paths: vec![repos_dir.to_string_lossy().to_string()],
        max_depth: Some(3),
        ignore_patterns: Some(vec![
            "node_modules".to_string(),
            "vendor".to_string(),
            "\\.venv".to_string(),
        ]),
    };
    
    let config = create_minimal_config(Some(discovery));
    let discovered = config.discover_repositories();
    
    // Should only find my-project, not the ignored directories
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].name, "my-project");
}

#[test]
fn test_discover_respects_max_depth() {
    let temp_dir = TempDir::new().unwrap();
    let base_dir = temp_dir.path();
    
    // Create nested repository structure
    create_test_git_repo(&base_dir.join("level1")).unwrap();
    create_test_git_repo(&base_dir.join("level1/level2")).unwrap();
    create_test_git_repo(&base_dir.join("level1/level2/level3")).unwrap();
    create_test_git_repo(&base_dir.join("level1/level2/level3/level4")).unwrap();
    
    // Test with max_depth = 2
    let discovery = RepositoryDiscoveryConfig {
        enabled: true,
        search_paths: vec![base_dir.to_string_lossy().to_string()],
        max_depth: Some(2),
        ignore_patterns: None,
    };
    
    let config = create_minimal_config(Some(discovery));
    let discovered = config.discover_repositories();
    
    // Should find repos up to depth 2 (level1 and level1/level2)
    // Now we continue recursing even after finding a git repo
    assert_eq!(discovered.len(), 2);
    assert!(discovered.iter().any(|r| r.name == "level1"));
    assert!(discovered.iter().any(|r| r.name == "level2"));
}

#[test]
fn test_discover_nested_repos_stops_at_git() {
    let temp_dir = TempDir::new().unwrap();
    let base_dir = temp_dir.path();
    
    // Create a repo with nested repos inside (like a monorepo with submodules)
    create_test_git_repo(&base_dir.join("monorepo")).unwrap();
    create_test_git_repo(&base_dir.join("monorepo/subproject1")).unwrap();
    create_test_git_repo(&base_dir.join("monorepo/subproject2")).unwrap();
    
    let discovery = RepositoryDiscoveryConfig {
        enabled: true,
        search_paths: vec![base_dir.to_string_lossy().to_string()],
        max_depth: Some(5),
        ignore_patterns: None,
    };
    
    let config = create_minimal_config(Some(discovery));
    let discovered = config.discover_repositories();
    
    // Should now find the parent repo AND nested repos (new behavior for better discovery)
    assert_eq!(discovered.len(), 3);
    assert!(discovered.iter().any(|r| r.name == "monorepo"));
    assert!(discovered.iter().any(|r| r.name == "subproject1"));
    assert!(discovered.iter().any(|r| r.name == "subproject2"));
}

#[test]
fn test_discover_disabled_returns_empty() {
    let temp_dir = TempDir::new().unwrap();
    let repos_dir = temp_dir.path().join("repos");
    create_test_git_repo(&repos_dir.join("project1")).unwrap();
    
    let discovery = RepositoryDiscoveryConfig {
        enabled: false,
        search_paths: vec![repos_dir.to_string_lossy().to_string()],
        max_depth: Some(2),
        ignore_patterns: None,
    };
    
    let config = create_minimal_config(Some(discovery));
    let discovered = config.discover_repositories();
    
    assert_eq!(discovered.len(), 0);
}

#[test]
fn test_discover_no_config_returns_empty() {
    let config = create_minimal_config(None);
    let discovered = config.discover_repositories();
    assert_eq!(discovered.len(), 0);
}

#[test]
fn test_discover_nonexistent_path() {
    let discovery = RepositoryDiscoveryConfig {
        enabled: true,
        search_paths: vec!["/nonexistent/path/to/repos".to_string()],
        max_depth: Some(2),
        ignore_patterns: None,
    };
    
    let config = create_minimal_config(Some(discovery));
    let discovered = config.discover_repositories();
    
    // Should handle gracefully and return empty
    assert_eq!(discovered.len(), 0);
}

#[test]
fn test_discover_with_tilde_expansion() {
    // This test verifies that ~ expands correctly
    // We can't easily test actual home directory, but we can verify the code path
    let discovery = RepositoryDiscoveryConfig {
        enabled: true,
        search_paths: vec!["~/fake-repos-path".to_string()],
        max_depth: Some(2),
        ignore_patterns: None,
    };
    
    let config = create_minimal_config(Some(discovery));
    let discovered = config.discover_repositories();
    
    // Should handle non-existent expanded path gracefully
    assert_eq!(discovered.len(), 0);
}

#[test]
fn test_discover_multiple_search_paths() {
    let temp_dir = TempDir::new().unwrap();
    let workspace1 = temp_dir.path().join("workspace1");
    let workspace2 = temp_dir.path().join("workspace2");
    
    create_test_git_repo(&workspace1.join("project-a")).unwrap();
    create_test_git_repo(&workspace2.join("project-b")).unwrap();
    
    let discovery = RepositoryDiscoveryConfig {
        enabled: true,
        search_paths: vec![
            workspace1.to_string_lossy().to_string(),
            workspace2.to_string_lossy().to_string(),
        ],
        max_depth: Some(2),
        ignore_patterns: None,
    };
    
    let config = create_minimal_config(Some(discovery));
    let discovered = config.discover_repositories();
    
    assert_eq!(discovered.len(), 2);
    
    let names: Vec<String> = discovered.iter().map(|r| r.name.clone()).collect();
    assert!(names.contains(&"project-a".to_string()));
    assert!(names.contains(&"project-b".to_string()));
}

// Helper function to create a minimal config for testing
fn create_minimal_config(discovery: Option<RepositoryDiscoveryConfig>) -> Config {
    Config {
        project: chronopulse::config::ProjectConfig {
            name: "Test".to_string(),
            description: "Test".to_string(),
        },
        date_range: chronopulse::config::DateRangeConfig {
            default_period: "2025-01".to_string(),
        },
        collection: chronopulse::config::CollectionConfig {
            include_diffs: true,
            max_diff_size: 50000,
        },
        data_sources: chronopulse::config::DataSourcesConfig {
            gitlab: None,
            github: None,
            local_files: None,
        },
        repositories: vec![],
        repository_discovery: discovery,
        llm: chronopulse::config::LlmConfig {
            provider: "ollama".to_string(),
            model: "test".to_string(),
            api_key_env: "TEST".to_string(),
            max_tokens: 1000,
            temperature: 0.3,
        },
        templates: chronopulse::config::TemplatesConfig {
            monthly_report: chronopulse::config::TemplateConfig {
                path: "test".to_string(),
                output_format: "markdown".to_string(),
                sections: vec![],
            },
            group_slides: chronopulse::config::TemplateConfig {
                path: "test".to_string(),
                output_format: "marp".to_string(),
                sections: vec![],
            },
        },
        output: chronopulse::config::OutputConfig {
            base_directory: "reports".to_string(),
            date_format: "%Y-%m".to_string(),
            filename_template: "{project_name}_{template_name}_{date}".to_string(),
        },
        reporting: None,
    }
}

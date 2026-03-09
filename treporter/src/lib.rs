// ChronoPulse - Automated project reporting tool with LLM integration
//
// This library provides functionality for collecting data from various sources
// (GitLab, GitHub, local files, git repositories) and generating automated
// reports using LLM integration.

pub mod config;
pub mod credentials;
pub mod types;
pub mod collectors;
pub mod llm;
pub mod generators;
pub mod cli;
pub mod preprocessor;
pub mod ollama;
pub mod retry_loop;
pub mod simple_pipeline;
pub mod direct_file_processor;
pub mod structured_extractor;
pub mod multistage_pipeline;

pub use config::Config;
pub use types::CollectedData;
pub use collectors::DataCollector;
pub use generators::ReportGenerator;

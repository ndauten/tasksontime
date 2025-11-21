// TReporter - Automated project reporting tool with LLM integration

pub mod config;
pub mod types;
pub mod collectors;
pub mod llm;
pub mod generators;
pub mod cli;

pub use config::Config;
pub use types::CollectedData;
pub use collectors::DataCollector;
pub use generators::ReportGenerator;

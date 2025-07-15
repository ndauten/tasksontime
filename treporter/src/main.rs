mod config;
mod types;
mod collectors;
mod llm;
mod generators;
mod cli;
mod preprocessor;
mod ollama;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use collectors::DataCollector;
use config::Config;
use generators::ReportGenerator;
use types::CollectedData;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    
    let cli = Cli::parse();
    
    // Load configuration
    let config = Config::load(&cli.config)?;
    
    if cli.verbose {
        println!("📋 Loaded configuration from: {}", cli.config);
        println!("🎯 Project: {}", config.project.name);
    }
    
    match cli.command {
        Commands::Config => {
            println!("📋 Configuration:");
            println!("  Project: {}", config.project.name);
            println!("  Description: {}", config.project.description);
            println!("  Data Sources:");
            if config.data_sources.gitlab.as_ref().map_or(false, |g| g.enabled) {
                println!("    ✅ GitLab");
            }
            if config.data_sources.github.as_ref().map_or(false, |g| g.enabled) {
                println!("    ✅ GitHub");
            }
            if config.data_sources.local_files.as_ref().map_or(false, |l| l.enabled) {
                println!("    ✅ Local Files");
            }
            println!("  Repositories: {}", config.repositories.len());
            println!("  LLM Provider: {}", config.llm.provider);
        },
        
        Commands::Test => {
            let (start_date, end_date) = cli.parse_date_range()?;
            println!("🧪 Testing data collection from {} to {}", 
                start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d"));
            
            let collector = DataCollector::new(config);
            let data = collector.collect(start_date, end_date).await?;
            
            println!("📊 Collection Results:");
            println!("  GitLab raw data: {}", data.gitlab.total_items());
            println!("  GitHub raw data: {}", data.github.total_items());
            println!("  Local files: {}", data.local_files.len());
            println!("  Git commits: {}", data.git_commits.len());
            println!("  Total items: {}", data.total_items());
        },
        
        Commands::Collect { ref output } => {
            let (start_date, end_date) = cli.parse_date_range()?;
            println!("🔍 Collecting data from {} to {}", 
                start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d"));
            
            let collector = DataCollector::new(config);
            let data = collector.collect(start_date, end_date).await?;
            
            data.save_to_file(output)?;
            println!("💾 Saved collected data to: {}", output);
            println!("📊 Total items collected: {}", data.total_items());
        },
        
        Commands::MonthlyReport { ref data_file, ref data_files } => {
            let data = match (data_file, data_files.is_empty()) {
                (Some(file), true) => {
                    println!("📂 Loading data from: {}", file);
                    CollectedData::load_from_file(file)?
                },
                (None, false) => {
                    println!("📂 Loading and merging data from {} files...", data_files.len());
                    for file in data_files {
                        println!("  - {}", file);
                    }
                    CollectedData::load_and_merge_files(data_files)?
                },
                (Some(_), false) => {
                    return Err(anyhow::anyhow!("Cannot specify both --data-file and --data-files"));
                },
                (None, true) => {
                    let (start_date, end_date) = cli.parse_date_range()?;
                    println!("🔍 Collecting fresh data...");
                    let collector = DataCollector::new(config.clone());
                    collector.collect(start_date, end_date).await?
                }
            };
            
            let generator = ReportGenerator::new(config)?;
            let report_path = generator.generate_monthly_report(&data).await?;
            println!("📄 Monthly report generated: {}", report_path);
        },
        
        Commands::GroupSlides { ref data_file } => {
            let data = match data_file {
                Some(file) => {
                    println!("📂 Loading data from: {}", file);
                    CollectedData::load_from_file(file)?
                },
                None => {
                    let (start_date, end_date) = cli.parse_date_range()?;
                    println!("🔍 Collecting fresh data...");
                    let collector = DataCollector::new(config.clone());
                    collector.collect(start_date, end_date).await?
                }
            };
            
            let generator = ReportGenerator::new(config)?;
            let slides_path = generator.generate_group_slides(&data).await?;
            println!("📊 Group slides generated: {}", slides_path);
        },
        
        Commands::All { ref data_file } => {
            let data = match data_file {
                Some(file) => {
                    println!("📂 Loading data from: {}", file);
                    CollectedData::load_from_file(file)?
                },
                None => {
                    let (start_date, end_date) = cli.parse_date_range()?;
                    println!("🔍 Collecting fresh data...");
                    let collector = DataCollector::new(config.clone());
                    collector.collect(start_date, end_date).await?
                }
            };
            
            let generator = ReportGenerator::new(config)?;
            let output_paths = generator.generate_all_reports(&data).await?;
            
            println!("🎉 Generated {} reports:", output_paths.len());
            for path in output_paths {
                println!("  📄 {}", path);
            }
        },
        
        Commands::ArchitecturalAnalysis { ref data_file, ref data_files } => {
            let data = match (data_file, data_files.is_empty()) {
                (Some(file), true) => {
                    println!("📂 Loading data from: {}", file);
                    CollectedData::load_from_file(file)?
                },
                (None, false) => {
                    println!("📂 Loading and merging data from {} files...", data_files.len());
                    for file in data_files {
                        println!("  - {}", file);
                    }
                    CollectedData::load_and_merge_files(data_files)?
                },
                (Some(_), false) => {
                    return Err(anyhow::anyhow!("Cannot specify both --data-file and --data-files"));
                },
                (None, true) => {
                    let (start_date, end_date) = cli.parse_date_range()?;
                    println!("🔍 Collecting fresh data...");
                    let collector = DataCollector::new(config.clone());
                    collector.collect(start_date, end_date).await?
                }
            };
            
            let generator = ReportGenerator::new(config)?;
            let analysis_path = generator.generate_architectural_analysis(&data).await?;
            println!("🏗️  Architectural analysis generated: {}", analysis_path);
        },
    }
    
    Ok(())
}

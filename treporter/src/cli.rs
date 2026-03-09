use clap::{Parser, Subcommand};
use chrono::{DateTime, Utc, NaiveDate, Datelike};

#[derive(Parser)]
#[command(name = "chronopulse")]
#[command(about = "ChronoPulse - Automated project reporting tool")]
#[command(version = "1.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    
    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    pub config: String,
    
    /// Start date for data collection (YYYY-MM-DD)
    #[arg(long)]
    pub from: Option<String>,
    
    /// End date for data collection (YYYY-MM-DD)  
    #[arg(long)]
    pub to: Option<String>,
    
    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Collect raw data from all configured sources
    Collect {
        /// Output file for collected data
        #[arg(short, long, default_value = "collected_data.json")]
        output: String,
    },
    
    /// Generate monthly project report
    MonthlyReport {
        /// Use previously collected data file
        #[arg(short, long)]
        data_file: Option<String>,
        
        /// Use multiple data files to merge
        #[arg(long, num_args = 1.., value_delimiter = ' ')]
        data_files: Vec<String>,
    },
    
    /// Generate group meeting slides
    GroupSlides {
        /// Use previously collected data file
        #[arg(short, long)]
        data_file: Option<String>,
    },
    
    /// Generate all reports (monthly + slides)
    All {
        /// Use previously collected data file
        #[arg(short, long)]
        data_file: Option<String>,
    },
    
    /// Generate comprehensive architectural analysis report
    ArchitecturalAnalysis {
        /// Use previously collected data file
        #[arg(short, long)]
        data_file: Option<String>,
        
        /// Use multiple data files to merge
        #[arg(long, num_args = 1.., value_delimiter = ' ')]
        data_files: Vec<String>,
        
        /// Output filename (without extension)
        #[arg(short, long)]
        output: Option<String>,
    },
    
    /// Generate simple architectural report (new pipeline)
    NewReport {
        /// Use previously collected data file
        #[arg(short, long)]
        data_file: Option<String>,
        
        /// Use multiple data files to merge
        #[arg(long, num_args = 1.., value_delimiter = ' ')]
        data_files: Vec<String>,
        
        /// Output filename (without extension)
        #[arg(short, long)]
        output: Option<String>,
    },
    
    /// Generate report using multi-stage local LLM pipeline (optimized for local models)
    LocalReport {
        /// Use previously collected data file
        #[arg(short, long)]
        data_file: Option<String>,
        
        /// Output filename
        #[arg(short, long)]
        output: Option<String>,
        
        /// Export structured data as JSON
        #[arg(long)]
        export_structured: bool,
        
        /// Export stage-by-stage outputs for debugging
        #[arg(long)]
        export_stages: bool,
    },
    
    /// Analyze any file directly with LLM (no preprocessing)
    DirectAnalysis {
        /// Input file to analyze
        #[arg(short, long)]
        input: String,
        
        /// Output file for analysis
        #[arg(short, long)]
        output: Option<String>,
    },
    
    /// Show configuration information
    Config,
    
    /// Test data collection without generating reports
    Test,
    
    /// Initialize a new config.toml with default settings
    Init {
        /// Output path for the config file
        #[arg(short, long, default_value = "config.toml")]
        output: String,
        
        /// Force overwrite if file exists
        #[arg(short, long)]
        force: bool,
    },
    
    /// Set up global credentials and/or project configuration
    Setup {
        /// Force overwrite if file exists
        #[arg(short, long)]
        force: bool,
        
        /// Show current global configuration
        #[arg(short, long)]
        show: bool,
        
        /// Set up project configuration only (skip global credentials)
        #[arg(short, long)]
        project: bool,
        
        /// Set up global credentials only (skip project setup)
        #[arg(short, long)]
        global: bool,
    },
}

impl Cli {
    pub fn parse_date_range(&self) -> anyhow::Result<(DateTime<Utc>, DateTime<Utc>)> {
        let (start_date, end_date) = match (&self.from, &self.to) {
            (Some(from), Some(to)) => {
                let start = NaiveDate::parse_from_str(from, "%Y-%m-%d")?
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc();
                let end = NaiveDate::parse_from_str(to, "%Y-%m-%d")?
                    .and_hms_opt(23, 59, 59)
                    .unwrap()
                    .and_utc();
                (start, end)
            },
            _ => {
                // Default to current month
                let now = Utc::now();
                let start_of_month = NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc();
                (start_of_month, now)
            }
        };
        
        Ok((start_date, end_date))
    }
}

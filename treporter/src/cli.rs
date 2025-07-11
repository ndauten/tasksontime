use clap::{Parser, Subcommand};
use chrono::{DateTime, Utc, NaiveDate, Datelike};

#[derive(Parser)]
#[command(name = "treporter")]
#[command(about = "Automated project reporting tool with LLM integration")]
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
    
    /// Show configuration information
    Config,
    
    /// Test data collection without generating reports
    Test,
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

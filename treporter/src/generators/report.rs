use crate::config::Config;
use crate::types::CollectedData;
use crate::llm::LLMClient;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::Path;

pub struct ReportGenerator {
    config: Config,
    llm_client: LLMClient,
}

impl ReportGenerator {
    pub fn new(config: Config) -> Result<Self> {
        let llm_client = LLMClient::new(config.clone());
        
        Ok(Self {
            config,
            llm_client,
        })
    }

    pub async fn generate_monthly_report(&self, data: &CollectedData) -> Result<String> {
        let template_config = &self.config.templates.monthly_report;
        let template_content = self.load_template(&template_config.path)?;
        
        println!("🤖 Generating monthly report using LLM...");
        let report = self.llm_client
            .generate_monthly_report(data, &template_content)
            .await?;
        
        let output_path = self.generate_output_path("monthly_report", &data.metadata.date_range_start)?;
        fs::write(&output_path, &report)?;
        
        println!("✅ Monthly report saved to: {}", output_path);
        Ok(output_path)
    }

    pub async fn generate_group_slides(&self, data: &CollectedData) -> Result<String> {
        let template_config = &self.config.templates.group_slides;
        let template_content = self.load_template(&template_config.path)?;
        
        println!("🤖 Generating group meeting slides using LLM...");
        let slides = self.llm_client
            .generate_group_slides(data)
            .await?;
        
        let output_path = self.generate_output_path("group_slides", &data.metadata.date_range_start)?;
        fs::write(&output_path, &slides)?;
        
        println!("✅ Group slides saved to: {}", output_path);
        Ok(output_path)
    }

    pub async fn generate_all_reports(&self, data: &CollectedData) -> Result<Vec<String>> {
        let mut output_paths = Vec::new();
        
        // Generate monthly report
        match self.generate_monthly_report(data).await {
            Ok(path) => output_paths.push(path),
            Err(e) => println!("⚠️  Failed to generate monthly report: {}", e),
        }
        
        // Generate group slides
        match self.generate_group_slides(data).await {
            Ok(path) => output_paths.push(path),
            Err(e) => println!("⚠️  Failed to generate group slides: {}", e),
        }
        
        Ok(output_paths)
    }

    fn load_template(&self, template_path: &str) -> Result<String> {
        if Path::new(template_path).exists() {
            Ok(fs::read_to_string(template_path)?)
        } else {
            // Return a default template if the file doesn't exist
            match template_path {
                path if path.contains("monthly_report") => Ok(self.default_monthly_template()),
                path if path.contains("group_slides") => Ok(self.default_slides_template()),
                _ => Err(anyhow!("Template file not found: {}", template_path)),
            }
        }
    }

    fn default_monthly_template(&self) -> String {
        r#"# Monthly Project Report - {project_name}

## Executive Summary
Provide a high-level overview of the project progress during this period.

## Key Accomplishments
List the major achievements and milestones reached during this month:

## Technical Progress
Describe the technical work completed:

### Development Activity
- Code commits and changes
- New features implemented
- Bug fixes and improvements

### Repository Activity
- GitLab/GitHub activity summary
- Collaboration and reviews

## Challenges and Issues
Document any blockers or challenges encountered:

## Project Health Metrics
- Activity levels
- Commit frequency
- Issue resolution

## Next Month's Priorities
Outline the planned focus areas for the upcoming month:

## Appendix
### Detailed Activity Log
Include relevant details from the collected data.
"#.to_string()
    }

    fn default_slides_template(&self) -> String {
        r#"---
marp: true
theme: default
paginate: true
---

# Project Update
## {project_name}
### {date_range}

---

## Highlights This Month

- Key accomplishment 1
- Key accomplishment 2  
- Key accomplishment 3

---

## Development Progress

### Code Activity
- X commits across Y repositories
- Z lines of code added/modified
- Major features completed

---

## Current Status

### ✅ Completed
- List completed items

### 🔄 In Progress  
- List ongoing work

### ⚠️ Blockers
- List any blockers or issues

---

## Next Steps

### Short Term (Next 2 weeks)
- Priority item 1
- Priority item 2

### Medium Term (Next month)
- Goal 1
- Goal 2

---

## Questions & Discussion
"#.to_string()
    }

    fn generate_output_path(&self, template_name: &str, date: &DateTime<Utc>) -> Result<String> {
        // Ensure output directory exists
        let output_dir = &self.config.output.base_directory;
        fs::create_dir_all(output_dir)?;
        
        let date_str = date.format(&self.config.output.date_format).to_string();
        let filename = self.config.output.filename_template
            .replace("{project_name}", &self.config.project.name.replace(" ", "_"))
            .replace("{template_name}", template_name)
            .replace("{date}", &date_str);
        
        let extension = match template_name {
            "group_slides" => "md",
            _ => "md",
        };
        
        Ok(format!("{}/{}.{}", output_dir, filename, extension))
    }
}

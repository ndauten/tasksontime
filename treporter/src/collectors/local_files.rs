use crate::config::LocalFilesConfig;
use crate::types::{LocalFileData, ContentSnippet, TimeBasedEntry};
use anyhow::Result;
use chrono::{DateTime, Utc};
use glob::glob;
use regex::Regex;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub struct LocalFilesCollector {
    config: LocalFilesConfig,
    time_patterns: Vec<Regex>,
}

impl LocalFilesCollector {
    pub fn new(config: &LocalFilesConfig) -> Result<Self> {
        let time_patterns = config.time_patterns
            .iter()
            .map(|pattern| Regex::new(pattern))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            config: config.clone(),
            time_patterns,
        })
    }

    pub fn collect(&self, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<LocalFileData>> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }

        println!("🔍 Collecting local file data from {} to {}", start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d"));
        
        let mut all_files = Vec::new();

        for path_pattern in &self.config.paths {
            let files = self.collect_files_from_pattern(path_pattern, start_date, end_date)?;
            all_files.extend(files);
        }

        println!("✅ Collected data from {} local files", all_files.len());
        Ok(all_files)
    }

    fn collect_files_from_pattern(&self, pattern: &str, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<LocalFileData>> {
        let mut files = Vec::new();

        if pattern.contains("**") {
            // Handle glob patterns
            for entry in glob(pattern)? {
                let path = entry?;
                if path.is_file() {
                    if let Some(file_data) = self.process_file(&path, start_date, end_date)? {
                        files.push(file_data);
                    }
                }
            }
        } else if Path::new(pattern).is_dir() {
            // Handle directory walking
            for entry in WalkDir::new(pattern).follow_links(true).into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "md" || ext == "txt") {
                    if let Some(file_data) = self.process_file(path, start_date, end_date)? {
                        files.push(file_data);
                    }
                }
            }
        } else if Path::new(pattern).is_file() {
            // Handle single file
            if let Some(file_data) = self.process_file(Path::new(pattern), start_date, end_date)? {
                files.push(file_data);
            }
        }

        Ok(files)
    }

    fn process_file(&self, path: &Path, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Option<LocalFileData>> {
        let metadata = fs::metadata(path)?;
        let last_modified = DateTime::from_timestamp(
            metadata.modified()?.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64,
            0
        ).unwrap_or(Utc::now());

        // Only process files modified within our date range or recently
        if last_modified < start_date.checked_sub_signed(chrono::Duration::days(30)).unwrap_or(start_date) {
            return Ok(None);
        }

        let content = fs::read_to_string(path)?;
        let lines: Vec<&str> = content.lines().collect();

        let mut content_snippets = Vec::new();
        let mut time_based_entries = Vec::new();

        // Process each line looking for time patterns and interesting content
        for (line_num, line) in lines.iter().enumerate() {
            // Check for time-based patterns
            for pattern in &self.time_patterns {
                if let Some(_captures) = pattern.captures(line) {
                    if let Some(date) = self.extract_date_from_line(line) {
                        if date >= start_date && date <= end_date {
                            let entry = TimeBasedEntry {
                                date,
                                content: self.extract_context_around_line(&lines, line_num, 3),
                                entry_type: self.classify_entry_type(line),
                            };
                            time_based_entries.push(entry);
                        }
                    }
                }
            }

            // Collect interesting content snippets (headers, lists, etc.)
            if self.is_interesting_line(line) {
                let snippet = ContentSnippet {
                    line_number: line_num + 1,
                    content: line.to_string(),
                    context_lines: self.get_context_lines(&lines, line_num, 2),
                };
                content_snippets.push(snippet);
            }
        }

        // Only include files that have relevant content
        if content_snippets.is_empty() && time_based_entries.is_empty() {
            return Ok(None);
        }

        Ok(Some(LocalFileData {
            file_path: path.to_string_lossy().to_string(),
            file_name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
            last_modified,
            content_snippets,
            time_based_entries,
        }))
    }

    fn extract_date_from_line(&self, line: &str) -> Option<DateTime<Utc>> {
        // Try to extract date from various patterns
        let date_regex = Regex::new(r"(\d{4})-(\d{2})-(\d{2})").ok()?;
        if let Some(captures) = date_regex.captures(line) {
            let year: i32 = captures[1].parse().ok()?;
            let month: u32 = captures[2].parse().ok()?;
            let day: u32 = captures[3].parse().ok()?;
            
            if let Some(naive_date) = chrono::NaiveDate::from_ymd_opt(year, month, day) {
                let naive_datetime = naive_date.and_hms_opt(0, 0, 0)?;
                return Some(DateTime::from_naive_utc_and_offset(naive_datetime, Utc));
            }
        }
        None
    }

    fn classify_entry_type(&self, line: &str) -> String {
        if line.starts_with("# ") {
            "daily_note".to_string()
        } else if line.contains("TODO") || line.contains("TASK") {
            "task_update".to_string()
        } else if line.contains("meeting") || line.contains("Meeting") {
            "meeting_note".to_string()
        } else if line.starts_with("- ") || line.starts_with("* ") {
            "list_item".to_string()
        } else {
            "general_note".to_string()
        }
    }

    fn is_interesting_line(&self, line: &str) -> bool {
        let trimmed = line.trim();
        
        // Headers
        if trimmed.starts_with('#') {
            return true;
        }
        
        // List items
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
            return true;
        }
        
        // Tasks/TODOs
        if trimmed.contains("TODO") || trimmed.contains("FIXME") || trimmed.contains("NOTE") {
            return true;
        }
        
        // Accomplishments or progress indicators
        if trimmed.contains("✅") || trimmed.contains("❌") || trimmed.contains("🔄") {
            return true;
        }
        
        // Important keywords
        let keywords = ["completed", "finished", "started", "blocked", "issue", "problem", "solution", "progress"];
        if keywords.iter().any(|keyword| trimmed.to_lowercase().contains(keyword)) {
            return true;
        }
        
        false
    }

    fn extract_context_around_line(&self, lines: &[&str], line_num: usize, context: usize) -> String {
        let start = line_num.saturating_sub(context);
        let end = (line_num + context + 1).min(lines.len());
        lines[start..end].join("\n")
    }

    fn get_context_lines(&self, lines: &[&str], line_num: usize, context: usize) -> Vec<String> {
        let start = line_num.saturating_sub(context);
        let end = (line_num + context + 1).min(lines.len());
        lines[start..end].iter().map(|s| s.to_string()).collect()
    }
}

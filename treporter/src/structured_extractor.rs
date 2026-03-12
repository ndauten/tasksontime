// Structured data extractor — deductive phase (Phase 2 of monthly-reporting-plan.md)
//
// Takes sanitized commits (from DataPreprocessor::sanitize) and produces
// StructuredData: tf-idf commit clusters + milestone markers extracted from
// the prior month's report.  All logic here is pure Rust and deterministic —
// no LLM involvement.  The LLM (inductive phase) receives this structured
// output and only needs to label clusters and write prose.

use crate::preprocessor::{SanitizedCommit, SanitizedData};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ---- Public output types ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredData {
    pub metadata: StructuredMetadata,
    pub summary_metrics: SummaryMetrics,
    pub commit_clusters: Vec<CommitCluster>,
    pub milestone_markers: Vec<MilestoneMarker>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredMetadata {
    pub total_commits_sanitized: usize,
    pub total_repos: usize,
    pub repos: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryMetrics {
    pub total_additions: u64,
    pub total_deletions: u64,
    pub total_files_touched: usize,
    pub total_mrs_and_prs: usize,
    pub total_issues: usize,
}

/// A cluster of thematically related commits found by tf-idf analysis.
/// The `label` field starts as the top tf-idf term; the LLM overwrites it
/// with a human-readable description in the inductive phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitCluster {
    /// Human-readable name — set by LLM; default = top tf-idf term.
    pub label: String,
    /// Top tf-idf terms characterising all commits in this cluster.
    pub terms: Vec<String>,
    pub commits: Vec<ClusterCommit>,
    /// Representative file paths across commits in this cluster (up to 10).
    pub file_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterCommit {
    pub id: String,
    pub title: String,
    pub author: String,
    pub repo: String,
    pub additions: u64,
    pub deletions: u64,
}

/// A milestone or goal item found in the prior month's report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestoneMarker {
    pub text: String,
    /// One of: COMPLETED, IN_PROGRESS, MISSED, ADDED, UNKNOWN
    pub status_hint: String,
}

// ---- Extractor ----

pub struct StructuredDataExtractor;

impl StructuredDataExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Main entry point for the deductive phase.
    ///
    /// Clusters sanitized commits by tf-idf similarity and extracts milestone
    /// markers from `prior_report` markdown (if provided).
    pub fn extract(&self, sanitized: &SanitizedData, prior_report: Option<&str>) -> StructuredData {
        let commits = &sanitized.commits;

        let repos: Vec<String> = {
            let mut seen = HashSet::new();
            commits
                .iter()
                .filter_map(|c| {
                    if seen.insert(c.repo.clone()) {
                        Some(c.repo.clone())
                    } else {
                        None
                    }
                })
                .collect()
        };

        let total_additions: u64 = commits.iter().map(|c| c.additions).sum();
        let total_deletions: u64 = commits.iter().map(|c| c.deletions).sum();
        let total_files_touched: usize = commits.iter().map(|c| c.files_changed.len()).sum();

        let commit_clusters = self.cluster_commits(commits);
        let milestone_markers = prior_report
            .map(|t| self.extract_milestones(t))
            .unwrap_or_default();

        StructuredData {
            metadata: StructuredMetadata {
                total_commits_sanitized: commits.len(),
                total_repos: repos.len(),
                repos,
            },
            summary_metrics: SummaryMetrics {
                total_additions,
                total_deletions,
                total_files_touched,
                total_mrs_and_prs: sanitized.mrs_and_prs.len(),
                total_issues: sanitized.issues.len(),
            },
            commit_clusters,
            milestone_markers,
        }
    }

    /// Serialise `StructuredData` to pretty-printed JSON.
    pub fn to_json(&self, data: &StructuredData) -> String {
        serde_json::to_string_pretty(data).unwrap_or_else(|_| "{}".to_string())
    }

    // ------------------------------------------------------------------ //
    //  Tf-idf commit clustering                                           //
    // ------------------------------------------------------------------ //

    fn cluster_commits(&self, commits: &[SanitizedCommit]) -> Vec<CommitCluster> {
        if commits.is_empty() {
            return Vec::new();
        }

        // 1. Tokenize all commit titles.
        let tokenized: Vec<Vec<String>> =
            commits.iter().map(|c| tokenize(&c.title)).collect();

        // 2. Compute document-frequency for every term.
        let n = tokenized.len() as f64;
        let mut doc_freq: HashMap<String, usize> = HashMap::new();
        for tokens in &tokenized {
            let unique: HashSet<&String> = tokens.iter().collect();
            for term in unique {
                *doc_freq.entry(term.clone()).or_insert(0) += 1;
            }
        }

        // 3. Assign each commit to its highest tf-idf-scoring term (cluster key).
        let mut cluster_indices: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, tokens) in tokenized.iter().enumerate() {
            if tokens.is_empty() {
                cluster_indices
                    .entry("other".to_string())
                    .or_default()
                    .push(i);
                continue;
            }

            let mut term_count: HashMap<&String, usize> = HashMap::new();
            for t in tokens {
                *term_count.entry(t).or_insert(0) += 1;
            }

            let best_term = term_count
                .iter()
                .map(|(term, &count)| {
                    let tf = count as f64 / tokens.len() as f64;
                    let df = *doc_freq.get(*term).unwrap_or(&1) as f64;
                    let idf = (n / df).ln().max(0.0);
                    (tf * idf, (*term).clone())
                })
                .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(_, term)| term)
                // Fallback: if all scores are 0 (e.g. all terms appear in every doc),
                // group by first token so commits still land somewhere meaningful.
                .unwrap_or_else(|| tokens[0].clone());

            cluster_indices.entry(best_term).or_default().push(i);
        }

        // 4. Build CommitCluster structs.
        let mut clusters: Vec<CommitCluster> = cluster_indices
            .iter()
            .map(|(key, indices)| {
                let cluster_commits: Vec<ClusterCommit> = indices
                    .iter()
                    .map(|&i| {
                        let c = &commits[i];
                        ClusterCommit {
                            id: c.hash.clone(),
                            title: c.title.clone(),
                            author: c.author.clone(),
                            repo: c.repo.clone(),
                            additions: c.additions,
                            deletions: c.deletions,
                        }
                    })
                    .collect();

                // Unique file paths across the cluster, sorted, max 10.
                let mut fp_set: HashSet<String> = HashSet::new();
                for &i in indices {
                    for f in &commits[i].files_changed {
                        fp_set.insert(f.clone());
                    }
                }
                let mut file_paths: Vec<String> = fp_set.into_iter().collect();
                file_paths.sort();
                file_paths.truncate(10);

                // Top 5 terms for the cluster ranked by summed IDF.
                let mut term_scores: HashMap<String, f64> = HashMap::new();
                for &i in indices {
                    for t in &tokenized[i] {
                        let df = *doc_freq.get(t).unwrap_or(&1) as f64;
                        let idf = (n / df).ln().max(0.0);
                        *term_scores.entry(t.clone()).or_insert(0.0) += idf;
                    }
                }
                let mut ranked: Vec<(f64, String)> =
                    term_scores.into_iter().map(|(t, s)| (s, t)).collect();
                ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
                let top_terms: Vec<String> = ranked.into_iter().take(5).map(|(_, t)| t).collect();

                CommitCluster {
                    label: top_terms.first().cloned().unwrap_or_else(|| key.clone()),
                    terms: top_terms,
                    commits: cluster_commits,
                    file_paths,
                }
            })
            .collect();

        // Largest clusters first.
        clusters.sort_by(|a, b| b.commits.len().cmp(&a.commits.len()));
        clusters
    }

    // ------------------------------------------------------------------ //
    //  Milestone extraction from prior report markdown                   //
    // ------------------------------------------------------------------ //

    fn extract_milestones(&self, prior_report: &str) -> Vec<MilestoneMarker> {
        let mut markers: Vec<MilestoneMarker> = Vec::new();

        // Match GFM task-list checkboxes: `- [x] ...` or `- [ ] ...`
        let checkbox_re =
            Regex::new(r"(?im)^\s*[-*]\s*\[(x|X| )\]\s*(.+)$").expect("valid regex");
        // Match status words embedded in a line.
        let status_word_re = Regex::new(
            r"(?i)\b(complet(?:ed?)?|done|finished|achieved|in[- ]?progress|wip|blocked|missed|skipped|added|new)\b",
        )
        .expect("valid regex");

        // Match explicit milestone labels: `**Milestone:** ...` or `### Milestone: ...`
        // Must contain the word "milestone" — does NOT match generic headings.
        let milestone_label_re =
            Regex::new(r"(?im)^(?:#{1,4}\s+)?\*{0,2}[Mm]ilestone\*{0,2}:?\s+(.+)$")
                .expect("valid regex");

        for line in prior_report.lines() {
            if let Some(caps) = checkbox_re.captures(line) {
                let checked = caps.get(1).map_or("", |m| m.as_str());
                let text = caps
                    .get(2)
                    .map_or("", |m| m.as_str())
                    .trim()
                    .to_string();
                let status_hint = if checked.eq_ignore_ascii_case("x") {
                    "COMPLETED"
                } else {
                    "IN_PROGRESS"
                }
                .to_string();
                markers.push(MilestoneMarker { text, status_hint });
            } else if milestone_label_re.is_match(line) {
                if let Some(caps) = milestone_label_re.captures(line) {
                    let text = caps
                        .get(1)
                        .map_or("", |m| m.as_str())
                        .trim()
                        .to_string();
                    if text.is_empty() {
                        continue;
                    }
                    let status_hint = if let Some(m) = status_word_re.find(line) {
                        status_word_to_hint(m.as_str())
                    } else {
                        "UNKNOWN"
                    }
                    .to_string();
                    markers.push(MilestoneMarker { text, status_hint });
                }
            }
        }

        markers
    }
}

// ---- helpers ----

fn status_word_to_hint(word: &str) -> &'static str {
    let lower = word.to_ascii_lowercase();
    if lower.contains("complet") || lower.contains("done") || lower.contains("finish") || lower.contains("achiev") {
        "COMPLETED"
    } else if lower.contains("progress") || lower.contains("wip") {
        "IN_PROGRESS"
    } else if lower.contains("block") || lower.contains("miss") || lower.contains("skip") {
        "MISSED"
    } else if lower.contains("add") || lower.contains("new") {
        "ADDED"
    } else {
        "UNKNOWN"
    }
}

/// Tokenise a string into lowercase alpha terms, filtering stopwords and
/// very short tokens.
fn tokenize(text: &str) -> Vec<String> {
    const STOPWORDS: &[&str] = &[
        "a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for",
        "of", "is", "it", "be", "by", "as", "this", "that", "with", "from",
        "up", "not", "no", "so", "if", "do", "we", "use", "using", "used",
        "via", "into", "also", "when", "then", "was", "are",
    ];

    text.split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 2)
        .map(|s| s.to_lowercase())
        .filter(|s| !STOPWORDS.contains(&s.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preprocessor::{SanitizedCommit, SanitizedData};

    fn make_commit(hash: &str, title: &str, repo: &str) -> SanitizedCommit {
        SanitizedCommit {
            hash: hash.to_string(),
            title: title.to_string(),
            author: "alice".to_string(),
            repo: repo.to_string(),
            full_message: title.to_string(),
            files_changed: vec![],
            additions: 10,
            deletions: 2,
        }
    }

    #[test]
    fn test_cluster_produces_output() {
        let extractor = StructuredDataExtractor::new();
        let data = SanitizedData {
            commits: vec![
                make_commit("aaa", "add authentication middleware", "api"),
                make_commit("bbb", "add jwt token validation", "api"),
                make_commit("ccc", "fix database connection pool", "db"),
                make_commit("ddd", "fix null pointer in database query", "db"),
                make_commit("eee", "update documentation readme", "docs"),
            ],
            mrs_and_prs: vec![],
            issues: vec![],
        };
        let result = extractor.extract(&data, None);
        assert!(!result.commit_clusters.is_empty());
        assert_eq!(result.metadata.total_commits_sanitized, 5);
    }

    #[test]
    fn test_cluster_repos_metadata() {
        let extractor = StructuredDataExtractor::new();
        let data = SanitizedData {
            commits: vec![
                make_commit("aaa", "add feature", "api"),
                make_commit("bbb", "fix bug", "db"),
                make_commit("ccc", "add tests", "api"),
            ],
            mrs_and_prs: vec![],
            issues: vec![],
        };
        let result = extractor.extract(&data, None);
        assert_eq!(result.metadata.total_commits_sanitized, 3);
        // Should detect 2 distinct repos
        assert_eq!(result.metadata.total_repos, 2);
        let mut repos = result.metadata.repos.clone();
        repos.sort();
        assert_eq!(repos, vec!["api", "db"]);
    }

    #[test]
    fn test_all_commits_appear_in_some_cluster() {
        let extractor = StructuredDataExtractor::new();
        let commits = vec![
            make_commit("a1", "implement oauth login", "api"),
            make_commit("a2", "implement refresh token", "api"),
            make_commit("b1", "fix sql injection", "db"),
            make_commit("b2", "fix race condition", "db"),
            make_commit("c1", "update readme docs", "docs"),
        ];
        let commit_ids: Vec<String> = commits.iter().map(|c| c.hash.clone()).collect();
        let data = SanitizedData { commits, mrs_and_prs: vec![], issues: vec![] };
        let result = extractor.extract(&data, None);

        // Every commit id must appear in exactly one cluster
        let mut found_ids: Vec<String> = result.commit_clusters
            .iter()
            .flat_map(|cl| cl.commits.iter().map(|c| c.id.clone()))
            .collect();
        found_ids.sort();
        let mut expected = commit_ids.clone();
        expected.sort();
        assert_eq!(found_ids, expected, "every commit must appear in a cluster");
    }

    #[test]
    fn test_summary_metrics_correct() {
        let extractor = StructuredDataExtractor::new();
        let mut c1 = make_commit("a", "add feature", "repo");
        c1.additions = 100; c1.deletions = 10;
        let mut c2 = make_commit("b", "fix bug", "repo");
        c2.additions = 20; c2.deletions = 5;
        let data = SanitizedData {
            commits: vec![c1, c2],
            mrs_and_prs: vec!["[merged] Add retry logic".to_string()],
            issues: vec!["[closed] Crash on empty input".to_string(), "[open] Slow query".to_string()],
        };
        let result = extractor.extract(&data, None);
        assert_eq!(result.summary_metrics.total_additions, 120);
        assert_eq!(result.summary_metrics.total_deletions, 15);
        assert_eq!(result.summary_metrics.total_mrs_and_prs, 1);
        assert_eq!(result.summary_metrics.total_issues, 2);
    }

    #[test]
    fn test_milestone_extraction_checkboxes() {
        let extractor = StructuredDataExtractor::new();
        let report = "## Goals\n- [x] Deploy new API\n- [ ] Write integration tests\n";
        let markers = extractor.extract_milestones(report);
        assert_eq!(markers.len(), 2);
        assert_eq!(markers[0].status_hint, "COMPLETED");
        assert_eq!(markers[1].status_hint, "IN_PROGRESS");
    }

    #[test]
    fn test_milestone_extraction_explicit_label() {
        let extractor = StructuredDataExtractor::new();
        // "Milestone: <text> — completed" style lines
        let report = "Milestone: Deploy staging environment completed\n\
                      Milestone: Write load tests in progress\n";
        let markers = extractor.extract_milestones(report);
        assert!(!markers.is_empty(), "should find at least one explicit milestone");
        let completed = markers.iter().any(|m| m.status_hint == "COMPLETED");
        assert!(completed, "should detect completed status");
        let in_progress = markers.iter().any(|m| m.status_hint == "IN_PROGRESS");
        assert!(in_progress, "should detect in-progress status");
    }

    #[test]
    fn test_empty_input_produces_empty_clusters() {
        let extractor = StructuredDataExtractor::new();
        let data = SanitizedData { commits: vec![], mrs_and_prs: vec![], issues: vec![] };
        let result = extractor.extract(&data, None);
        assert!(result.commit_clusters.is_empty());
        assert_eq!(result.metadata.total_commits_sanitized, 0);
    }
}

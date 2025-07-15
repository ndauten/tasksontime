#!/usr/bin/env python3
"""
Report generator script using the collected data and template.
Based on the Rust types defined in src/types.rs
"""

import json
import datetime
from typing import Dict, List, Any
from pathlib import Path

def load_data(data_path: str) -> Dict[str, Any]:
    """Load the collected data JSON file"""
    with open(data_path, 'r') as f:
        return json.load(f)

def load_template(template_path: str) -> str:
    """Load the markdown template"""
    with open(template_path, 'r') as f:
        return f.read()

def extract_metrics(data: Dict[str, Any]) -> Dict[str, Any]:
    """Extract key metrics from the collected data"""
    metrics = {}
    
    # GitLab metrics
    gitlab = data.get('gitlab', {})
    gitlab_commits = gitlab.get('commits', [])
    gitlab_merge_requests = gitlab.get('merge_requests', [])
    gitlab_issues = gitlab.get('issues', [])
    gitlab_comments = gitlab.get('comments', [])
    
    # GitHub metrics
    github = data.get('github', {})
    github_commits = github.get('commits', [])
    github_pull_requests = github.get('pull_requests', [])
    github_issues = github.get('issues', [])
    github_comments = github.get('comments', [])
    
    # Local files and git commits
    local_files = data.get('local_files', [])
    git_commits = data.get('git_commits', [])
    
    # Calculate totals
    metrics['git_commits_count'] = len(gitlab_commits) + len(github_commits) + len(git_commits)
    metrics['gitlab_events_count'] = len(gitlab_commits) + len(gitlab_merge_requests) + len(gitlab_issues) + len(gitlab_comments)
    metrics['github_events_count'] = len(github_commits) + len(github_pull_requests) + len(github_issues) + len(github_comments)
    metrics['local_files_count'] = len(local_files)
    
    # Code change metrics
    total_insertions = 0
    total_deletions = 0
    files_changed = set()
    
    # Parse diffs from commits for line count metrics
    for commit in gitlab_commits:
        diff = commit.get('_diff', '')
        if diff:
            lines = diff.split('\n')
            for line in lines:
                if line.startswith('+') and not line.startswith('+++'):
                    total_insertions += 1
                elif line.startswith('-') and not line.startswith('---'):
                    total_deletions += 1
    
    # Count unique repositories
    unique_repos = set()
    for commit in gitlab_commits:
        if commit.get('_project_name'):
            unique_repos.add(commit['_project_name'])
    for commit in github_commits:
        if commit.get('repo', {}).get('name'):
            unique_repos.add(commit['repo']['name'])
    for commit in git_commits:
        if commit.get('repo_path'):
            unique_repos.add(commit['repo_path'])
    
    metrics['total_insertions'] = total_insertions
    metrics['total_deletions'] = total_deletions
    metrics['files_changed_count'] = len(files_changed)
    metrics['unique_repos_count'] = len(unique_repos)
    
    # Metadata
    metadata = data.get('metadata', {})
    metrics['collection_time'] = metadata.get('collection_time', '')
    metrics['date_range_start'] = metadata.get('date_range_start', '')
    metrics['date_range_end'] = metadata.get('date_range_end', '')
    
    return metrics

def format_recent_commits(data: Dict[str, Any], limit: int = 10) -> str:
    """Format recent commits for the report"""
    all_commits = []
    
    # Collect all commits with timestamps
    gitlab_commits = data.get('gitlab', {}).get('commits', [])
    for commit in gitlab_commits:
        all_commits.append({
            'source': 'GitLab',
            'title': commit.get('title', ''),
            'message': commit.get('message', ''),
            'author': commit.get('author_name', ''),
            'date': commit.get('authored_date', ''),
            'project': commit.get('_project_name', ''),
            'short_id': commit.get('short_id', ''),
            'web_url': commit.get('web_url', '')
        })
    
    git_commits = data.get('git_commits', [])
    for commit in git_commits:
        all_commits.append({
            'source': 'Local Git',
            'title': commit.get('message', '').split('\n')[0],
            'message': commit.get('message', ''),
            'author': commit.get('author_name', ''),
            'date': commit.get('timestamp', ''),
            'project': commit.get('repo_path', ''),
            'short_id': commit.get('hash', '')[:8] if commit.get('hash') else '',
            'web_url': ''
        })
    
    # Sort by date (most recent first)
    all_commits.sort(key=lambda x: x['date'], reverse=True)
    
    # Format as markdown
    output = []
    for commit in all_commits[:limit]:
        if commit['web_url']:
            title_link = f"[{commit['title']}]({commit['web_url']})"
        else:
            title_link = commit['title']
        
        output.append(f"- **{commit['project']}** ({commit['source']}) - {title_link}")
        output.append(f"  - Author: {commit['author']}")
        output.append(f"  - Date: {commit['date']}")
        if commit['short_id']:
            output.append(f"  - Commit: `{commit['short_id']}`")
        output.append("")
    
    return '\n'.join(output)

def format_file_updates(data: Dict[str, Any], limit: int = 10) -> str:
    """Format recent file updates for the report"""
    local_files = data.get('local_files', [])
    
    if not local_files:
        return "No local file updates tracked in this period."
    
    # Sort by last_modified
    sorted_files = sorted(local_files, key=lambda x: x.get('last_modified', ''), reverse=True)
    
    output = []
    for file_data in sorted_files[:limit]:
        file_path = file_data.get('file_path', '')
        file_name = file_data.get('file_name', '')
        last_modified = file_data.get('last_modified', '')
        
        output.append(f"- **{file_name}** (`{file_path}`)")
        output.append(f"  - Last Modified: {last_modified}")
        
        # Include content snippets if available
        snippets = file_data.get('content_snippets', [])
        if snippets:
            output.append(f"  - Key Updates: {len(snippets)} content changes tracked")
        
        output.append("")
    
    return '\n'.join(output)

def generate_activity_summary(data: Dict[str, Any]) -> str:
    """Generate an executive summary of activity"""
    metrics = extract_metrics(data)
    
    total_activity = (metrics['git_commits_count'] + 
                     metrics['gitlab_events_count'] + 
                     metrics['github_events_count'] + 
                     metrics['local_files_count'])
    
    summary_parts = []
    
    if metrics['git_commits_count'] > 0:
        summary_parts.append(f"{metrics['git_commits_count']} commits across {metrics['unique_repos_count']} repositories")
    
    if metrics['total_insertions'] > 0 or metrics['total_deletions'] > 0:
        summary_parts.append(f"{metrics['total_insertions']} lines added, {metrics['total_deletions']} lines removed")
    
    if metrics['gitlab_events_count'] > 0:
        summary_parts.append(f"{metrics['gitlab_events_count']} GitLab activities")
    
    if metrics['github_events_count'] > 0:
        summary_parts.append(f"{metrics['github_events_count']} GitHub activities")
    
    if metrics['local_files_count'] > 0:
        summary_parts.append(f"{metrics['local_files_count']} local file updates")
    
    if summary_parts:
        return f"During this reporting period, there were {total_activity} total activities including: " + ", ".join(summary_parts) + "."
    else:
        return "No significant activity detected during this reporting period."

def fill_template(template: str, data: Dict[str, Any]) -> str:
    """Fill the template with data from the collected information"""
    metrics = extract_metrics(data)
    
    # Determine project name and report period
    metadata = data.get('metadata', {})
    start_date = metadata.get('date_range_start', '').split('T')[0] if metadata.get('date_range_start') else 'Unknown'
    end_date = metadata.get('date_range_end', '').split('T')[0] if metadata.get('date_range_end') else 'Unknown'
    
    # Template variables
    template_vars = {
        'project_name': 'SPEAR Project',
        'report_period': f"{start_date} to {end_date}",
        'activity_summary': generate_activity_summary(data),
        'git_commits_count': str(metrics['git_commits_count']),
        'gitlab_events_count': str(metrics['gitlab_events_count']),
        'github_events_count': str(metrics['github_events_count']),
        'local_files_count': str(metrics['local_files_count']),
        'total_insertions': str(metrics['total_insertions']),
        'total_deletions': str(metrics['total_deletions']),
        'files_changed_count': str(metrics['files_changed_count']),
        'unique_repos_count': str(metrics['unique_repos_count']),
        'recent_commits': format_recent_commits(data),
        'file_updates': format_file_updates(data)
    }
    
    # Replace template variables
    filled_template = template
    for var, value in template_vars.items():
        filled_template = filled_template.replace('{{' + var + '}}', value)
    
    return filled_template

def main():
    """Main function to generate the report"""
    base_path = Path(__file__).parent
    data_path = base_path / "collected_data.json"
    template_path = base_path / "templates" / "monthly_report.md"
    output_path = base_path / "generated_report.md"
    
    print("Loading data...")
    data = load_data(str(data_path))
    
    print("Loading template...")
    template = load_template(str(template_path))
    
    print("Generating report...")
    report = fill_template(template, data)
    
    print(f"Saving report to {output_path}...")
    with open(output_path, 'w') as f:
        f.write(report)
    
    print("Report generated successfully!")
    
    # Print some basic stats
    metrics = extract_metrics(data)
    print(f"\nReport Summary:")
    print(f"- Total commits: {metrics['git_commits_count']}")
    print(f"- GitLab events: {metrics['gitlab_events_count']}")
    print(f"- GitHub events: {metrics['github_events_count']}")
    print(f"- Local files: {metrics['local_files_count']}")
    print(f"- Repositories: {metrics['unique_repos_count']}")

if __name__ == "__main__":
    main()

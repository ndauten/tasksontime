#!/usr/bin/env python3

import json
import sys
import os

def merge_git_events(files, output_file):
    """Merge multiple git event JSON files into a single file."""
    merged_data = {
        "gitlab": {"commits": [], "merge_requests": [], "issues": [], "comments": []},
        "github": {"commits": [], "pull_requests": [], "issues": [], "comments": []},
        "local_files": [],
        "git_commits": [],
        "metadata": {
            "collection_date": "2025-Q2",
            "sources": ["gitlab", "github"],
            "date_range": {"start": "2025-04-01", "end": "2025-06-30"}
        }
    }
    
    for file_path in files:
        if not os.path.exists(file_path):
            print(f"Warning: File {file_path} does not exist")
            continue
            
        with open(file_path, 'r') as f:
            try:
                data = json.load(f)
                
                # Merge gitlab commits
                if 'gitlab' in data and 'commits' in data['gitlab']:
                    merged_data['gitlab']['commits'].extend(data['gitlab']['commits'])
                    
                # Merge gitlab merge requests
                if 'gitlab' in data and 'merge_requests' in data['gitlab']:
                    merged_data['gitlab']['merge_requests'].extend(data['gitlab']['merge_requests'])
                
                # Merge github commits
                if 'github' in data and 'commits' in data['github']:
                    merged_data['github']['commits'].extend(data['github']['commits'])
                    
                # Merge github merge requests
                if 'github' in data and 'merge_requests' in data['github']:
                    merged_data['github']['merge_requests'].extend(data['github']['merge_requests'])
                
                # Merge local commits
                if 'local' in data and 'commits' in data['local']:
                    merged_data['local']['commits'].extend(data['local']['commits'])
                    
                # Merge local merge requests
                if 'local' in data and 'merge_requests' in data['local']:
                    merged_data['local']['merge_requests'].extend(data['local']['merge_requests'])
                    
            except json.JSONDecodeError as e:
                print(f"Error reading {file_path}: {e}")
                continue
    
    # Write merged data
    with open(output_file, 'w') as f:
        json.dump(merged_data, f, indent=2)
    
    print(f"Merged {len(files)} files into {output_file}")
    print(f"Total commits: GitLab={len(merged_data['gitlab']['commits'])}, GitHub={len(merged_data['github']['commits'])}, Local={len(merged_data['local']['commits'])}")
    print(f"Total merge requests: GitLab={len(merged_data['gitlab']['merge_requests'])}, GitHub={len(merged_data['github']['merge_requests'])}, Local={len(merged_data['local']['merge_requests'])}")

if __name__ == "__main__":
    # Define Q2 files
    q2_files = [
        "/Users/ndd/haven/thebrain/projects/spear/spear-hq/spear-hq.wiki/reports/2025-Q2/2025-04-git-events.json",
        "/Users/ndd/haven/thebrain/projects/spear/spear-hq/spear-hq.wiki/reports/2025-Q2/2025-05-git-events.json",
        "/Users/ndd/haven/thebrain/projects/spear/spear-hq/spear-hq.wiki/reports/2025-Q2/2025-06-git-events.json"
    ]
    
    output_file = "/Users/ndd/haven/thebrain/projects/spear/tasksontime/treporter/cache/2025-Q2-git-events.json"
    
    merge_git_events(q2_files, output_file)

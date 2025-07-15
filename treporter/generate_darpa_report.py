#!/usr/bin/env python3
"""
Enhanced report generator that analyzes git diffs and code changes to extract
technical accomplishments and categorize them by impact (major/minor).
"""

import json
import re
import datetime
from typing import Dict, List, Any, Tuple
from pathlib import Path
from collections import defaultdict

class CodeAnalyzer:
    """Analyzes code changes to determine impact and categorize accomplishments"""
    
    def __init__(self):
        # Keywords that indicate major vs minor changes
        self.major_indicators = {
            'architecture': ['refactor', 'redesign', 'architecture', 'framework', 'infrastructure'],
            'new_features': ['add', 'implement', 'create', 'new', 'feature', 'support'],
            'performance': ['optimize', 'performance', 'speed', 'memory', 'efficiency'],
            'security': ['security', 'vulnerability', 'fix', 'cve', 'auth', 'permission'],
            'integration': ['integrate', 'api', 'interface', 'protocol', 'format'],
            'tooling': ['tool', 'cli', 'script', 'automation', 'build', 'deploy']
        }
        
        self.minor_indicators = [
            'fix', 'bug', 'typo', 'cleanup', 'format', 'style', 'comment', 
            'doc', 'readme', 'minor', 'small', 'tweak', 'adjust'
        ]
        
        # File patterns that indicate different types of work
        self.file_patterns = {
            'source': ['.rs', '.py', '.c', '.cpp', '.h', '.js', '.ts'],
            'config': ['.toml', '.yaml', '.yml', '.json', '.xml', '.conf'],
            'build': ['Makefile', 'Cargo.toml', 'package.json', '.gitlab-ci.yml', '.github/'],
            'docs': ['.md', '.txt', '.rst', '.adoc'],
            'tests': ['test', 'spec', '_test.py', '_test.rs']
        }

    def analyze_commit(self, commit: Dict[str, Any]) -> Dict[str, Any]:
        """Analyze a single commit to extract technical significance"""
        title = commit.get('title', '').lower()
        message = commit.get('message', '').lower()
        diff = commit.get('_diff', '')
        
        analysis = {
            'title': commit.get('title', ''),
            'message': commit.get('message', ''),
            'author': commit.get('author_name', ''),
            'date': commit.get('authored_date', ''),
            'project': commit.get('_project_name', ''),
            'short_id': commit.get('short_id', ''),
            'web_url': commit.get('web_url', ''),
            'impact': 'minor',
            'categories': [],
            'technical_details': [],
            'files_affected': [],
            'lines_changed': {'added': 0, 'removed': 0}
        }
        
        # Analyze diff for technical content
        if diff:
            analysis.update(self._analyze_diff(diff))
        
        # Categorize by impact
        analysis['impact'] = self._determine_impact(title, message, analysis['files_affected'], analysis['lines_changed'])
        analysis['categories'] = self._categorize_change(title, message, analysis['files_affected'])
        
        return analysis

    def _analyze_diff(self, diff: str) -> Dict[str, Any]:
        """Extract technical details from git diff"""
        lines = diff.split('\n')
        
        files_affected = []
        lines_added = 0
        lines_removed = 0
        technical_details = []
        
        current_file = None
        
        for line in lines:
            # Track file changes
            if line.startswith('--- ') or line.startswith('+++ '):
                if line.startswith('--- ') and not line.endswith('/dev/null'):
                    current_file = line[4:].strip()
                elif line.startswith('+++ ') and not line.endswith('/dev/null'):
                    current_file = line[4:].strip()
                    if current_file and current_file not in files_affected:
                        files_affected.append(current_file)
            
            # Count line changes
            elif line.startswith('+') and not line.startswith('+++'):
                lines_added += 1
                # Look for significant additions
                if any(keyword in line.lower() for keyword in ['struct', 'class', 'function', 'fn ', 'def ', 'impl']):
                    technical_details.append(f"New code structure: {line.strip()}")
            elif line.startswith('-') and not line.startswith('---'):
                lines_removed += 1
            
            # Look for important patterns
            if current_file:
                if 'cargo.toml' in current_file.lower() and ('+' in line and 'dependencies' in line.lower()):
                    technical_details.append(f"New dependency: {line.strip()}")
                elif '.yml' in current_file.lower() and '+' in line and ('stage' in line or 'job' in line):
                    technical_details.append(f"CI/CD change: {line.strip()}")
        
        return {
            'files_affected': files_affected,
            'lines_changed': {'added': lines_added, 'removed': lines_removed},
            'technical_details': technical_details
        }

    def _determine_impact(self, title: str, message: str, files: List[str], lines: Dict[str, int]) -> str:
        """Determine if change is major or minor"""
        text = f"{title} {message}".lower()
        
        # Major indicators
        for category, keywords in self.major_indicators.items():
            if any(keyword in text for keyword in keywords):
                return 'major'
        
        # Large code changes
        if lines['added'] + lines['removed'] > 100:
            return 'major'
        
        # Multiple files affected
        if len(files) > 5:
            return 'major'
        
        # Core system files
        core_files = ['main.rs', 'lib.rs', 'mod.rs', 'Cargo.toml', 'Makefile']
        if any(any(core in f for core in core_files) for f in files):
            return 'major'
        
        # Minor indicators
        if any(indicator in text for indicator in self.minor_indicators):
            return 'minor'
        
        # Default to minor for small changes
        return 'minor' if lines['added'] + lines['removed'] < 20 else 'major'

    def _categorize_change(self, title: str, message: str, files: List[str]) -> List[str]:
        """Categorize the type of change"""
        text = f"{title} {message}".lower()
        categories = []
        
        for category, keywords in self.major_indicators.items():
            if any(keyword in text for keyword in keywords):
                categories.append(category)
        
        # File-based categorization
        for file in files:
            file_lower = file.lower()
            if any(ext in file_lower for ext in self.file_patterns['source']):
                categories.append('source_code')
            elif any(ext in file_lower for ext in self.file_patterns['config']):
                categories.append('configuration')
            elif any(pattern in file_lower for pattern in self.file_patterns['build']):
                categories.append('build_system')
            elif any(ext in file_lower for ext in self.file_patterns['docs']):
                categories.append('documentation')
            elif any(pattern in file_lower for pattern in self.file_patterns['tests']):
                categories.append('testing')
        
        return list(set(categories))

class DARPAReportGenerator:
    """Generates DARPA quarterly report from analyzed data"""
    
    def __init__(self, analyzer: CodeAnalyzer):
        self.analyzer = analyzer
    
    def generate_report(self, data: Dict[str, Any], template_path: str) -> str:
        """Generate complete DARPA report"""
        # Analyze all commits
        all_commits = self._collect_all_commits(data)
        analyzed_commits = [self.analyzer.analyze_commit(commit) for commit in all_commits]
        
        # Filter and categorize
        major_accomplishments = [c for c in analyzed_commits if c['impact'] == 'major']
        minor_accomplishments = [c for c in analyzed_commits if c['impact'] == 'minor']
        
        # Group by time periods
        june_commits = self._filter_by_month(analyzed_commits, 2025, 6)
        q2_commits = self._filter_by_quarter(analyzed_commits, 2025, 2)
        
        # Load template
        with open(template_path, 'r') as f:
            template = f.read()
        
        # Fill template sections
        filled_template = self._fill_template(template, {
            'all_commits': analyzed_commits,
            'major_accomplishments': major_accomplishments,
            'minor_accomplishments': minor_accomplishments,
            'june_commits': june_commits,
            'q2_commits': q2_commits,
            'metadata': data.get('metadata', {})
        })
        
        return filled_template
    
    def _collect_all_commits(self, data: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Collect all commits from all sources"""
        all_commits = []
        
        # GitLab commits
        gitlab_commits = data.get('gitlab', {}).get('commits', [])
        for commit in gitlab_commits:
            commit['_source'] = 'gitlab'
            all_commits.append(commit)
        
        # GitHub commits  
        github_commits = data.get('github', {}).get('commits', [])
        for commit in github_commits:
            commit['_source'] = 'github'
            all_commits.append(commit)
        
        # Local git commits
        git_commits = data.get('git_commits', [])
        for commit in git_commits:
            # Convert to same format as GitLab/GitHub
            commit['_source'] = 'local_git'
            commit['title'] = commit.get('message', '').split('\n')[0]
            commit['authored_date'] = commit.get('timestamp', '')
            commit['short_id'] = commit.get('hash', '')[:8] if commit.get('hash') else ''
            commit['_project_name'] = commit.get('repo_path', '')
            all_commits.append(commit)
        
        return all_commits
    
    def _filter_by_month(self, commits: List[Dict], year: int, month: int) -> List[Dict]:
        """Filter commits by specific month"""
        filtered = []
        for commit in commits:
            date_str = commit.get('date', '')
            if date_str:
                try:
                    # Parse various date formats
                    if 'T' in date_str:
                        date_obj = datetime.datetime.fromisoformat(date_str.replace('Z', '+00:00'))
                    else:
                        date_obj = datetime.datetime.fromisoformat(date_str)
                    
                    if date_obj.year == year and date_obj.month == month:
                        filtered.append(commit)
                except:
                    continue
        return filtered
    
    def _filter_by_quarter(self, commits: List[Dict], year: int, quarter: int) -> List[Dict]:
        """Filter commits by quarter (Q2 = months 4,5,6)"""
        months = {1: [1,2,3], 2: [4,5,6], 3: [7,8,9], 4: [10,11,12]}
        target_months = months.get(quarter, [])
        
        filtered = []
        for commit in commits:
            date_str = commit.get('date', '')
            if date_str:
                try:
                    if 'T' in date_str:
                        date_obj = datetime.datetime.fromisoformat(date_str.replace('Z', '+00:00'))
                    else:
                        date_obj = datetime.datetime.fromisoformat(date_str)
                    
                    if date_obj.year == year and date_obj.month in target_months:
                        filtered.append(commit)
                except:
                    continue
        return filtered
    
    def _fill_template(self, template: str, analysis_data: Dict) -> str:
        """Fill the DARPA template with analyzed data"""
        filled = template
        
        # Replace date fields
        today = datetime.date.today()
        filled = filled.replace('YYYY-MM-DD', today.strftime('%Y-%m-%d'))
        
        # Section 1: June Technical Accomplishments
        june_accomplishments = self._format_accomplishments(
            analysis_data['june_commits'], 
            limit=2, 
            priority='major'
        )
        filled = filled.replace(
            '- [LLM_FILL: Accomplishment 1 summary + source refs]\n- [LLM_FILL: Accomplishment 2 summary + source refs]',
            june_accomplishments
        )
        
        # Section 3: Q2 Objective 8 Status (Static Analysis & Interchange Format)
        objective8_status = self._analyze_objective8_progress(analysis_data['q2_commits'])
        filled = self._replace_objective8_section(filled, objective8_status)
        
        # Section 4: Q2 Technical Accomplishments
        q2_accomplishments = self._format_accomplishments(
            analysis_data['q2_commits'],
            limit=5,
            show_categories=True
        )
        filled = self._replace_section_4(filled, q2_accomplishments)
        
        # Section 5: Prototype Improvements
        prototype_improvements = self._analyze_prototype_improvements(analysis_data['q2_commits'])
        filled = self._replace_section_5(filled, prototype_improvements)
        
        return filled
    
    def _format_accomplishments(self, commits: List[Dict], limit: int = 10, priority: str = None, show_categories: bool = False) -> str:
        """Format accomplishments for the report"""
        if priority:
            commits = [c for c in commits if c['impact'] == priority]
        
        # Sort by impact and date
        commits.sort(key=lambda x: (x['impact'] == 'minor', x['date']), reverse=True)
        
        formatted = []
        for commit in commits[:limit]:
            impact_emoji = "🚀" if commit['impact'] == 'major' else "🔧"
            categories_str = f" [{', '.join(commit['categories'])}]" if show_categories and commit['categories'] else ""
            
            if commit.get('web_url'):
                title_link = f"[{commit['title']}]({commit['web_url']})"
            else:
                title_link = commit['title']
            
            accomplishment = f"- **{impact_emoji} {commit['impact'].upper()}:** {title_link}{categories_str}"
            accomplishment += f"\n  - **Project:** {commit['project']}"
            accomplishment += f"\n  - **Author:** {commit['author']}"
            
            if commit['technical_details']:
                accomplishment += f"\n  - **Technical Impact:** {'; '.join(commit['technical_details'][:2])}"
            
            if commit['lines_changed']['added'] + commit['lines_changed']['removed'] > 0:
                accomplishment += f"\n  - **Code Changes:** +{commit['lines_changed']['added']} -{commit['lines_changed']['removed']} lines"
            
            formatted.append(accomplishment)
        
        return '\n\n'.join(formatted) if formatted else "- No significant accomplishments identified in this period"
    
    def _analyze_objective8_progress(self, q2_commits: List[Dict]) -> Dict[str, Any]:
        """Analyze progress on Objective 8: Static Analysis & Interchange Format"""
        relevant_commits = []
        
        for commit in q2_commits:
            text = f"{commit['title']} {commit['message']}".lower()
            # Look for static analysis, interchange format, least privilege keywords
            if any(keyword in text for keyword in [
                'static', 'analysis', 'interchange', 'format', 'privilege', 
                'cpm', 'if', 'export', 'analyzer', 'parser'
            ]):
                relevant_commits.append(commit)
        
        accomplished = len([c for c in relevant_commits if c['impact'] == 'major']) > 0
        
        return {
            'accomplished': accomplished,
            'evidence': relevant_commits[:3],  # Top 3 most relevant
            'notes': self._generate_objective8_notes(relevant_commits, accomplished)
        }
    
    def _generate_objective8_notes(self, commits: List[Dict], accomplished: bool) -> str:
        """Generate notes for Objective 8"""
        if accomplished:
            return f"Significant progress made with {len(commits)} related commits focusing on static analysis and interchange format development."
        else:
            if commits:
                return f"Partial progress with {len(commits)} commits. Additional work needed to complete static analysis integration with interchange format export."
            else:
                return "No significant progress identified. May need to prioritize static analysis work in Q3."
    
    def _replace_objective8_section(self, template: str, status: Dict) -> str:
        """Replace the Objective 8 section in template"""
        accomplished_text = "Yes" if status['accomplished'] else "No"
        
        evidence_text = ""
        for commit in status['evidence']:
            if commit.get('web_url'):
                evidence_text += f"  - [{commit['title']}]({commit['web_url']}) - {commit['project']}\n"
            else:
                evidence_text += f"  - {commit['title']} - {commit['project']}\n"
        
        if not evidence_text:
            evidence_text = "  - No specific commits identified for this objective\n"
        
        # Replace the objective status
        pattern = r'- \*\*Objective Accomplished:\*\* \[Yes / No\]'
        replacement = f"- **Objective Accomplished:** {accomplished_text}"
        template = re.sub(pattern, replacement, template)
        
        # Replace supporting evidence
        pattern = r'- \*\*Supporting Evidence:\*\*\s+- \[LLM_FILL: link\(s\) to commits/MRs \+ brief explanation\]'
        replacement = f"- **Supporting Evidence:**\n{evidence_text.rstrip()}"
        template = re.sub(pattern, replacement, template, flags=re.MULTILINE)
        
        # Replace notes
        pattern = r'- \*\*Notes if not accomplished:\*\*\s+- \[LLM_FILL: in-progress tasks, blockers, expected completion date\]'
        replacement = f"- **Notes if not accomplished:**\n  - {status['notes']}"
        template = re.sub(pattern, replacement, template, flags=re.MULTILINE)
        
        return template
    
    def _replace_section_4(self, template: str, accomplishments: str) -> str:
        """Replace Section 4: Q2 Technical Accomplishments"""
        pattern = r'- \[LLM_FILL: Description 1 \+ links to task, MR, result\]\s*\n- \[LLM_FILL: Description 2 \+ notes on complexity/value\]'
        template = re.sub(pattern, accomplishments, template, flags=re.MULTILINE)
        return template
    
    def _analyze_prototype_improvements(self, q2_commits: List[Dict]) -> List[Dict]:
        """Analyze prototype improvements from commits"""
        prototypes = defaultdict(list)
        
        for commit in q2_commits:
            project = commit['project']
            if project and commit['impact'] == 'major':
                prototypes[project].append(commit)
        
        improvements = []
        for project, commits in prototypes.items():
            if len(commits) >= 2:  # Only include if multiple significant changes
                improvements.append({
                    'name': project,
                    'commits': commits,
                    'enhancements': self._extract_enhancements(commits),
                    'metrics': self._extract_metrics(commits)
                })
        
        return improvements
    
    def _extract_enhancements(self, commits: List[Dict]) -> str:
        """Extract enhancement descriptions from commits"""
        enhancements = []
        for commit in commits[:3]:  # Top 3
            categories = commit.get('categories', [])
            if 'new_features' in categories:
                enhancements.append(f"New functionality: {commit['title']}")
            elif 'performance' in categories:
                enhancements.append(f"Performance improvement: {commit['title']}")
            elif 'architecture' in categories:
                enhancements.append(f"Architectural enhancement: {commit['title']}")
        
        return '; '.join(enhancements) if enhancements else "Multiple code improvements and feature additions"
    
    def _extract_metrics(self, commits: List[Dict]) -> str:
        """Extract quantitative metrics from commits"""
        total_lines_added = sum(c['lines_changed']['added'] for c in commits)
        total_lines_removed = sum(c['lines_changed']['removed'] for c in commits)
        total_files = len(set(f for c in commits for f in c.get('files_affected', [])))
        
        return f"{len(commits)} major commits, {total_lines_added} lines added, {total_files} files modified"
    
    def _replace_section_5(self, template: str, improvements: List[Dict]) -> str:
        """Replace Section 5: Prototype Improvements"""
        if not improvements:
            replacement = "- **No major prototype improvements identified in this period**"
        else:
            formatted_improvements = []
            for improvement in improvements:
                formatted = f"- **Prototype:** {improvement['name']}\n"
                formatted += f"  - **Enhancements:** {improvement['enhancements']}\n"
                formatted += f"  - **Metrics or Benchmarks:** {improvement['metrics']}"
                formatted_improvements.append(formatted)
            replacement = '\n\n'.join(formatted_improvements)
        
        pattern = r'- \*\*Prototype:\*\* \[LLM_FILL: name\]\s+- \*\*Enhancements:\*\* \[LLM_FILL: e\.g\. support for feature X, performance increase\]\s+- \*\*Metrics or Benchmarks:\*\* \[LLM_FILL: e\.g\., 3x faster, 90% test coverage\]\s+\(Repeat for each prototype\)'
        template = re.sub(pattern, replacement, template, flags=re.MULTILINE | re.DOTALL)
        
        return template

def main():
    """Main function"""
    base_path = Path(__file__).parent
    data_path = base_path / "collected_data.json"
    template_path = Path("/Users/ndd/haven/thebrain/projects/spear/spear-hq/spear-hq.wiki/reports/templates/cpm-spear-monthly.md")
    output_path = base_path / "darpa_quarterly_report.md"
    
    print("🔍 Loading and analyzing data...")
    with open(data_path) as f:
        data = json.load(f)
    
    print("🧠 Analyzing code changes and categorizing impact...")
    analyzer = CodeAnalyzer()
    generator = DARPAReportGenerator(analyzer)
    
    print("📝 Generating DARPA quarterly report...")
    report = generator.generate_report(data, str(template_path))
    
    print(f"💾 Saving report to {output_path}...")
    with open(output_path, 'w') as f:
        f.write(report)
    
    print("✅ DARPA quarterly report generated successfully!")
    
    # Print summary stats
    all_commits = generator._collect_all_commits(data)
    analyzed = [analyzer.analyze_commit(c) for c in all_commits]
    major_count = len([c for c in analyzed if c['impact'] == 'major'])
    minor_count = len([c for c in analyzed if c['impact'] == 'minor'])
    
    print(f"\n📊 Analysis Summary:")
    print(f"- Total commits analyzed: {len(analyzed)}")
    print(f"- Major technical accomplishments: {major_count}")
    print(f"- Minor improvements: {minor_count}")
    print(f"- June-specific commits: {len(generator._filter_by_month(analyzed, 2025, 6))}")
    print(f"- Q2 commits: {len(generator._filter_by_quarter(analyzed, 2025, 2))}")

if __name__ == "__main__":
    main()

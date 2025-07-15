#!/usr/bin/env python3
"""
Enhanced DARPA report generator that combines rule-based analysis with LLM meaning-making
for more nuanced technical accomplishment descriptions.
"""

import json
import re
import datetime
from typing import Dict, List, Any, Tuple
from pathlib import Path
from collections import defaultdict
import openai
import os

class LLMAnalyzer:
    """Uses LLM to provide nuanced analysis of technical accomplishments"""
    
    def __init__(self):
        # Try to get OpenAI API key from environment
        self.client = None
        api_key = os.getenv('OPENAI_API_KEY')
        if api_key:
            self.client = openai.OpenAI(api_key=api_key)
        else:
            print("⚠️  No OpenAI API key found. Will use rule-based analysis only.")
    
    def analyze_technical_significance(self, commits: List[Dict[str, Any]], context: str = "") -> str:
        """Use LLM to analyze the technical significance of a group of commits"""
        if not self.client:
            return self._fallback_analysis(commits)
        
        # Prepare commit data for LLM
        commit_summaries = []
        for commit in commits[:5]:  # Limit to top 5 for API efficiency
            summary = {
                'title': commit.get('title', ''),
                'project': commit.get('project', ''),
                'files_changed': len(commit.get('files_affected', [])),
                'lines_added': commit.get('lines_changed', {}).get('added', 0),
                'categories': commit.get('categories', []),
                'technical_details': commit.get('technical_details', [])
            }
            commit_summaries.append(summary)
        
        prompt = self._build_analysis_prompt(commit_summaries, context)
        
        try:
            response = self.client.chat.completions.create(
                model="gpt-4",
                messages=[
                    {"role": "system", "content": "You are a technical analyst specializing in software engineering and research project assessment. Provide concise, professional analysis focusing on technical merit and research impact."},
                    {"role": "user", "content": prompt}
                ],
                max_tokens=300,
                temperature=0.3
            )
            return response.choices[0].message.content.strip()
        except Exception as e:
            print(f"⚠️  LLM analysis failed: {e}")
            return self._fallback_analysis(commits)
    
    def _build_analysis_prompt(self, commits: List[Dict], context: str) -> str:
        """Build a prompt for LLM analysis"""
        prompt = f"Analyze these technical commits for a DARPA research project report:\n\n"
        
        if context:
            prompt += f"Context: {context}\n\n"
        
        prompt += "Commits:\n"
        for i, commit in enumerate(commits, 1):
            prompt += f"{i}. **{commit['title']}** (Project: {commit['project']})\n"
            prompt += f"   - Files: {commit['files_changed']}, Lines: +{commit['lines_added']}\n"
            if commit['categories']:
                prompt += f"   - Categories: {', '.join(commit['categories'])}\n"
            if commit['technical_details']:
                prompt += f"   - Technical: {'; '.join(commit['technical_details'][:2])}\n"
            prompt += "\n"
        
        prompt += """Please provide a 2-3 sentence analysis focusing on:
1. The technical significance and research impact
2. How these changes advance the overall project goals
3. Any architectural or methodological innovations

Keep the response professional and suitable for a DARPA quarterly report."""
        
        return prompt
    
    def _fallback_analysis(self, commits: List[Dict[str, Any]]) -> str:
        """Fallback analysis when LLM is not available"""
        if not commits:
            return "No significant technical activity identified in this period."
        
        major_count = len([c for c in commits if c.get('impact') == 'major'])
        projects = list(set(c.get('project', 'Unknown') for c in commits))
        total_lines = sum(c.get('lines_changed', {}).get('added', 0) for c in commits)
        
        analysis = f"Significant development activity across {len(projects)} projects with {major_count} major technical contributions. "
        analysis += f"Total of {total_lines} lines of code added, indicating substantial implementation work. "
        
        if len(projects) > 1:
            analysis += f"Multi-project coordination demonstrates integrated approach to research objectives."
        
        return analysis

    def analyze_objective_progress(self, objective_name: str, commits: List[Dict[str, Any]]) -> Dict[str, str]:
        """Analyze progress on a specific DARPA objective"""
        if not self.client:
            return self._fallback_objective_analysis(objective_name, commits)
        
        prompt = f"""Analyze progress on DARPA objective: "{objective_name}"

Based on these commits:
"""
        for commit in commits[:3]:
            prompt += f"- {commit.get('title', '')}\n"
            if commit.get('technical_details'):
                prompt += f"  Technical: {'; '.join(commit['technical_details'][:2])}\n"
        
        prompt += """
Determine:
1. Is this objective accomplished? (Yes/No)
2. Provide 1-2 sentences of supporting evidence or explanation
3. If not accomplished, what are the next steps?

Format as: 
ACCOMPLISHED: Yes/No
EVIDENCE: [explanation]
NOTES: [next steps if needed]"""
        
        try:
            response = self.client.chat.completions.create(
                model="gpt-4",
                messages=[
                    {"role": "system", "content": "You are evaluating DARPA research objective completion. Be precise and evidence-based."},
                    {"role": "user", "content": prompt}
                ],
                max_tokens=200,
                temperature=0.2
            )
            
            result = response.choices[0].message.content.strip()
            return self._parse_objective_response(result)
            
        except Exception as e:
            print(f"⚠️  LLM objective analysis failed: {e}")
            return self._fallback_objective_analysis(objective_name, commits)
    
    def _parse_objective_response(self, response: str) -> Dict[str, str]:
        """Parse LLM response for objective analysis"""
        lines = response.split('\n')
        result = {'accomplished': 'No', 'evidence': '', 'notes': ''}
        
        for line in lines:
            if line.startswith('ACCOMPLISHED:'):
                result['accomplished'] = 'Yes' if 'yes' in line.lower() else 'No'
            elif line.startswith('EVIDENCE:'):
                result['evidence'] = line.replace('EVIDENCE:', '').strip()
            elif line.startswith('NOTES:'):
                result['notes'] = line.replace('NOTES:', '').strip()
        
        return result
    
    def _fallback_objective_analysis(self, objective_name: str, commits: List[Dict[str, Any]]) -> Dict[str, str]:
        """Fallback objective analysis"""
        relevant_commits = len(commits)
        major_commits = len([c for c in commits if c.get('impact') == 'major'])
        
        accomplished = 'Yes' if major_commits >= 2 else 'No'
        evidence = f"{relevant_commits} related commits identified with {major_commits} major contributions."
        notes = "Continue development work to fully complete objective." if accomplished == 'No' else ""
        
        return {
            'accomplished': accomplished,
            'evidence': evidence,
            'notes': notes
        }

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
        
        # Minor indicators
        if any(indicator in text for indicator in self.minor_indicators):
            return 'minor'
        
        return 'minor' if lines['added'] + lines['removed'] < 20 else 'major'

    def _categorize_change(self, title: str, message: str, files: List[str]) -> List[str]:
        """Categorize the type of change"""
        text = f"{title} {message}".lower()
        categories = []
        
        for category, keywords in self.major_indicators.items():
            if any(keyword in text for keyword in keywords):
                categories.append(category)
        
        return list(set(categories))

class EnhancedDARPAReportGenerator:
    """Generates DARPA quarterly report with LLM-enhanced analysis"""
    
    def __init__(self, code_analyzer: CodeAnalyzer, llm_analyzer: LLMAnalyzer):
        self.code_analyzer = code_analyzer
        self.llm_analyzer = llm_analyzer
    
    def generate_report(self, data: Dict[str, Any], template_path: str) -> str:
        """Generate complete DARPA report with LLM enhancements"""
        print("🔍 Analyzing commits with code analysis...")
        all_commits = self._collect_all_commits(data)
        analyzed_commits = [self.code_analyzer.analyze_commit(commit) for commit in all_commits]
        
        print("🧠 Enhancing analysis with LLM...")
        # Group commits for LLM analysis
        june_commits = self._filter_by_month(analyzed_commits, 2025, 6)
        q2_commits = self._filter_by_quarter(analyzed_commits, 2025, 2)
        major_june = [c for c in june_commits if c['impact'] == 'major'][:2]
        
        # Get LLM analysis for key sections
        june_analysis = self.llm_analyzer.analyze_technical_significance(
            major_june, 
            "June 2025 technical accomplishments for DARPA quarterly report"
        )
        
        q2_analysis = self.llm_analyzer.analyze_technical_significance(
            [c for c in q2_commits if c['impact'] == 'major'][:5],
            "Q2 2025 overall technical progress"
        )
        
        # Analyze Objective 8 specifically
        objective8_commits = self._filter_objective8_commits(q2_commits)
        objective8_analysis = self.llm_analyzer.analyze_objective_progress(
            "Static Analysis Producing Least Privilege Interchange Format",
            objective8_commits
        )
        
        print("📝 Filling template...")
        # Load and fill template
        with open(template_path, 'r') as f:
            template = f.read()
        
        filled_template = self._fill_template_with_llm_analysis(template, {
            'all_commits': analyzed_commits,
            'june_commits': june_commits,
            'q2_commits': q2_commits,
            'major_june': major_june,
            'june_analysis': june_analysis,
            'q2_analysis': q2_analysis,
            'objective8_analysis': objective8_analysis,
            'objective8_commits': objective8_commits,
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
        """Filter commits by quarter"""
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
    
    def _filter_objective8_commits(self, commits: List[Dict]) -> List[Dict]:
        """Filter commits relevant to Objective 8"""
        relevant = []
        for commit in commits:
            text = f"{commit['title']} {commit['message']}".lower()
            if any(keyword in text for keyword in [
                'static', 'analysis', 'interchange', 'format', 'privilege', 
                'cpm', 'if', 'export', 'analyzer', 'parser'
            ]):
                relevant.append(commit)
        return relevant
    
    def _fill_template_with_llm_analysis(self, template: str, analysis_data: Dict) -> str:
        """Fill template with LLM-enhanced analysis"""
        filled = template
        
        # Replace date
        today = datetime.date.today()
        filled = filled.replace('YYYY-MM-DD', today.strftime('%Y-%m-%d'))
        
        # Section 1: June Accomplishments (LLM-enhanced)
        june_accomplishments = self._format_llm_accomplishments(
            analysis_data['major_june'], 
            analysis_data['june_analysis']
        )
        filled = filled.replace(
            '- [LLM_FILL: Accomplishment 1 summary + source refs]\n- [LLM_FILL: Accomplishment 2 summary + source refs]',
            june_accomplishments
        )
        
        # Section 3: Objective 8 (LLM analysis)
        filled = self._replace_objective8_with_llm(filled, analysis_data['objective8_analysis'], analysis_data['objective8_commits'])
        
        # Section 4: Q2 Technical Accomplishments (LLM-enhanced)
        q2_accomplishments = self._format_q2_accomplishments(
            analysis_data['q2_commits'], 
            analysis_data['q2_analysis']
        )
        filled = self._replace_section_4_with_llm(filled, q2_accomplishments)
        
        return filled
    
    def _format_llm_accomplishments(self, commits: List[Dict], llm_analysis: str) -> str:
        """Format accomplishments with LLM analysis"""
        if not commits:
            return "- No major technical accomplishments identified in June 2025"
        
        formatted = f"**LLM Analysis:** {llm_analysis}\n\n"
        
        for i, commit in enumerate(commits, 1):
            if commit.get('web_url'):
                title_link = f"[{commit['title']}]({commit['web_url']})"
            else:
                title_link = commit['title']
            
            formatted += f"- **Accomplishment {i}:** {title_link}\n"
            formatted += f"  - **Project:** {commit['project']}\n"
            formatted += f"  - **Impact:** {commit['impact'].upper()} - {commit['lines_changed']['added']}+ lines added\n"
            if commit['categories']:
                formatted += f"  - **Categories:** {', '.join(commit['categories'])}\n"
        
        return formatted
    
    def _replace_objective8_with_llm(self, template: str, analysis: Dict[str, str], commits: List[Dict]) -> str:
        """Replace Objective 8 section with LLM analysis"""
        # Replace accomplished status
        pattern = r'- \*\*Objective Accomplished:\*\* \[Yes / No\]'
        replacement = f"- **Objective Accomplished:** {analysis['accomplished']}"
        template = re.sub(pattern, replacement, template)
        
        # Replace evidence
        evidence_text = f"  - **LLM Analysis:** {analysis['evidence']}\n"
        for commit in commits[:3]:
            if commit.get('web_url'):
                evidence_text += f"  - [{commit['title']}]({commit['web_url']}) - {commit['project']}\n"
            else:
                evidence_text += f"  - {commit['title']} - {commit['project']}\n"
        
        pattern = r'- \*\*Supporting Evidence:\*\*\s+- \[LLM_FILL: link\(s\) to commits/MRs \+ brief explanation\]'
        replacement = f"- **Supporting Evidence:**\n{evidence_text.rstrip()}"
        template = re.sub(pattern, replacement, template, flags=re.MULTILINE)
        
        # Replace notes
        pattern = r'- \*\*Notes if not accomplished:\*\*\s+- \[LLM_FILL: in-progress tasks, blockers, expected completion date\]'
        replacement = f"- **Notes if not accomplished:**\n  - {analysis['notes']}"
        template = re.sub(pattern, replacement, template, flags=re.MULTILINE)
        
        return template
    
    def _format_q2_accomplishments(self, commits: List[Dict], llm_analysis: str) -> str:
        """Format Q2 accomplishments with LLM insights"""
        major_commits = [c for c in commits if c['impact'] == 'major'][:5]
        
        formatted = f"**LLM Analysis:** {llm_analysis}\n\n"
        
        for commit in major_commits:
            if commit.get('web_url'):
                title_link = f"[{commit['title']}]({commit['web_url']})"
            else:
                title_link = commit['title']
            
            formatted += f"- **{commit['project']}:** {title_link}\n"
            formatted += f"  - Categories: {', '.join(commit['categories']) if commit['categories'] else 'Core Development'}\n"
            formatted += f"  - Impact: {commit['lines_changed']['added']} lines added\n\n"
        
        return formatted
    
    def _replace_section_4_with_llm(self, template: str, accomplishments: str) -> str:
        """Replace Section 4 with LLM-enhanced content"""
        pattern = r'- \[LLM_FILL: Description 1 \+ links to task, MR, result\]\s*\n- \[LLM_FILL: Description 2 \+ notes on complexity/value\]'
        template = re.sub(pattern, accomplishments, template, flags=re.MULTILINE)
        return template

def main():
    """Main function"""
    base_path = Path(__file__).parent
    data_path = base_path / "collected_data.json"
    template_path = Path("/Users/ndd/haven/thebrain/projects/spear/spear-hq/spear-hq.wiki/reports/templates/cpm-spear-monthly.md")
    output_path = base_path / "llm_enhanced_darpa_report.md"
    
    print("🔍 Loading data...")
    with open(data_path) as f:
        data = json.load(f)
    
    print("🚀 Initializing analyzers...")
    code_analyzer = CodeAnalyzer()
    llm_analyzer = LLMAnalyzer()
    generator = EnhancedDARPAReportGenerator(code_analyzer, llm_analyzer)
    
    print("📝 Generating LLM-enhanced DARPA report...")
    report = generator.generate_report(data, str(template_path))
    
    print(f"💾 Saving to {output_path}...")
    with open(output_path, 'w') as f:
        f.write(report)
    
    print("✅ LLM-enhanced DARPA report generated!")

if __name__ == "__main__":
    main()

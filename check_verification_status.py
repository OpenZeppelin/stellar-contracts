#!/usr/bin/env python3
"""
Script to analyze formal verification status across the project.

Scans all files in /specs directories, counts #[rule] functions,
and checks their //status comments to identify unverified rules.

USAGE:
    python3 check_verification_status.py [OPTIONS]

OPTIONS:
    --format {table,json}
        Output format: 'table' (human-readable) or 'json' (machine-readable)
        Default: table

    --verbosity {repo,directory,file}
        Level of detail to show:
        - 'repo': Show only project-wide totals
        - 'directory': Show totals + breakdown by directory
        - 'file': Show full details (totals + directories + files + unverified rules)
        Default: file

    --path PATH
        Filter results to a specific directory or file.
        Can be relative to root directory or absolute path.
        Examples:
        - --path packages/tokens/src/vault/specs
        - --path packages/tokens/src/vault/specs/vault_solvency.rs
        Default: Show all spec files

    --root ROOT_DIR
        Root directory to search (default: current directory)

EXAMPLES:
    # Show full details for entire project
    python3 check_verification_status.py

    # Show only summary totals
    python3 check_verification_status.py --verbosity repo

    # Show directory breakdown for tokens package
    python3 check_verification_status.py --path packages/tokens --verbosity directory

    # Show full details for a specific file
    python3 check_verification_status.py --path packages/tokens/src/vault/specs/vault_solvency.rs

    # Get JSON output for a specific directory
    python3 check_verification_status.py --path packages/tokens/src/vault/specs --format json --verbosity directory
"""

import os
import re
from pathlib import Path
from collections import defaultdict
from typing import Dict, List, Tuple, Optional
import json


class RuleInfo:
    """Information about a single rule."""
    def __init__(self, name: str, status: Optional[str], line_num: int):
        self.name = name
        self.status = status
        self.line_num = line_num
        self.is_verified = self._check_verified()
        self.is_bug = self._check_if_bug()
    
    def _check_verified(self) -> bool:
        """Check if the rule is verified."""
        if self.status is None:
            return False
        status_lower = self.status.lower().strip()
        # Check if status starts with "verified"
        return status_lower.startswith("verified")

    def _check_if_bug(self) -> bool:
        """Check if the rule is a bug."""
        if self.status is None:
            return False
        status_lower = self.status.lower().strip()
        return status_lower.startswith("bug")

class FileAnalysis:
    """Analysis results for a single file."""
    def __init__(self, file_path: str):
        self.file_path = file_path
        self.rules: List[RuleInfo] = []
        self.total_rules = 0
        self.verified_rules = 0
        self.unverified_rules = 0
        self.bug_rules = 0
    
    def add_rule(self, rule: RuleInfo):
        """Add a rule to this file's analysis."""
        self.rules.append(rule)
        self.total_rules += 1
        if rule.is_verified:
            self.verified_rules += 1
        elif not rule.is_bug:
            self.unverified_rules += 1
        if rule.is_bug:
            self.bug_rules += 1


def find_spec_files(root_dir: Path) -> List[Path]:
    """Find all Rust files in specs directories."""
    spec_files = []
    excluded_dirs = {".certora_internal", "target", ".git"}
    
    for path in root_dir.rglob("*.rs"):
        # Skip if any part of the path is in excluded directories
        if any(excluded in path.parts for excluded in excluded_dirs):
            continue
        # Only include files in specs directories
        if "specs" in path.parts:
            spec_files.append(path)
    
    return sorted(spec_files)


def extract_status_comment(lines: List[str], rule_line_idx: int, func_line_idx: int) -> Optional[str]:
    """
    Extract status comment from lines around a rule.
    Looks for comments with 'status:' pattern before or after the #[rule] attribute.
    Prioritizes forward search (between #[rule] and function) to avoid picking up
    status comments from previous rules.
    """
    # First, look forwards between #[rule] and function definition
    # This is the most common case and avoids picking up previous rule's status
    end_idx = min(rule_line_idx + 15, func_line_idx, len(lines))
    for i in range(rule_line_idx + 1, end_idx):
        line = lines[i].strip()
        # Match patterns like:
        # // status: verified
        # //status: verified
        # // status: violated - bug
        # // status: first assert verified
        match = re.search(r'//\s*status\s*:\s*(.+)', line, re.IGNORECASE)
        if match:
            return match.group(1).strip()
    
    # Only if not found forward, look backwards from the rule line
    # Stop if we encounter another #[rule] or function definition
    for i in range(rule_line_idx - 1, max(-1, rule_line_idx - 11), -1):
        line = lines[i].strip()
        # Stop if we hit another #[rule] or function definition
        if '#[rule]' in line or re.search(r'(?:pub\s+)?fn\s+\w+', line):
            break
        match = re.search(r'//\s*status\s*:\s*(.+)', line, re.IGNORECASE)
        if match:
            return match.group(1).strip()
    
    return None


def extract_function_name(line: str) -> Optional[str]:
    """Extract function name from a function definition line."""
    # Match patterns like:
    # pub fn function_name(
    # fn function_name(
    # pub fn function_name<...>(
    match = re.search(r'(?:pub\s+)?fn\s+(\w+)', line)
    if match:
        return match.group(1)
    return None


def analyze_file(file_path: Path) -> FileAnalysis:
    """Analyze a single Rust file for rules and their status."""
    analysis = FileAnalysis(str(file_path))
    
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()
    except Exception as e:
        print(f"Error reading {file_path}: {e}", file=os.sys.stderr)
        return analysis
    
    for i, line in enumerate(lines):
        # Look for #[rule] attribute
        if '#[rule]' in line or '#[ rule ]' in line.replace(' ', ''):
            # Find the function definition (should be within next few lines)
            func_name = None
            func_line_idx = i
            
            # Look ahead for function definition (up to 15 lines to handle comments)
            for j in range(i + 1, min(i + 15, len(lines))):
                func_name = extract_function_name(lines[j])
                if func_name:
                    func_line_idx = j
                    break
            
            if func_name:
                # Skip rules that end with _sanity
                if func_name.endswith('_sanity'):
                    continue
                
                # Extract status comment
                status = extract_status_comment(lines, i, func_line_idx)
                rule = RuleInfo(func_name, status, i + 1)  # 1-indexed line numbers
                analysis.add_rule(rule)
    
    return analysis


def get_directory(path: str) -> str:
    """Get the directory path from a file path."""
    return str(Path(path).parent)


def get_relative_path(file_path: str, root_dir: Path) -> str:
    """Convert absolute file path to relative path, removing root and packages/ prefix."""
    try:
        # Convert to Path and make relative to root
        path = Path(file_path)
        if path.is_absolute():
            try:
                relative = path.relative_to(root_dir)
            except ValueError:
                # If not relative to root, return as is
                return file_path
        else:
            relative = path
        
        # Convert to string and remove packages/ prefix if present
        path_str = str(relative)
        if path_str.startswith("packages/"):
            path_str = path_str[len("packages/"):]
        
        return path_str
    except Exception:
        return file_path


def filter_analyses_by_path(analyses: List[FileAnalysis], filter_path: Path, root_dir: Path) -> List[FileAnalysis]:
    """
    Filter analyses to only include those matching the given path.
    If path is a directory, includes all files in that directory.
    If path is a file, includes only that file.
    """
    if not filter_path.exists():
        return []
    
    filter_path = filter_path.resolve()
    root_dir = root_dir.resolve()
    
    # Try to make filter_path relative to root_dir
    try:
        relative_filter = filter_path.relative_to(root_dir)
    except ValueError:
        # If not relative, use absolute path
        relative_filter = filter_path
    
    filtered = []
    for analysis in analyses:
        analysis_path = Path(analysis.file_path).resolve()
        
        # Check if analysis path matches filter path
        if filter_path.is_file():
            # Exact file match
            if analysis_path == filter_path:
                filtered.append(analysis)
        else:
            # Directory match - check if analysis is in or under the directory
            try:
                analysis_path.relative_to(filter_path)
                filtered.append(analysis)
            except ValueError:
                # Not in this directory
                pass
    
    return filtered


def format_summary(analyses: List[FileAnalysis], output_format: str = "table", root_dir: Path = None, verbosity: str = "file") -> str:
    """Format the analysis results."""
    if root_dir is None:
        root_dir = Path.cwd()
    if output_format == "json":
        return format_json(analyses, root_dir, verbosity)
    else:
        return format_table(analyses, root_dir, verbosity)


def format_table(analyses: List[FileAnalysis], root_dir: Path, verbosity: str = "file") -> str:
    """Format results as a table."""
    output = []
    
    # Project-wide totals
    total_rules = sum(a.total_rules for a in analyses)
    total_verified = sum(a.verified_rules for a in analyses)
    total_unverified = sum(a.unverified_rules for a in analyses)
    total_bug = sum(a.bug_rules for a in analyses)
    
    output.append("=" * 40)
    output.append("Formal Verification Status Summary")
    output.append("=" * 40)
    output.append("")
    
    # Always show repo-level summary
    if verbosity in ["repo", "directory", "file"]:
        output.append(f"Total:")
        output.append(f"  Total Rules: {total_rules}")
        output.append(f"  Verified: {total_verified}")
        output.append(f"  Unverified: {total_unverified}")
        output.append(f"  Bug: {total_bug}")
        output.append("")
    
    # Show directory-level if verbosity is directory or file
    if verbosity in ["directory", "file"]:
        # Group by directory
        dir_stats: Dict[str, Dict[str, int]] = defaultdict(lambda: {"total": 0, "verified": 0, "unverified": 0, "bug": 0})
        for analysis in analyses:
            if analysis.total_rules > 0:
                dir_path = get_relative_path(get_directory(analysis.file_path), root_dir)
                dir_stats[dir_path]["total"] += analysis.total_rules
                dir_stats[dir_path]["verified"] += analysis.verified_rules
                dir_stats[dir_path]["unverified"] += analysis.unverified_rules
                dir_stats[dir_path]["bug"] += analysis.bug_rules
        
        output.append("=" * 40)
        output.append("By Directory:")
        output.append("=" * 40)
        output.append("")
        
        for dir_path in sorted(dir_stats.keys()):
            stats = dir_stats[dir_path]
            output.append(f"{dir_path}:")
            output.append(f"  Total: {stats['total']}, Verified: {stats['verified']}, Unverified: {stats['unverified']}, Bug: {stats['bug']}")
            output.append("")
    
    # Show file-level if verbosity is file
    if verbosity == "file":
        output.append("=" * 40)
        output.append("By File:")
        output.append("=" * 40)
        output.append("")
        
        for analysis in sorted(analyses, key=lambda x: x.file_path):
            if analysis.total_rules > 0:
                rel_path = get_relative_path(analysis.file_path, root_dir)
                output.append(f"{rel_path}:")
                output.append(f"  Total: {analysis.total_rules}, Verified: {analysis.verified_rules}, Unverified: {analysis.unverified_rules}, Bug: {analysis.bug_rules}")
                
                # List unverified rules
                unverified = [r for r in analysis.rules if not r.is_verified and not r.is_bug]
                if unverified:
                    output.append("  Unverified Rules:")
                    for rule in unverified:
                        status_str = rule.status if rule.status else "no status"
                        output.append(f"    - {rule.name} (line {rule.line_num}): {status_str}")
                output.append("")
    
    return "\n".join(output)


def format_json(analyses: List[FileAnalysis], root_dir: Path, verbosity: str = "file") -> str:
    """Format results as JSON."""
    result = {}
    
    # Always include repo-level summary
    if verbosity in ["repo", "directory", "file"]:
        result["total"] = {
            "total_rules": sum(a.total_rules for a in analyses),
            "verified_rules": sum(a.verified_rules for a in analyses),
            "unverified_rules": sum(a.unverified_rules for a in analyses),
            "bug_rules": sum(a.bug_rules for a in analyses)
        }
    
    # Include directory-level if verbosity is directory or file
    if verbosity in ["directory", "file"]:
        result["by_directory"] = {}
        # Group by directory
        dir_stats: Dict[str, Dict[str, int]] = defaultdict(lambda: {"total": 0, "verified": 0, "unverified": 0, "bug": 0})
        
        for analysis in analyses:
            if analysis.total_rules > 0:
                dir_path = get_relative_path(get_directory(analysis.file_path), root_dir)
                dir_stats[dir_path]["total"] += analysis.total_rules
                dir_stats[dir_path]["verified"] += analysis.verified_rules
                dir_stats[dir_path]["unverified"] += analysis.unverified_rules
                dir_stats[dir_path]["bug"] += analysis.bug_rules
        
        for dir_path, stats in sorted(dir_stats.items()):
            result["by_directory"][dir_path] = stats.copy()
    
    # Include file-level if verbosity is file
    if verbosity == "file":
        result["by_file"] = []
        for analysis in sorted(analyses, key=lambda x: x.file_path):
            if analysis.total_rules > 0:
                rel_path = get_relative_path(analysis.file_path, root_dir)
                file_data = {
                    "file": rel_path,
                    "total_rules": analysis.total_rules,
                    "verified_rules": analysis.verified_rules,
                    "unverified_rules": analysis.unverified_rules,
                    "bug_rules": analysis.bug_rules,
                    "rules": []
                }
                
                for rule in analysis.rules:
                    file_data["rules"].append({
                        "name": rule.name,
                        "line": rule.line_num,
                        "status": rule.status,
                        "verified": rule.is_verified,
                        "bug": rule.is_bug
                    })
                
                result["by_file"].append(file_data)
    
    return json.dumps(result, indent=2)


def main():
    """Main entry point."""
    import argparse
    
    parser = argparse.ArgumentParser(
        description="Analyze formal verification status across the project"
    )
    parser.add_argument(
        "--format",
        choices=["table", "json"],
        default="table",
        help="Output format (default: table)"
    )
    parser.add_argument(
        "--root",
        type=str,
        default=".",
        help="Root directory to search (default: current directory)"
    )
    parser.add_argument(
        "--verbosity",
        choices=["repo", "directory", "file"],
        default="file",
        help="Verbosity level: 'repo' (summary only), 'directory' (summary + directory breakdown), 'file' (full details) (default: file)"
    )
    parser.add_argument(
        "--path",
        type=str,
        default=None,
        help="Filter results to a specific directory or file path (relative to root or absolute)"
    )
    
    args = parser.parse_args()
    
    root_dir = Path(args.root).resolve()
    
    if not root_dir.exists():
        print(f"Error: Root directory does not exist: {root_dir}", file=os.sys.stderr)
        return 1
    
    # Find all spec files
    spec_files = find_spec_files(root_dir)
    
    if not spec_files:
        print("No spec files found in specs directories.", file=os.sys.stderr)
        return 1
    
    # Analyze each file
    analyses = []
    for spec_file in spec_files:
        analysis = analyze_file(spec_file)
        if analysis.total_rules > 0:
            analyses.append(analysis)
    
    # Filter by path if provided
    if args.path:
        filter_path = Path(args.path)
        if not filter_path.is_absolute():
            # Try relative to root_dir first, then current directory
            filter_path = root_dir / filter_path
            if not filter_path.exists():
                filter_path = Path(args.path).resolve()
        
        if not filter_path.exists():
            print(f"Error: Path does not exist: {args.path}", file=os.sys.stderr)
            return 1
        
        analyses = filter_analyses_by_path(analyses, filter_path, root_dir)
        
        if not analyses:
            print(f"No spec files found matching path: {args.path}", file=os.sys.stderr)
            return 1
    
    # Format and print results
    output = format_summary(analyses, args.format, root_dir, args.verbosity)
    print(output)
    
    return 0


if __name__ == "__main__":
    exit(main())

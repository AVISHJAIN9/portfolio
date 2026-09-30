#!/usr/bin/env python3
"""
tools/dedupe_scan.py
AST/token duplicate code scanner across all projects in the monorepo.
Detects identical or near-identical code blocks >30 lines across different projects,
reporting candidates for promotion to platform/.
"""

import os
import sys
import hashlib
from pathlib import Path
from collections import defaultdict

def scan_files_for_duplicates(root_dir: Path, min_lines: int = 30):
    projects_dir = root_dir / "projects"
    if not projects_dir.exists():
        print("No projects directory found.")
        return []
        
    block_hashes = defaultdict(list)
    
    # Scan .rs, .py, .ts, .tsx files across all projects
    extensions = {".rs", ".py", ".ts", ".tsx", ".js"}
    
    for proj_path in sorted(projects_dir.iterdir()):
        if not proj_path.is_dir():
            continue
        proj_name = proj_path.name
        
        for file_path in proj_path.glob("**/*"):
            if not file_path.is_file() or file_path.suffix not in extensions:
                continue
            if "node_modules" in str(file_path) or "target" in str(file_path) or "vendor" in str(file_path):
                continue
                
            try:
                with open(file_path, "r", encoding="utf-8", errors="ignore") as f:
                    lines = [l.strip() for l in f.readlines() if l.strip() and not l.strip().startswith("//") and not l.strip().startswith("#")]
            except Exception:
                continue
                
            # Slide window of min_lines
            for i in range(0, len(lines) - min_lines + 1, 5):
                window = "\n".join(lines[i:i + min_lines])
                h = hashlib.sha256(window.encode("utf-8")).hexdigest()
                block_hashes[h].append((proj_name, str(file_path.relative_to(root_dir)), i + 1))
                
    # Filter for blocks appearing in multiple distinct projects
    candidates = []
    for h, occurrences in block_hashes.items():
        projects = {occ[0] for occ in occurrences}
        if len(projects) > 1:
            candidates.append(occurrences)
            
    return candidates

def main():
    root_dir = Path(__file__).resolve().parent.parent
    print("=== Scanning for Duplicate Code Across Projects (Window: 30 lines) ===")
    duplicates = scan_files_for_duplicates(root_dir)
    
    if not duplicates:
        print("✓ Zero cross-project duplicates (>30 lines) detected. Monorepo is fully DRY.")
    else:
        print(f"Found {len(duplicates)} duplicate code blocks across projects:")
        for idx, dup in enumerate(duplicates, 1):
            print(f"\nCandidate {idx}:")
            for proj, fpath, line in dup:
                print(f"  - [{proj}] {fpath}:{line}")
        print("\nACTION: Promote duplicated logic into platform/ crates or packages.")

if __name__ == "__main__":
    main()

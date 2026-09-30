#!/usr/bin/env python3
"""
tools/evidence_gen.py
Autonomous evidence pipeline.
Runs cargo test across monorepo projects, parses annotated test cases (// FEAT: <id>),
and programmatically generates evidence/feature_evidence.csv and features.jsonl status updates.
"""

import os
import sys
import re
import json
import subprocess
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent

def extract_feature_annotations(project_dir: Path):
    """Scans all test files in project for '// FEAT: <id>' annotations."""
    annotations = {}
    tests_dir = project_dir / "tests"
    if not tests_dir.exists():
        return annotations

    feat_pattern = re.compile(r'//\s*FEAT:\s*([A-Za-z0-9_\-]+)')
    fn_pattern = re.compile(r'fn\s+([a-zA-Z0-9_]+)\s*\(')

    for rs_file in tests_dir.glob("*.rs"):
        current_feat = None
        try:
            with open(rs_file, "r", encoding="utf-8") as f:
                for line in f:
                    feat_match = feat_pattern.search(line)
                    if feat_match:
                        current_feat = feat_match.group(1).strip()
                    fn_match = fn_pattern.search(line)
                    if fn_match and current_feat:
                        test_fn = fn_match.group(1)
                        annotations[test_fn] = (current_feat, str(rs_file.relative_to(ROOT_DIR)))
        except Exception:
            pass
    return annotations

def run_project_tests(project_slug: str):
    """Runs cargo test for a project and collects passing test names."""
    manifest_path = ROOT_DIR / "projects" / project_slug / "backend" / "Cargo.toml"
    if not manifest_path.exists():
        return []
        
    cmd = ["cargo", "test", "--manifest-path", str(manifest_path), "--", "--nocapture"]
    res = subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT_DIR)
    
    passing_tests = []
    for line in res.stdout.splitlines():
        if line.startswith("test ") and line.endswith("... ok"):
            test_name = line.split()[1]
            # Strip module path if any
            test_fn = test_name.split("::")[-1]
            passing_tests.append(test_fn)
    return passing_tests

def main():
    print("=== Running Autonomous Evidence Generation Pipeline ===")
    evidence_rows = []
    
    git_commit = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()[:10]
    
    projects = [
        "01-rollback-netcode-server",
        "02-hnsw-filtered-vector-index",
        "04-columnar-query-engine",
        "05-mini-quic"
    ]
    
    for slug in projects:
        proj_dir = ROOT_DIR / "projects" / slug / "backend"
        annotations = extract_feature_annotations(proj_dir)
        passing = run_project_tests(slug)
        
        jsonl_path = ROOT_DIR / "projects" / slug / "spec" / "features.jsonl"
        features = []
        if jsonl_path.exists():
            with open(jsonl_path, "r", encoding="utf-8") as f:
                features = [json.loads(l) for l in f if l.strip()]
                
        verified_feat_ids = set()
        for test_fn in passing:
            if test_fn in annotations:
                feat_id, test_path = annotations[test_fn]
                verified_feat_ids.add(feat_id)
                evidence_rows.append({
                    "feature_id": feat_id,
                    "project": slug,
                    "status": "Verified",
                    "test_path": test_path,
                    "command": f"cargo test --test {Path(test_path).stem}",
                    "result_file": f"projects/{slug}/benchmarks/results/{slug.split('-')[1]}_benchmarks.json",
                    "commit": git_commit
                })
                
        # Update features.jsonl
        for feat in features:
            if feat["id"] in verified_feat_ids:
                feat["status"] = "Verified"
            elif feat.get("status") == "Verified" and feat["id"] not in verified_feat_ids:
                feat["status"] = "Implemented"
                
        if features:
            with open(jsonl_path, "w", encoding="utf-8") as f:
                for feat in features:
                    f.write(json.dumps(feat) + "\n")
                    
    # Write evidence/feature_evidence.csv
    evidence_dir = ROOT_DIR / "evidence"
    evidence_dir.mkdir(parents=True, exist_ok=True)
    csv_path = evidence_dir / "feature_evidence.csv"
    with open(csv_path, "w", encoding="utf-8") as f:
        f.write("feature_id,project,status,test_path,command,result_file,commit\n")
        for row in evidence_rows:
            f.write(f"{row['feature_id']},{row['project']},{row['status']},{row['test_path']},{row['command']},{row['result_file']},{row['commit']}\n")
            
    print(f"✓ Generated {len(evidence_rows)} verified evidence rows in {csv_path}")

if __name__ == "__main__":
    main()

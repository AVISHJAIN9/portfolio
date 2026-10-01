#!/usr/bin/env python3
"""
tools/evidence_gen.py
Autonomous evidence pipeline.
Runs cargo test across monorepo projects, parses annotated test cases (// FEAT: <id> + // ACCEPT: <quote>),
counts assertions, verifies execution per test fn, and updates evidence/feature_evidence.csv and features.jsonl.
"""

import os
import sys
import re
import json
import subprocess
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent

def extract_feature_annotations_and_assertions(project_dir: Path):
    """
    Scans all test files in project for:
    // FEAT: <id>
    // ACCEPT: <quote>
    fn <test_name>() { ... }
    Counts assertion macros inside each test function body.
    """
    annotations = {}
    tests_dir = project_dir / "tests"
    if not tests_dir.exists():
        return annotations

    feat_pattern = re.compile(r'//\s*FEAT:\s*([A-Za-z0-9_\-]+)')
    accept_pattern = re.compile(r'//\s*ACCEPT:\s*(.+)')
    fn_start_pattern = re.compile(r'fn\s+([a-zA-Z0-9_]+)\s*\(')
    assert_pattern = re.compile(r'\b(assert|assert_eq|assert_ne|assert_matches|prop_assert|prop_assert_eq)!')

    for rs_file in tests_dir.glob("*.rs"):
        rel_path = str(rs_file.relative_to(ROOT_DIR))
        try:
            with open(rs_file, "r", encoding="utf-8") as f:
                lines = f.readlines()
        except Exception:
            continue

        i = 0
        while i < len(lines):
            line = lines[i]
            feat_match = feat_pattern.search(line)
            if feat_match:
                feat_id = feat_match.group(1).strip()
                accept_quote = ""
                # Look ahead for optional ACCEPT or fn
                j = i + 1
                while j < len(lines) and j < i + 6:
                    ac_match = accept_pattern.search(lines[j])
                    if ac_match:
                        accept_quote = ac_match.group(1).strip()
                    fn_match = fn_start_pattern.search(lines[j])
                    if fn_match:
                        test_fn = fn_match.group(1)
                        # Count assertions in this function body
                        brace_count = 0
                        body_started = False
                        assertions = 0
                        k = j
                        while k < len(lines):
                            k_line = lines[k]
                            assertions += len(assert_pattern.findall(k_line))
                            brace_count += k_line.count('{') - k_line.count('}')
                            if '{' in k_line:
                                body_started = True
                            if body_started and brace_count <= 0:
                                break
                            k += 1
                        annotations[test_fn] = {
                            "feat_id": feat_id,
                            "accept_quote": accept_quote,
                            "test_file": rel_path,
                            "assertion_count": assertions
                        }
                        i = k
                        break
                    j += 1
            i += 1
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
            parts = line.split()
            if len(parts) >= 2:
                test_name = parts[1]
                test_fn = test_name.split("::")[-1]
                passing_tests.append(test_fn)
    return passing_tests

def main():
    print("=== Running Autonomous Evidence Generation Pipeline ===")
    evidence_rows = []
    
    try:
        git_commit = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()[:10]
    except Exception:
        git_commit = "HEAD"
    
    projects = [
        "04-columnar-query-engine",
        "05-mini-quic",
        "06-lsm-kv-engine"
    ]
    
    print("\n| Feature ID | Project | Test Function | Assertions | Acceptance Summary | Status |")
    print("|---|---|---|---|---|---|")
    
    for slug in projects:
        proj_dir = ROOT_DIR / "projects" / slug / "backend"
        annotations = extract_feature_annotations_and_assertions(proj_dir)
        passing = run_project_tests(slug)
        
        jsonl_path = ROOT_DIR / "projects" / slug / "spec" / "features.jsonl"
        features = []
        if jsonl_path.exists():
            with open(jsonl_path, "r", encoding="utf-8") as f:
                features = [json.loads(l) for l in f if l.strip()]
                
        verified_feat_ids = set()
        for test_fn, meta in annotations.items():
            feat_id = meta["feat_id"]
            test_file = meta["test_file"]
            assertions = meta["assertion_count"]
            accept_quote = meta["accept_quote"] or f"Verified by {test_fn}"
            
            is_passed = test_fn in passing
            is_verified = is_passed and assertions > 0
            status = "Verified" if is_verified else ("Implemented" if is_passed else "Specified")
            
            if is_verified:
                verified_feat_ids.add(feat_id)
                evidence_rows.append({
                    "feature_id": feat_id,
                    "project": slug,
                    "status": "Verified",
                    "test_file": test_file,
                    "test_fn": test_fn,
                    "command": f"cargo test --test {Path(test_file).stem} {test_fn}",
                    "assertion_count": assertions,
                    "acceptance_criteria": accept_quote,
                    "commit": git_commit
                })
            
            status_icon = "✓" if is_verified else "✗"
            print(f"| {feat_id} | {slug} | `{test_fn}` | {assertions} | {accept_quote[:40]} | {status_icon} {status} |")
                
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
        f.write("feature_id,project,status,test_file,test_fn,command,assertion_count,acceptance_criteria,commit\n")
        for row in evidence_rows:
            f.write(f"{row['feature_id']},{row['project']},{row['status']},{row['test_file']},{row['test_fn']},{row['command']},{row['assertion_count']},{row['acceptance_criteria']},{row['commit']}\n")
            
    print(f"\n✓ Generated {len(evidence_rows)} verified evidence rows in {csv_path}")

if __name__ == "__main__":
    main()


#!/usr/bin/env python3
"""
tools/audit_test_annotations.py
Read-only mechanical gate for // FEAT: annotations.
Validates:
(a) Feature ID is present in evidence/master_features_04_05_06.json
(b) Quoted feature name matches master workbook verbatim (exact string equality)
(c) Priority matches P0/P1/P2
(d) No single test fn carries FEAT ids for > 2 features without written justification comment
(e) Test fn name shares at least one meaningful keyword/stem with feature name
"""

import sys
import re
import json
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent

STOPWORDS = {
    "and", "the", "for", "with", "support", "handling", "tests", "engine", "format",
    "strategy", "path", "suite", "level", "group", "option", "custom", "based", "mode",
    "policy", "limits", "state", "core", "test", "item", "check", "controls"
}

def extract_keywords(feature_name: str) -> set:
    words = re.findall(r'[a-zA-Z0-9]+', feature_name.lower())
    keywords = set()
    for w in words:
        if len(w) >= 3 and w not in STOPWORDS:
            keywords.add(w)
            # also add stem of length >= 4
            if len(w) > 4:
                keywords.add(w[:4])
    return keywords

def audit_annotations(test_dirs=None, master_json_path=None) -> list:
    errors = []
    if master_json_path is None:
        master_json_path = ROOT_DIR / "evidence" / "master_features_04_05_06.json"
    
    if not master_json_path.exists():
        return [f"Master features JSON not found at {master_json_path}"]
    
    with open(master_json_path, "r", encoding="utf-8") as f:
        master = json.load(f)
        
    feat_pat = re.compile(r'//\s*FEAT:\s*([0-9]{2}-[0-9]{3})(?:\s+\"([^\"]+)\")?')
    fn_pat = re.compile(r'fn\s+([a-zA-Z0-9_]+)\s*\(')
    justification_pat = re.compile(r'//\s*JUSTIFICATION:\s*.+')

    if test_dirs is None:
        test_dirs = [
            ROOT_DIR / "projects" / "04-columnar-query-engine" / "backend" / "tests",
            ROOT_DIR / "projects" / "05-mini-quic" / "backend" / "tests",
            ROOT_DIR / "projects" / "06-lsm-kv-engine" / "backend" / "tests",
        ]

    for tdir in test_dirs:
        if not tdir.exists():
            continue
        for rs_file in sorted(tdir.glob("*.rs")):
            rel_file = str(rs_file.relative_to(ROOT_DIR)) if rs_file.is_relative_to(ROOT_DIR) else str(rs_file)
            lines = rs_file.read_text(encoding="utf-8").splitlines()
            
            i = 0
            while i < len(lines):
                line = lines[i]
                m = feat_pat.search(line)
                if m:
                    # Gather all FEAT annotations directly above the test function
                    feat_group = [m]
                    has_justification = bool(justification_pat.search(line))
                    k = i + 1
                    while k < len(lines) and lines[k].strip().startswith("//"):
                        next_m = feat_pat.search(lines[k])
                        if next_m:
                            feat_group.append(next_m)
                        if justification_pat.search(lines[k]):
                            has_justification = True
                        k += 1
                    
                    # Find associated test function
                    fn_name = None
                    for j in range(k, min(len(lines), k + 6)):
                        fn_m = fn_pat.search(lines[j])
                        if fn_m:
                            fn_name = fn_m.group(1)
                            break
                            
                    # Check (d): > 2 features without justification
                    if len(feat_group) > 2 and not has_justification:
                        errors.append(
                            f"{rel_file}:{i+1} Rule (d) Violation: Test `{fn_name}` carries {len(feat_group)} FEAT annotations (> 2) without `// JUSTIFICATION:` comment."
                        )
                        
                    # Check each annotation in group
                    for item_m in feat_group:
                        fid = item_m.group(1).strip()
                        qname = item_m.group(2)
                        
                        # Check (a): ID in master
                        if fid not in master:
                            errors.append(
                                f"{rel_file}:{i+1} Rule (a) Violation: Feature ID `{fid}` does not exist in master workbook."
                            )
                            continue
                            
                        master_item = master[fid]
                        expected_name = master_item["name"]
                        expected_prio = master_item["priority"]
                        
                        # Check (b): Exact name equality
                        if qname is None or qname != expected_name:
                            errors.append(
                                f"{rel_file}:{i+1} Rule (b) Violation: Feature `{fid}` annotated with '{qname}' != verbatim workbook name '{expected_name}'."
                            )
                            
                        # Check (c): Valid priority
                        if expected_prio not in ["P0", "P1", "P2"]:
                            errors.append(
                                f"{rel_file}:{i+1} Rule (c) Violation: Feature `{fid}` has invalid master priority '{expected_prio}'."
                            )
                            
                        # Check (e): Keyword match with function name
                        if fn_name:
                            keywords = extract_keywords(expected_name)
                            fn_lower = fn_name.lower().replace("_", "")
                            matched = any(kw in fn_lower for kw in keywords)
                            if not matched and len(keywords) > 0:
                                errors.append(
                                    f"{rel_file}:{i+1} Rule (e) Violation: Test function `{fn_name}` shares no keyword with feature name '{expected_name}' (keywords: {sorted(keywords)})."
                                )
                    i = k
                i += 1
    return errors

def main():
    print("=== Auditing Test Annotations Against Master Workbooks ===")
    errors = audit_annotations()
    if errors:
        print(f"\n❌ AUDIT FAILED with {len(errors)} error(s):")
        for err in errors:
            print(f"  - {err}")
        sys.exit(1)
    else:
        print("✓ All test annotations strictly match master workbook names, IDs, priorities, and keyword signatures.")
        sys.exit(0)

if __name__ == "__main__":
    main()

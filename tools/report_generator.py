#!/usr/bin/env python3
"""
tools/report_generator.py
Generates the comprehensive Session 10 Integrity & Verification report.
Validates 3-way count consistency across P0 table, _STATUS.xlsx, and PORTFOLIO_STATE.md.
"""

import os
import sys
import json
import glob
import re
import hashlib
from pathlib import Path
import openpyxl

ROOT_DIR = Path(__file__).resolve().parent.parent

def check_count_consistency(print_table=True):
    with open(ROOT_DIR / "PORTFOLIO_STATE.md", "r", encoding="utf-8") as f:
        md_text = f.read()

    consistency_data = {}
    all_match = True

    for slot, name in [("04", "Columnar Query Engine"), ("05", "Mini-QUIC"), ("06", "LSM KV Engine")]:
        # (a) features.jsonl
        jsonl = list((ROOT_DIR / "projects").glob(f"{slot}-*/spec/features.jsonl"))[0]
        with open(jsonl, "r", encoding="utf-8") as f:
            feats = [json.loads(l) for l in f if l.strip()]
        a_count = sum(1 for feat in feats if feat.get("priority") == "P0" and feat.get("status") == "Verified")
        p0_tot = sum(1 for feat in feats if feat.get("priority") == "P0")

        # (b) STATUS.xlsx
        wb_path = list((ROOT_DIR / "evidence" / "workbooks").glob(f"{slot}_*_STATUS.xlsx"))[0]
        wb = openpyxl.load_workbook(wb_path, data_only=True)
        ws = wb["Detailed Analysis"]
        headers = [ws.cell(1, c).value for c in range(1, ws.max_column + 1)]
        p_idx = headers.index("Priority") + 1
        s_idx = headers.index("Status") + 1
        b_count = sum(1 for r in range(2, ws.max_row + 1) if ws.cell(r, p_idx).value == "P0" and ws.cell(r, s_idx).value == "Verified")

        # (c) PORTFOLIO_STATE.md
        m = re.search(r"\|\s*" + slot + r"\s*\|\s*`[^`]+`\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|", md_text)
        c_count = int(m.group(2)) if m else -1

        match = (a_count == b_count == c_count)
        if not match:
            all_match = False
        consistency_data[slot] = {
            "name": name,
            "p0_total": p0_tot,
            "table_verified": a_count,
            "xlsx_verified": b_count,
            "state_md_verified": c_count,
            "match": match
        }

    if print_table:
        print("### PART 0.2: THREE-WAY COUNT CONSISTENCY GATE PER PROJECT")
        print("| Project Slot | Project Name | P0 Total | (a) P0 Table Verified | (b) _STATUS.xlsx Verified | (c) PORTFOLIO_STATE.md | Consistent? |")
        print("|:---:|:---|:---:|:---:|:---:|:---:|:---:|")
        for slot, d in consistency_data.items():
            status_str = "✓ PASS (MATCH)" if d["match"] else "✗ FAIL (MISMATCH)"
            print(f"| `{slot}` | {d['name']} | {d['p0_total']} | **{d['table_verified']}** | **{d['xlsx_verified']}** | **{d['state_md_verified']}** | {status_str} |")
        print()

    return all_match, consistency_data

def main():
    if "--check" in sys.argv:
        all_match, _ = check_count_consistency(print_table=False)
        if not all_match:
            print("ERROR: Count consistency check failed between P0 table, _STATUS.xlsx, and PORTFOLIO_STATE.md!")
            sys.exit(1)
        else:
            print("Count consistency check: PASS (All project counts match across Table, _STATUS.xlsx, and PORTFOLIO_STATE.md)")
            sys.exit(0)

    with open(ROOT_DIR / "evidence" / "master_features_04_05_06.json", "r", encoding="utf-8") as f:
        master = json.load(f)

    # Load evidence csv
    evidence = {}
    csv_path = ROOT_DIR / "evidence" / "feature_evidence.csv"
    if csv_path.exists():
        with open(csv_path, "r", encoding="utf-8") as f:
            for line in f.readlines()[1:]:
                parts = line.strip().split(",")
                if len(parts) >= 8:
                    evidence[parts[0]] = {
                        "status": parts[2],
                        "file": parts[3],
                        "fn": parts[4],
                        "assertions": parts[6],
                    }

    print("================================================================================")
    print("SESSION 10 — VERIFY THE KEEPS, REAL INTEROP, LSM SPINE & SQL EXECUTION REPORT")
    print("================================================================================\n")

    print("### PART 0.1: SOURCE-OF-TRUTH WORKBOOK SHA256 INTEGRITY CHECK")
    print("```")
    for fname, expected in [
        ("04_Columnar_Query_Engine_ULTIMATE_145_Master.xlsx", "6699a333e16e245f"),
        ("05_Mini_QUIC_ULTIMATE_145_Master.xlsx", "69429364a3380b66"),
        ("06_LSM_KV_ENGINE_ULTIMATE_145_Master.xlsx", "7e8a3e65fc301aef"),
    ]:
        p = ROOT_DIR / "inputs" / "specs" / fname
        with open(p, "rb") as f:
            h = hashlib.sha256(f.read()).hexdigest()
        match = "MATCH (VERIFIED)" if h.startswith(expected) else "MISMATCH"
        print(f"{fname:<50} sha256={h[:16]}... ({match})")
    print("```\n")

    all_match, consistency_data = check_count_consistency(print_table=True)
    if not all_match:
        print("CRITICAL WARNING: Count consistency check failed!")

    print("### PART 0.6: AUDITED VERIFIED RATIOS")
    for slot, d in consistency_data.items():
        pct = (d['table_verified'] / d['p0_total'] * 100.0) if d['p0_total'] > 0 else 0.0
        print(f"- **Project {slot} ({d['name']})**: **Verified {d['table_verified']}/{d['p0_total']} P0 ({pct:.1f}%)**")
    print()

    print("### PART 2(a): PER-PROJECT P0 AUDITED STATUS TABLES\n")
    for slot, title in [("04", "04 - Columnar Query Engine"), ("05", "05 - Mini-QUIC"), ("06", "06 - LSM KV Engine")]:
        print(f"#### {title} (P0 Features)")
        print("| Feature ID | Verbatim Workbook Name | Status | Test Function | Test File:Line | Assertions |")
        print("|:---|:---|:---:|:---|:---|:---:|")
        slot_p0 = [(fid, meta) for fid, meta in master.items() if fid.startswith(f"{slot}-") and meta.get("priority") == "P0"]
        for fid, meta in slot_p0:
            ev = evidence.get(fid)
            if ev and ev["status"] == "Verified":
                status = f"✓ Verified"
                fn = f"`{ev['fn']}`"
                loc = f"[`{ev['file']}`](file:///{ROOT_DIR}/{ev['file']})"
                assertions = ev["assertions"]
            else:
                status = "✗ Specified"
                fn = "N/A"
                loc = "Pending Test Implementation"
                assertions = "0"
            print(f"| {fid} | {meta['name']} | {status} | {fn} | {loc} | {assertions} |")
        print()

if __name__ == "__main__":
    main()

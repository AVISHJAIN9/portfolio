#!/usr/bin/env python3
"""
tools/ingest_07_08_09.py
Parses the master workbooks for:
- 07 Stream Detect Engine (145 features)
- 08 PubSub Broker (145 features)
- 09 Distributed Cache (150 features)
Emits features.jsonl and docs/FEATURE_STATUS.csv for each project.
"""

import os
import sys
import json
import csv
import openpyxl
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent

WORKBOOKS = [
    ("07", "07-stream-detect-engine", "07_Stream_Detect_Engine_ULTIMATE_145_Master.xlsx"),
    ("08", "08-pubsub-broker", "08_PubSub_Broker_ULTIMATE_145_Master.xlsx"),
    ("09", "09-distributed-cache", "09_Distributed_Cache_ULTIMATE_150_Master.xlsx"),
]

def ingest_workbook(slot, slug, filename):
    wb_path = ROOT_DIR / "inputs" / "specs" / filename
    if not wb_path.exists():
        print(f"ERROR: {wb_path} not found!")
        return

    wb = openpyxl.load_workbook(wb_path, data_only=True)
    ws_d = wb["Detailed Analysis"]
    
    # Header row
    headers = [ws_d.cell(1, c).value for c in range(1, 20)]
    
    def get_col(name):
        for idx, h in enumerate(headers):
            if h and name.lower() in str(h).lower():
                return idx + 1
        return None

    feat_col = get_col("feature") or 1
    area_col = get_col("area") or 2
    priority_col = get_col("priority") or 3
    hours_col = get_col("hours") or 4
    risk_col = get_col("risk") or 5
    deps_col = get_col("depend") or 6
    acc_col = get_col("acceptance") or 7
    notes_col = get_col("implementation") or 8

    # Matrix sheet for FE, BE, DB, AIML ticks
    matrix_sheet = wb["Feature Matrix"] if "Feature Matrix" in wb.sheetnames else wb["Master Roadmap"]
    m_headers = [matrix_sheet.cell(1, c).value for c in range(1, 20)]
    
    fe_col = None
    be_col = None
    db_col = None
    ai_col = None
    for idx, h in enumerate(m_headers):
        val = str(h or "").strip().lower()
        if val in ["fe", "frontend"]: fe_col = idx + 1
        elif val in ["be", "backend"]: be_col = idx + 1
        elif val in ["db", "database"]: db_col = idx + 1
        elif val in ["ai", "ai/ml", "aiml", "ml"]: ai_col = idx + 1

    features = []
    csv_rows = []
    
    row_idx = 1
    for r in range(2, ws_d.max_row + 1):
        feat_name = ws_d.cell(r, feat_col).value
        if not feat_name or not str(feat_name).strip():
            continue
            
        feat_id = f"{slot}-{row_idx:03d}"
        name_str = str(feat_name).strip()
        area_str = str(ws_d.cell(r, area_col).value or "Core").strip()
        prio_str = str(ws_d.cell(r, priority_col).value or "P0").strip()
        hours_val = ws_d.cell(r, hours_col).value or 0
        risk_str = str(ws_d.cell(r, risk_col).value or "Medium").strip()
        deps_str = str(ws_d.cell(r, deps_col).value or "").strip()
        acc_str = str(ws_d.cell(r, acc_col).value or "").strip()
        notes_str = str(ws_d.cell(r, notes_col).value or "").strip()

        # Check responsibilities from matrix if available
        fe_tick = bool(matrix_sheet.cell(r, fe_col).value) if fe_col else False
        be_tick = bool(matrix_sheet.cell(r, be_col).value) if be_col else True
        db_tick = bool(matrix_sheet.cell(r, db_col).value) if db_col else False
        ai_tick = bool(matrix_sheet.cell(r, ai_col).value) if ai_col else False

        # Project 08 relabeling handling per prompt
        if slot == "08":
            if "Security, Testing & Deployment" in area_str and row_idx <= 8:
                fe_tick = True
            if "AI/ML & Intelligent Operations" in area_str:
                prio_str = "P0" # promote security items to P0
                ai_tick = False

        feat_obj = {
            "id": feat_id,
            "name": name_str,
            "area": area_str,
            "priority": prio_str,
            "est_hours": hours_val,
            "risk": risk_str,
            "dependencies": deps_str,
            "acceptance_criteria": acc_str,
            "notes": notes_str,
            "responsibilities": {
                "frontend": fe_tick,
                "backend": be_tick,
                "database": db_tick,
                "aiml": ai_tick,
            },
            "status": "Specified",
            "evidence": "",
        }
        features.append(feat_obj)

        csv_rows.append({
            "feature": feat_id,
            "name": name_str,
            "area": area_str,
            "priority": prio_str,
            "fe": "✓" if fe_tick else "",
            "be": "✓" if be_tick else "",
            "db": "✓" if db_tick else "",
            "aiml": "✓" if ai_tick else "",
            "est_hours": hours_val,
            "risk": risk_str,
            "dependencies": deps_str,
            "status": "Specified",
            "evidence": "",
            "notes": notes_str,
        })
        row_idx += 1

    # Write features.jsonl
    proj_dir = ROOT_DIR / "projects" / slug
    spec_dir = proj_dir / "spec"
    spec_dir.mkdir(parents=True, exist_ok=True)
    jsonl_path = spec_dir / "features.jsonl"
    with open(jsonl_path, "w", encoding="utf-8") as f:
        for feat in features:
            f.write(json.dumps(feat) + "\n")

    # Write FEATURE_STATUS.csv
    docs_dir = proj_dir / "docs"
    docs_dir.mkdir(parents=True, exist_ok=True)
    csv_path = docs_dir / "FEATURE_STATUS.csv"
    with open(csv_path, "w", newline="", encoding="utf-8") as f:
        fieldnames = ["feature", "name", "area", "priority", "fe", "be", "db", "aiml", "est_hours", "risk", "dependencies", "status", "evidence", "notes"]
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        for r in csv_rows:
            writer.writerow(r)

    # Copy spreadsheet
    ss_dir = proj_dir / "spreadsheets"
    ss_dir.mkdir(parents=True, exist_ok=True)
    dest_ss = ss_dir / f"{slug}-spec.xlsx"
    with open(wb_path, "rb") as src_f, open(dest_ss, "wb") as dst_f:
        dst_f.write(src_f.read())

    print(f"✓ Project {slot} ({slug}): Ingested {len(features)} features -> {jsonl_path} & {csv_path}")

def main():
    print("=== Ingesting Projects 07, 08, 09 Master Workbooks ===")
    for slot, slug, fn in WORKBOOKS:
        ingest_workbook(slot, slug, fn)
    print("✓ All workbooks ingested successfully.")

if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""
tools/feature_ingest.py
Ingests master feature workbooks from inputs/specs/ into canonical spec/features.jsonl.
Strictly parses real workbooks with column detection.
Assigns deterministic feature IDs: <NN>-<zero-padded row index> (e.g., 05-001) + sha1 of name.
"""

import os
import sys
import glob
import json
import hashlib
import openpyxl
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent

SLUG_MAP = {
    1: "01-rollback-netcode-server",
    2: "02-hnsw-filtered-vector-index",
    3: "03-bytecode-vm-gc",
    4: "04-columnar-query-engine",
    5: "05-mini-quic",
    6: "06-lsm-kv-engine"
}

def detect_header_and_cols(ws):
    header_row = None
    cols = {}
    for r in range(1, 10):
        for c in range(1, ws.max_column + 1):
            val = str(ws.cell(r, c).value or "").strip()
            if not val:
                continue
            val_lower = val.lower()
            if val_lower in ["task / deliverable", "feature", "feature name", "task", "work item"]:
                header_row = r
                cols["feature"] = c
            elif val_lower in ["#", "id", "feature id", "feat id"]:
                cols["#"] = c
            elif val_lower in ["priority", "p-level"]:
                cols["priority"] = c
            elif val_lower in ["area", "phase", "part"]:
                cols["area"] = c
            elif val_lower in ["risk"]:
                cols["risk"] = c
            elif val_lower in ["acceptance criteria", "acceptance_criteria", "description"]:
                cols["acceptance"] = c
        if "feature" in cols:
            break
    return header_row, cols

def parse_slot_workbook(wb_path: Path):
    slot_num = int(wb_path.name.split("_")[0])
    slug = SLUG_MAP.get(slot_num, f"{slot_num:02d}-project")
    wb = openpyxl.load_workbook(wb_path, data_only=True)
    
    # Check if there is a detailed analysis / tasks sheet
    da_sheet_name = None
    for name in ["Detailed Analysis", "Detailed Tasks"]:
        if name in wb.sheetnames:
            da_sheet_name = name
            break
            
    da_data = {}
    if da_sheet_name:
        ws_da = wb[da_sheet_name]
        da_h_row, da_cols = detect_header_and_cols(ws_da)
        if da_h_row:
            for r in range(da_h_row + 1, ws_da.max_row + 1):
                feat_val = ws_da.cell(r, da_cols.get("feature", 1)).value
                if not feat_val or not str(feat_val).strip():
                    continue
                feat_name = str(feat_val).strip()
                row_info = {
                    "priority": str(ws_da.cell(r, da_cols.get("priority", 1)).value or "").strip() if "priority" in da_cols else None,
                    "area": str(ws_da.cell(r, da_cols.get("area", 1)).value or "").strip() if "area" in da_cols else None,
                    "risk": str(ws_da.cell(r, da_cols.get("risk", 1)).value or "").strip() if "risk" in da_cols else None,
                    "acceptance": str(ws_da.cell(r, da_cols.get("acceptance", 1)).value or "").strip() if "acceptance" in da_cols else None,
                }
                da_data[feat_name] = row_info
                if "#" in da_cols:
                    num_val = ws_da.cell(r, da_cols["#"]).value
                    if num_val is not None:
                        da_data[str(num_val).strip()] = row_info

    # Primary matrix sheet
    matrix_sheet_name = None
    for name in ["Feature Matrix", "Master Roadmap", "Features", "Sheet1"]:
        if name in wb.sheetnames:
            matrix_sheet_name = name
            break
    assert matrix_sheet_name is not None, f"No matrix sheet found in {wb_path.name}"
    
    ws_m = wb[matrix_sheet_name]
    m_h_row, m_cols = detect_header_and_cols(ws_m)
    assert m_h_row is not None and "feature" in m_cols, f"Could not detect feature column in {wb_path.name}"
    
    features = []
    feat_idx = 1
    
    for r in range(m_h_row + 1, ws_m.max_row + 1):
        feat_val = ws_m.cell(r, m_cols["feature"]).value
        if not feat_val or not str(feat_val).strip():
            continue
        feat_name = str(feat_val).strip()
        
        da_info = da_data.get(feat_name, {})
        if not da_info and "#" in m_cols:
            num_val = str(ws_m.cell(r, m_cols["#"]).value or "").strip()
            da_info = da_data.get(num_val, {})
            
        priority = (
            da_info.get("priority")
            or (str(ws_m.cell(r, m_cols["priority"]).value).strip() if "priority" in m_cols and ws_m.cell(r, m_cols["priority"]).value else None)
            or "P0"
        )
        if not priority.startswith("P"):
            priority = "P0"
            
        area = (
            da_info.get("area")
            or (str(ws_m.cell(r, m_cols["area"]).value).strip() if "area" in m_cols and ws_m.cell(r, m_cols["area"]).value else None)
            or "Core"
        )
        
        risk = da_info.get("risk") or "Medium"
        acceptance = da_info.get("acceptance") or f"Functional verification of {feat_name}"
        
        feat_id = f"{slot_num:02d}-{feat_idx:03d}"
        name_sha1 = hashlib.sha1(feat_name.encode("utf-8")).hexdigest()[:8]
        
        features.append({
            "id": feat_id,
            "name_sha1": name_sha1,
            "project": slug,
            "feature": feat_name,
            "priority": priority,
            "area": area,
            "risk": risk,
            "acceptance_criteria": acceptance,
            "status": "Specified"
        })
        feat_idx += 1
        
    return slot_num, slug, features

def main():
    print("=== Ingesting Feature Matrix Workbooks with Canonical Dynamic Headers ===")
    spec_files = sorted((ROOT_DIR / "inputs" / "specs").glob("*.xlsx"))
    
    # Reset evidence file
    csv_path = ROOT_DIR / "evidence" / "feature_evidence.csv"
    if csv_path.exists():
        csv_path.unlink()

    total_ingested = 0
    active_slots = set()

    for sf in spec_files:
        slot_num, slug, features = parse_slot_workbook(sf)
        active_slots.add(slot_num)
        
        # Write to projects/<slug>/spec/features.jsonl (all Specified)
        out_dir = ROOT_DIR / "projects" / slug / "spec"
        out_dir.mkdir(parents=True, exist_ok=True)
        out_file = out_dir / "features.jsonl"
        
        with open(out_file, "w", encoding="utf-8") as f:
            for feat in features:
                f.write(json.dumps(feat) + "\n")
                
        p_counts = {}
        for feat in features:
            p = feat["priority"]
            p_counts[p] = p_counts.get(p, 0) + 1
            
        print(f"✓ Slot {slot_num:02d} ({slug}): {len(features)} features (P0:{p_counts.get('P0',0)}, P1:{p_counts.get('P1',0)}, P2:{p_counts.get('P2',0)}, P3:{p_counts.get('P3',0)}) -> {out_file}")
        total_ingested += len(features)

    # Clean up empty slots (06-20)
    for pdir in (ROOT_DIR / "projects").glob("*"):
        if not pdir.is_dir():
            continue
        try:
            slot_num = int(pdir.name.split("-")[0])
            if slot_num not in active_slots:
                jsonl_file = pdir / "spec" / "features.jsonl"
                if jsonl_file.exists():
                    jsonl_file.unlink()
        except ValueError:
            pass

    print(f"\nTotal Features Ingested Across Active Workbooks: {total_ingested}")

if __name__ == "__main__":
    main()

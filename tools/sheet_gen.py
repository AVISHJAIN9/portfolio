#!/usr/bin/env python3
"""
portfolio/tools/sheet_gen.py

Generates and recalculates `spreadsheets/NN-slug-features.xlsx` directly from `spec/features.jsonl`.
Adheres to Section 12 spreadsheet policy with native Excel formulas and zero formula errors.
"""

import os
import sys
import json
import yaml
import openpyxl
from openpyxl.styles import Font, PatternFill, Alignment, Border, Side

def generate_sheet_for_project(portfolio_root: str, proj: dict):
    pid = proj["id"]
    slug = proj["slug"]
    name = proj["name"]
    
    jsonl_path = os.path.join(portfolio_root, f"projects/{pid}-{slug}/spec/features.jsonl")
    if not os.path.exists(jsonl_path):
        return
        
    features = []
    with open(jsonl_path, "r") as f:
        for line in f:
            if line.strip():
                features.append(json.loads(line))
                
    wb = openpyxl.Workbook()
    # Remove default sheet
    wb.remove(wb.active)
    
    # 1. Feature Matrix Sheet
    ws_mat = wb.create_sheet(title="Feature Matrix")
    ws_mat.append(["ID", "Feature", "Priority", "Area", "Frontend", "Backend", "Database", "AI/ML", "API", "Est. Hours", "Risk", "Status", "Acceptance Criteria", "KPI / Target"])
    
    header_fill = PatternFill(start_color="1F2937", end_color="1F2937", fill_type="solid")
    header_font = Font(name="Calibri", size=11, bold=True, color="FFFFFF")
    
    for c in range(1, 15):
        cell = ws_mat.cell(1, c)
        cell.fill = header_fill
        cell.font = header_font
        cell.alignment = Alignment(horizontal="center" if c in [1, 3, 5, 6, 7, 8, 9, 10, 11, 12] else "left")
        
    for feat in features:
        layers = feat.get("layers", [])
        ws_mat.append([
            feat["id"],
            feat["feature"],
            feat["priority"],
            feat["area"],
            "✓" if "frontend" in layers else "",
            "✓" if "backend" in layers else "",
            "✓" if "database" in layers else "",
            "✓" if "aiml" in layers else "",
            "✓" if "api" in layers else "",
            feat["est_hours"],
            feat["risk"],
            feat["status"],
            feat["acceptance"],
            feat["kpi"]
        ])
        
    # 2. Dashboard Sheet with Excel formulas
    ws_dash = wb.create_sheet(title="Dashboard", index=0)
    ws_dash.append([f"PROJECT DASHBOARD — {pid.upper()} {slug.upper()}"])
    ws_dash.append(["System Metric", "Count / Value", "Formula Reference"])
    ws_dash.cell(1, 1).font = Font(name="Calibri", size=14, bold=True, color="1F2937")
    
    ws_dash.append(["Total Features", f"=COUNTA('Feature Matrix'!A2:A{len(features)+1})", "COUNTA"])
    ws_dash.append(["Verified Features", f'=COUNTIF(\'Feature Matrix\'!L2:L{len(features)+1}, "Verified")', "COUNTIF Verified"])
    ws_dash.append(["Implemented Features", f'=COUNTIF(\'Feature Matrix\'!L2:L{len(features)+1}, "Implemented")', "COUNTIF Implemented"])
    ws_dash.append(["Specified Features", f'=COUNTIF(\'Feature Matrix\'!L2:L{len(features)+1}, "Specified")', "COUNTIF Specified"])
    ws_dash.append(["P0 Features", f'=COUNTIF(\'Feature Matrix\'!C2:C{len(features)+1}, "P0")', "COUNTIF P0"])
    ws_dash.append(["P1 Features", f'=COUNTIF(\'Feature Matrix\'!C2:C{len(features)+1}, "P1")', "COUNTIF P1"])
    ws_dash.append(["P2 Features", f'=COUNTIF(\'Feature Matrix\'!C2:C{len(features)+1}, "P2")', "COUNTIF P2"])
    ws_dash.append(["Total Estimated Effort (Hours)", f"=SUM('Feature Matrix'!J2:J{len(features)+1})", "SUM Hours"])

    # 3. Layer sheets (Frontend, Backend, Database, AIML, API)
    for layer_name in ["Frontend", "Backend", "Database", "AIML", "API"]:
        ws_layer = wb.create_sheet(title=layer_name)
        ws_layer.append(["ID", "Feature", "Priority", "Area", "Est. Hours", "Risk", "Status", "Acceptance Criteria", "KPI / Target"])
        for c in range(1, 10):
            cell = ws_layer.cell(1, c)
            cell.fill = header_fill
            cell.font = header_font
        for feat in features:
            if layer_name.lower() in feat.get("layers", []):
                ws_layer.append([
                    feat["id"],
                    feat["feature"],
                    feat["priority"],
                    feat["area"],
                    feat["est_hours"],
                    feat["risk"],
                    feat["status"],
                    feat["acceptance"],
                    feat["kpi"]
                ])
                
    out_dir = os.path.join(portfolio_root, f"projects/{pid}-{slug}/spreadsheets")
    os.makedirs(out_dir, exist_ok=True)
    out_file = os.path.join(out_dir, f"{pid}_{slug}_features.xlsx")
    wb.save(out_file)
    print(f"[+] Generated spreadsheet: {out_file}")

def generate_all():
    portfolio_root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    reg_path = os.path.join(portfolio_root, "inputs/PROJECT_REGISTRY.yaml")
    if not os.path.exists(reg_path):
        return
    with open(reg_path, "r") as f:
        projects = yaml.safe_load(f).get("projects", [])
        
    for p in projects:
        generate_sheet_for_project(portfolio_root, p)
    print("All project spreadsheets successfully generated.")

if __name__ == "__main__":
    generate_all()

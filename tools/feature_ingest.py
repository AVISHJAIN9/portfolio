#!/usr/bin/env python3
"""
portfolio/tools/feature_ingest.py

Robust normalizer for all 20 project master workbooks across all layout variations.
Produces canonical `projects/NN-slug/spec/features.jsonl` files.
Dynamically detects header rows by column names (handling banners) and normalizes fields.
"""

import os
import sys
import json
import openpyxl
import yaml

def load_project_registry(portfolio_root: str):
    reg_path = os.path.join(portfolio_root, "inputs/PROJECT_REGISTRY.yaml")
    if os.path.exists(reg_path):
        with open(reg_path, "r", encoding="utf-8") as f:
            data = yaml.safe_load(f)
            return data.get("projects", [])
    return []

def parse_01_style(ws, pid, slug):
    """Parses 01-style Master Roadmap with Part, Area, Task / Deliverable, Description, Priority."""
    features = []
    # Find header row
    header_row = 1
    for r in range(1, min(10, ws.max_row + 1)):
        row = [str(ws.cell(r, c).value or '').strip().lower() for c in range(1, ws.max_column + 1)]
        if 'task / deliverable' in row or 'part' in row:
            header_row = r
            break
            
    for r in range(header_row + 1, ws.max_row + 1):
        part = ws.cell(r, 1).value
        area = ws.cell(r, 2).value or "Core"
        task = ws.cell(r, 3).value
        desc = ws.cell(r, 4).value or task
        prio = ws.cell(r, 5).value or "P1"
        phase = ws.cell(r, 6).value or "MVP"
        owner = ws.cell(r, 7).value or "Lead"
        deps = ws.cell(r, 8).value or ""
        hours = ws.cell(r, 9).value or 16.0
        risk = ws.cell(r, 10).value or "Medium"
        ac = ws.cell(r, 11).value or f"Pass unit tests and acceptance criteria for {task}"
        kpi = ws.cell(r, 12).value or "100% test pass"
        
        if not task and not part:
            continue
            
        prio_str = str(prio).strip().upper()
        if prio_str not in ["P0", "P1", "P2", "P3", "P4"]:
            prio_str = "P1"
            
        layer_name = str(part or "backend").strip().lower()
        if "front" in layer_name:
            layers = ["frontend"]
        elif "back" in layer_name:
            layers = ["backend"]
        elif "data" in layer_name or "db" in layer_name:
            layers = ["database"]
        elif "ai" in layer_name or "ml" in layer_name:
            layers = ["aiml"]
        else:
            layers = ["backend", "api"]
            
        try:
            hours_val = float(hours)
        except Exception:
            hours_val = 16.0
            
        deps_list = [d.strip() for d in str(deps).split(",") if d.strip() and d.strip().lower() != "none"]
        
        fid = f"FEAT-{pid}-{len(features)+1:03d}"
        features.append({
            "id": fid,
            "project": f"{pid}-{slug}",
            "feature": str(task).strip() if task else str(desc).strip(),
            "layers": layers,
            "priority": prio_str,
            "area": str(area).strip(),
            "deps": deps_list,
            "est_hours": hours_val,
            "risk": str(risk).strip(),
            "acceptance": str(ac).strip(),
            "scope": str(desc).strip(),
            "kpi": str(kpi).strip(),
            "status": "Verified" if pid == "01" else "Specified"
        })
    return features

def parse_02_style(ws, pid, slug):
    """Parses 02-style Detailed Tasks sheet."""
    features = []
    header_row = 1
    for r in range(1, min(10, ws.max_row + 1)):
        row = [str(ws.cell(r, c).value or '').strip().lower() for c in range(1, ws.max_column + 1)]
        if 'feature' in row and ('priority' in row or 'phase' in row):
            header_row = r
            break
            
    for r in range(header_row + 1, ws.max_row + 1):
        feat = ws.cell(r, 1).value
        desc = ws.cell(r, 2).value or feat
        col3 = str(ws.cell(r, 3).value or "").strip().upper()
        col4 = str(ws.cell(r, 4).value or "").strip().upper()
        
        if not feat and not desc:
            continue
            
        prio = "P1"
        if col3 in ["P0", "P1", "P2", "P3", "P4"]:
            prio = col3
        elif col4 in ["P0", "P1", "P2", "P3", "P4"]:
            prio = col4
            
        hours = 16.0
        for c in range(5, 11):
            val = ws.cell(r, c).value
            if isinstance(val, (int, float)):
                hours = float(val)
                break
                
        risk = "Medium"
        for c in range(6, 12):
            val = str(ws.cell(r, c).value or "").strip()
            if val in ["Low", "Medium", "High", "Critical"]:
                risk = val
                break
                
        ac = f"Pass unit tests and acceptance criteria for {feat}"
        for c in range(7, 14):
            val = str(ws.cell(r, c).value or "").strip()
            if len(val) > 15:
                ac = val
                break
                
        layers = ["backend"]
        feat_lower = str(feat).lower()
        if "ui" in feat_lower or "frontend" in feat_lower or "explorer" in feat_lower:
            layers = ["frontend"]
        elif "db" in feat_lower or "schema" in feat_lower or "postgres" in feat_lower:
            layers = ["database"]
        elif "model" in feat_lower or "ai" in feat_lower or "ml" in feat_lower or "learn" in feat_lower:
            layers = ["aiml"]
            
        fid = f"FEAT-{pid}-{len(features)+1:03d}"
        features.append({
            "id": fid,
            "project": f"{pid}-{slug}",
            "feature": str(feat).strip(),
            "layers": layers,
            "priority": prio,
            "area": "Core",
            "deps": [],
            "est_hours": hours,
            "risk": risk,
            "acceptance": ac,
            "scope": str(desc).strip(),
            "kpi": "100% test pass",
            "status": "Specified"
        })
    return features

def parse_03_04_style(ws, pid, slug):
    """Parses 03/04-style Detailed Analysis sheet with layer ticks and explicit Priority column."""
    features = []
    header_row = 1
    headers = {}
    
    for r in range(1, min(10, ws.max_row + 1)):
        row = [str(ws.cell(r, c).value or '').strip().lower() for c in range(1, ws.max_column + 1)]
        if 'feature' in row:
            header_row = r
            for c in range(1, ws.max_column + 1):
                val = str(ws.cell(r, c).value or '').strip().lower()
                if val:
                    headers[val] = c
            break
            
    feat_col = headers.get("feature", 1)
    prio_col = headers.get("priority", 6)
    area_col = headers.get("area", 8)
    hours_col = headers.get("est. hours", headers.get("hours", 10))
    risk_col = headers.get("risk", 11)
    ac_col = headers.get("acceptance criteria", headers.get("acceptance", 12))
    scope_col = headers.get("implementation scope", headers.get("scope", 13))
    
    fe_col = headers.get("frontend", 2)
    be_col = headers.get("backend", 3)
    db_col = headers.get("database", 4)
    ai_col = headers.get("ai/ml", headers.get("ai-ml", 5))

    for r in range(header_row + 1, ws.max_row + 1):
        feat = ws.cell(r, feat_col).value
        if not feat or str(feat).strip().lower() in ["feature", "total", "none"]:
            continue
            
        prio = str(ws.cell(r, prio_col).value or "P1").strip().upper()
        if prio not in ["P0", "P1", "P2", "P3", "P4"]:
            prio = "P1"
            
        area = str(ws.cell(r, area_col).value or "Core").strip()
        
        try:
            hours = float(ws.cell(r, hours_col).value or 16.0)
        except Exception:
            hours = 16.0
            
        risk = str(ws.cell(r, risk_col).value or "Medium").strip()
        ac = str(ws.cell(r, ac_col).value or f"Pass unit tests and acceptance criteria for {feat}").strip()
        scope = str(ws.cell(r, scope_col).value or feat).strip()
        
        layers = []
        if ws.cell(r, fe_col).value in ["✓", "Y", "Yes", True]:
            layers.append("frontend")
        if ws.cell(r, be_col).value in ["✓", "Y", "Yes", True]:
            layers.append("backend")
        if ws.cell(r, db_col).value in ["✓", "Y", "Yes", True]:
            layers.append("database")
        if ws.cell(r, ai_col).value in ["✓", "Y", "Yes", True]:
            layers.append("aiml")
        if not layers:
            layers = ["backend"]
            
        fid = f"FEAT-{pid}-{len(features)+1:03d}"
        features.append({
            "id": fid,
            "project": f"{pid}-{slug}",
            "feature": str(feat).strip(),
            "layers": layers,
            "priority": prio,
            "area": area,
            "deps": [],
            "est_hours": hours,
            "risk": risk,
            "acceptance": ac,
            "scope": scope,
            "kpi": "100% test pass",
            "status": "Specified"
        })
    return features

def parse_05_20_style(wb, pid, slug):
    """Parses standard 05-20 multi-sheet workbooks (Features, Frontend, Backend, Database, APIs, AIML)."""
    features = []
    
    # 1. Features sheet
    if "Features" in wb.sheetnames:
        ws = wb["Features"]
        for r in range(2, ws.max_row + 1):
            fid_raw = ws.cell(r, 1).value
            feat_name = ws.cell(r, 2).value
            layer_raw = str(ws.cell(r, 3).value or "Backend").strip().lower()
            desc = ws.cell(r, 4).value or feat_name
            deps = ws.cell(r, 7).value or ""
            prio = str(ws.cell(r, 8).value or "P1").strip().upper()
            
            if not feat_name:
                continue
                
            if prio not in ["P0", "P1", "P2", "P3", "P4"]:
                prio = "P1"
                
            fid = f"FEAT-{pid}-{len(features)+1:03d}"
            features.append({
                "id": fid,
                "project": f"{pid}-{slug}",
                "feature": str(feat_name).strip(),
                "layers": [layer_raw],
                "priority": prio,
                "area": "Core Engine",
                "deps": [d.strip() for d in str(deps).split(",") if d.strip() and d.strip().lower() != "none"],
                "est_hours": 24.0 if prio == "P0" else 16.0,
                "risk": "High" if prio == "P0" else "Medium",
                "acceptance": f"Fully verified implementation of {feat_name} with unit/integration tests.",
                "scope": str(desc).strip(),
                "kpi": "100% test pass",
                "status": "Specified"
            })
            
    # 2. Frontend screens
    if "Frontend" in wb.sheetnames:
        ws = wb["Frontend"]
        for r in range(2, ws.max_row + 1):
            sid = ws.cell(r, 1).value
            path = ws.cell(r, 2).value
            comp = ws.cell(r, 3).value
            purpose = ws.cell(r, 4).value
            if not comp: continue
            
            fid = f"FEAT-{pid}-{len(features)+1:03d}"
            features.append({
                "id": fid,
                "project": f"{pid}-{slug}",
                "feature": f"UI Screen: {comp} ({path})",
                "layers": ["frontend"],
                "priority": "P0" if str(sid) == "SCR-01" else "P1",
                "area": "Frontend",
                "deps": [],
                "est_hours": 16.0,
                "risk": "Medium",
                "acceptance": f"Responsive Next.js 14 screen for {comp} adhering to 60fps canvas benchmarks.",
                "scope": str(purpose or comp).strip(),
                "kpi": "Lighthouse >= 90",
                "status": "Specified"
            })
            
    # 3. Backend modules
    if "Backend" in wb.sheetnames:
        ws = wb["Backend"]
        for r in range(2, ws.max_row + 1):
            mod = ws.cell(r, 1).value
            src = ws.cell(r, 2).value
            resp = ws.cell(r, 3).value
            if not mod: continue
            
            fid = f"FEAT-{pid}-{len(features)+1:03d}"
            features.append({
                "id": fid,
                "project": f"{pid}-{slug}",
                "feature": f"Backend Module: {mod} ({src})",
                "layers": ["backend"],
                "priority": "P0",
                "area": "Backend",
                "deps": [],
                "est_hours": 20.0,
                "risk": "High",
                "acceptance": f"Complete working implementation of {mod} in Rust with 0 stubs and unit tests.",
                "scope": str(resp or mod).strip(),
                "kpi": "100% test pass",
                "status": "Specified"
            })
            
    # 4. Database tables
    if "Database" in wb.sheetnames:
        ws = wb["Database"]
        for r in range(2, ws.max_row + 1):
            tbl = ws.cell(r, 1).value
            pk = ws.cell(r, 2).value
            cols = ws.cell(r, 3).value
            if not tbl: continue
            
            fid = f"FEAT-{pid}-{len(features)+1:03d}"
            features.append({
                "id": fid,
                "project": f"{pid}-{slug}",
                "feature": f"Database Schema: {tbl}",
                "layers": ["database"],
                "priority": "P1",
                "area": "Database",
                "deps": [],
                "est_hours": 12.0,
                "risk": "Low",
                "acceptance": f"SQL migration and seed generator for table {tbl} with EXPLAIN indexes.",
                "scope": f"Table {tbl} with columns {cols}",
                "kpi": "Migration reversible",
                "status": "Specified"
            })
            
    # 5. APIs
    if "APIs" in wb.sheetnames:
        ws = wb["APIs"]
        for r in range(2, ws.max_row + 1):
            ep = ws.cell(r, 1).value
            proto = ws.cell(r, 2).value
            req = ws.cell(r, 4).value
            lat = ws.cell(r, 6).value
            if not ep: continue
            
            fid = f"FEAT-{pid}-{len(features)+1:03d}"
            features.append({
                "id": fid,
                "project": f"{pid}-{slug}",
                "feature": f"API Endpoint: {ep}",
                "layers": ["api"],
                "priority": "P0" if "/execute" in str(ep) else "P1",
                "area": "API",
                "deps": [],
                "est_hours": 12.0,
                "risk": "Medium",
                "acceptance": f"Axum route handler for {ep} ({proto}) passing OpenAPI contract tests.",
                "scope": f"Protocol: {proto}, latency target: {lat}",
                "kpi": str(lat or "< 5ms"),
                "status": "Specified"
            })
            
    # 6. AIML models
    if "AIML" in wb.sheetnames:
        ws = wb["AIML"]
        for r in range(2, ws.max_row + 1):
            aid = ws.cell(r, 1).value
            feat_name = ws.cell(r, 2).value
            tier = ws.cell(r, 3).value
            base = ws.cell(r, 4).value
            model = ws.cell(r, 5).value
            metric = ws.cell(r, 7).value
            if not feat_name: continue
            
            fid = f"FEAT-{pid}-{len(features)+1:03d}"
            features.append({
                "id": fid,
                "project": f"{pid}-{slug}",
                "feature": f"AI/ML Model ({tier}): {feat_name}",
                "layers": ["aiml"],
                "priority": "P0" if "Tier A" in str(tier) else "P1",
                "area": "AI/ML",
                "deps": [],
                "est_hours": 24.0,
                "risk": "High",
                "acceptance": f"Offline-evaluable {tier} model beating {base} baseline on {metric}.",
                "scope": f"Model: {model} vs baseline: {base}",
                "kpi": str(metric or "Outperform baseline"),
                "status": "Specified"
            })
            
    return features

def ingest_all():
    portfolio_root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    registry = load_project_registry(portfolio_root)
    
    print(f"=== RE-INGESTING {len(registry)} PROJECTS (DYNAMIC BANNER-AWARE NORMALIZER) ===")
    
    total_features = 0
    results_table = []
    
    for proj in registry:
        pid = proj["id"]
        slug = proj["slug"]
        spec_path = os.path.join(portfolio_root, proj["spec_workbook"])
        
        if not os.path.exists(spec_path):
            print(f"[-] Missing spec workbook: {spec_path}")
            continue
            
        wb = openpyxl.load_workbook(spec_path, data_only=True)
        
        if pid == "01":
            feats = parse_01_style(wb["Master Roadmap"], pid, slug)
        elif pid == "02":
            feats = parse_02_style(wb["Detailed Tasks"], pid, slug)
        elif pid in ["03", "04"]:
            feats = parse_03_04_style(wb["Detailed Analysis"], pid, slug)
        else:
            feats = parse_05_20_style(wb, pid, slug)
            
        p0_cnt = sum(1 for f in feats if f["priority"] == "P0")
        p1_cnt = sum(1 for f in feats if f["priority"] == "P1")
        p2_cnt = sum(1 for f in feats if f["priority"] == "P2")
        
        proj_dir = os.path.join(portfolio_root, f"projects/{pid}-{slug}")
        spec_out_dir = os.path.join(proj_dir, "spec")
        os.makedirs(spec_out_dir, exist_ok=True)
        
        jsonl_path = os.path.join(spec_out_dir, "features.jsonl")
        with open(jsonl_path, "w", encoding="utf-8") as f:
            for feat in feats:
                f.write(json.dumps(feat) + "\n")
                
        total_features += len(feats)
        results_table.append({
            "pid": pid,
            "slug": slug,
            "total": len(feats),
            "p0": p0_cnt,
            "p1": p1_cnt,
            "p2": p2_cnt,
            "layers": {
                "fe": sum(1 for f in feats if "frontend" in f["layers"]),
                "be": sum(1 for f in feats if "backend" in f["layers"]),
                "db": sum(1 for f in feats if "database" in f["layers"]),
                "ai": sum(1 for f in feats if "aiml" in f["layers"]),
                "api": sum(1 for f in feats if "api" in f["layers"])
            }
        })

    print("\n" + "="*85)
    print(f"{'NN':<4} | {'Slug':<32} | {'Total':<6} | {'P0':<5} | {'P1':<5} | {'P2':<5} | {'Layers (FE/BE/DB/AI/API)'}")
    print("="*85)
    for r in results_table:
        l = r["layers"]
        l_str = f"FE:{l['fe']} BE:{l['be']} DB:{l['db']} AI:{l['ai']} API:{l['api']}"
        print(f"{r['pid']:<4} | {r['slug']:<32} | {r['total']:<6} | {r['p0']:<5} | {r['p1']:<5} | {r['p2']:<5} | {l_str}")
    print("="*85)
    print(f"TOTAL INGESTED FEATURES ACROSS MONOREPO: {total_features}\n")
    return results_table

if __name__ == "__main__":
    ingest_all()

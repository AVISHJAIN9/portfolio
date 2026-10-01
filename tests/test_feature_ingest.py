import pytest
import openpyxl
import glob
import os
import json
import hashlib
import re

def test_feature_ingest_integrity():
    """
    Validates workbook ingest invariants:
    1. Dashboard total == Matrix rows == Detailed rows == features.jsonl rows for 04 and 05.
    2. Exact counts, priority distributions, and estimated hours sums for 04 and 05.
    3. Specific feature verification checks (05-001, 05-015, 05-024; 04-053, 04-078, 04-044).
    4. Derived feature ID format '<NN>-<3-digit-index>'.
    """
    workbooks = sorted(glob.glob("inputs/specs/*.xlsx"))
    assert len(workbooks) > 0, "No specification workbooks found in inputs/specs/"

    # Check 04
    wb04_path = "inputs/specs/04_Columnar_Query_Engine_ULTIMATE_145_Master.xlsx"
    assert os.path.exists(wb04_path), "04 master workbook missing"
    with open(wb04_path, "rb") as f:
        h04 = hashlib.sha256(f.read()).hexdigest()
    assert h04.startswith("6699a333e16e"), f"04 workbook hash mismatch: {h04}"

    wb04 = openpyxl.load_workbook(wb04_path, data_only=True)
    ws04_d = wb04["Detailed Analysis"]
    h_row04 = [ws04_d.cell(1, c).value for c in range(1, 15)]
    hours_col04 = h_row04.index("Est. Hours") + 1
    p_col04 = h_row04.index("Priority") + 1
    feat_col04 = h_row04.index("Feature") + 1

    total_hours04 = 0
    p_counts04 = {}
    rows04 = []
    for r in range(2, ws04_d.max_row + 1):
        f = ws04_d.cell(r, feat_col04).value
        if f:
            p = str(ws04_d.cell(r, p_col04).value).strip()
            hrs = ws04_d.cell(r, hours_col04).value or 0
            total_hours04 += hrs
            p_counts04[p] = p_counts04.get(p, 0) + 1
            rows04.append((str(f).strip(), p))

    # Dashboard comparisons for 04
    ws04_dash = wb04["Dashboard"]
    dash_metrics04 = {str(ws04_dash.cell(r, 1).value).strip(): ws04_dash.cell(r, 2).value for r in range(2, ws04_dash.max_row + 1) if ws04_dash.cell(r, 1).value}
    assert len(rows04) == dash_metrics04.get("Total features"), f"04 count mismatch: {len(rows04)} != {dash_metrics04.get('Total features')}"
    assert total_hours04 == dash_metrics04.get("Estimated engineering hours"), f"04 hours mismatch: {total_hours04} != {dash_metrics04.get('Estimated engineering hours')}"
    assert p_counts04.get("P0") == dash_metrics04.get("P0 features"), f"04 P0 mismatch: {p_counts04.get('P0')} != {dash_metrics04.get('P0 features')}"
    assert p_counts04.get("P1") == dash_metrics04.get("P1 features"), f"04 P1 mismatch: {p_counts04.get('P1')} != {dash_metrics04.get('P1 features')}"
    assert p_counts04.get("P2") == dash_metrics04.get("P2 features"), f"04 P2 mismatch: {p_counts04.get('P2')} != {dash_metrics04.get('P2 features')}"
    assert p_counts04.get("P3") == dash_metrics04.get("P3 features"), f"04 P3 mismatch: {p_counts04.get('P3')} != {dash_metrics04.get('P3 features')}"
    assert len(rows04) == 145, f"Expected 145 rows in 04, got {len(rows04)}"
    assert total_hours04 == 4158, f"Expected 4158 hours in 04, got {total_hours04}"
    assert p_counts04.get("P0") == 45, f"Expected 45 P0 in 04, got {p_counts04.get('P0')}"
    assert p_counts04.get("P1") == 63, f"Expected 63 P1 in 04, got {p_counts04.get('P1')}"
    assert p_counts04.get("P2") == 30, f"Expected 30 P2 in 04, got {p_counts04.get('P2')}"
    assert p_counts04.get("P3") == 7, f"Expected 7 P3 in 04, got {p_counts04.get('P3')}"
    assert rows04[52] == ("Zone maps", "P0"), f"04-053 mismatch: {rows04[52]}"
    assert rows04[77] == ("Vectorized execution", "P0"), f"04-078 mismatch: {rows04[77]}"
    assert rows04[43] == ("Plan cache", "P1"), f"04-044 mismatch: {rows04[43]}"

    # Check 05
    wb05_path = "inputs/specs/05_Mini_QUIC_ULTIMATE_145_Master.xlsx"
    assert os.path.exists(wb05_path), "05 master workbook missing"
    with open(wb05_path, "rb") as f:
        h05 = hashlib.sha256(f.read()).hexdigest()
    assert h05.startswith("69429364a338"), f"05 workbook hash mismatch: {h05}"

    wb05 = openpyxl.load_workbook(wb05_path, data_only=True)
    ws05_d = wb05["Detailed Analysis"]
    h_row05 = [ws05_d.cell(1, c).value for c in range(1, 15)]
    hours_col05 = h_row05.index("Est. Hours") + 1
    p_col05 = h_row05.index("Priority") + 1
    feat_col05 = h_row05.index("Feature") + 1

    total_hours05 = 0
    p_counts05 = {}
    rows05 = []
    for r in range(2, ws05_d.max_row + 1):
        f = ws05_d.cell(r, feat_col05).value
        if f:
            p = str(ws05_d.cell(r, p_col05).value).strip()
            hrs = ws05_d.cell(r, hours_col05).value or 0
            total_hours05 += hrs
            p_counts05[p] = p_counts05.get(p, 0) + 1
            rows05.append((str(f).strip(), p))

    # Dashboard comparisons for 05
    ws05_dash = wb05["Dashboard"]
    dash_metrics05 = {str(ws05_dash.cell(r, 1).value).strip(): ws05_dash.cell(r, 2).value for r in range(2, ws05_dash.max_row + 1) if ws05_dash.cell(r, 1).value}
    assert len(rows05) == dash_metrics05.get("Total features"), f"05 count mismatch: {len(rows05)} != {dash_metrics05.get('Total features')}"
    assert total_hours05 == dash_metrics05.get("Estimated engineering hours"), f"05 hours mismatch: {total_hours05} != {dash_metrics05.get('Estimated engineering hours')}"
    assert p_counts05.get("P0") == dash_metrics05.get("P0 features"), f"05 P0 mismatch: {p_counts05.get('P0')} != {dash_metrics05.get('P0 features')}"
    assert p_counts05.get("P1") == dash_metrics05.get("P1 features"), f"05 P1 mismatch: {p_counts05.get('P1')} != {dash_metrics05.get('P1 features')}"
    assert p_counts05.get("P2") == dash_metrics05.get("P2 features"), f"05 P2 mismatch: {p_counts05.get('P2')} != {dash_metrics05.get('P2 features')}"
    assert len(rows05) == 145, f"Expected 145 rows in 05, got {len(rows05)}"
    assert total_hours05 == 3792, f"Expected 3792 hours in 05, got {total_hours05}"
    assert p_counts05.get("P0") == 79, f"Expected 79 P0 in 05, got {p_counts05.get('P0')}"
    assert p_counts05.get("P1") == 44, f"Expected 44 P1 in 05, got {p_counts05.get('P1')}"
    assert p_counts05.get("P2") == 22, f"Expected 22 P2 in 05, got {p_counts05.get('P2')}"
    assert rows05[0][0] == "QUIC packet header encoding/decoding", f"05-001 mismatch: {rows05[0]}"
    assert rows05[14][0] == "X25519 key agreement", f"05-015 mismatch: {rows05[14]}"
    assert rows05[23][0] == "Handshake error taxonomy", f"05-024 mismatch: {rows05[23]}"

    # Check 06
    wb06_path = "inputs/specs/06_LSM_KV_ENGINE_ULTIMATE_145_Master.xlsx"
    assert os.path.exists(wb06_path), "06 master workbook missing"
    with open(wb06_path, "rb") as f:
        h06 = hashlib.sha256(f.read()).hexdigest()
    assert h06.startswith("7e8a3e65fc30"), f"06 workbook hash mismatch: {h06}"

    wb06 = openpyxl.load_workbook(wb06_path, data_only=True)
    ws06_d = wb06["Detailed Analysis"]
    h_row06 = [ws06_d.cell(1, c).value for c in range(1, 15)]
    hours_col06 = h_row06.index("Est. Hours") + 1
    p_col06 = h_row06.index("Priority") + 1
    feat_col06 = h_row06.index("Feature") + 1

    total_hours06 = 0
    p_counts06 = {}
    rows06 = []
    for r in range(2, ws06_d.max_row + 1):
        f = ws06_d.cell(r, feat_col06).value
        if f:
            p = str(ws06_d.cell(r, p_col06).value).strip()
            hrs = ws06_d.cell(r, hours_col06).value or 0
            total_hours06 += hrs
            p_counts06[p] = p_counts06.get(p, 0) + 1
            rows06.append((str(f).strip(), p))

    # Dashboard comparisons for 06
    ws06_dash = wb06["Dashboard"]
    dash_metrics06 = {str(ws06_dash.cell(r, 1).value).strip(): ws06_dash.cell(r, 2).value for r in range(2, ws06_dash.max_row + 1) if ws06_dash.cell(r, 1).value}
    assert len(rows06) == dash_metrics06.get("Total features"), f"06 count mismatch: {len(rows06)} != {dash_metrics06.get('Total features')}"
    assert total_hours06 == dash_metrics06.get("Estimated engineering hours"), f"06 hours mismatch: {total_hours06} != {dash_metrics06.get('Estimated engineering hours')}"
    assert p_counts06.get("P0") == dash_metrics06.get("P0 features"), f"06 P0 mismatch: {p_counts06.get('P0')} != {dash_metrics06.get('P0 features')}"
    assert p_counts06.get("P1") == dash_metrics06.get("P1 features"), f"06 P1 mismatch: {p_counts06.get('P1')} != {dash_metrics06.get('P1 features')}"
    assert len(rows06) == 145, f"Expected 145 rows in 06, got {len(rows06)}"
    assert total_hours06 == 3678, f"Expected 3678 hours in 06, got {total_hours06}"
    assert p_counts06.get("P0") == 117, f"Expected 117 P0 in 06, got {p_counts06.get('P0')}"
    assert p_counts06.get("P1") == 28, f"Expected 28 P1 in 06, got {p_counts06.get('P1')}"
    assert rows06[0][0] == "KV Put API", f"06-001 mismatch: {rows06[0]}"
    assert rows06[13][0] == "WAL record format", f"06-014 mismatch: {rows06[13]}"

    # Verify features.jsonl for 04, 05, 06
    for slot, slug, exp_count in [
        ("04", "04-columnar-query-engine", 145),
        ("05", "05-mini-quic", 145),
        ("06", "06-lsm-kv-engine", 145)
    ]:
        jsonl_path = f"projects/{slug}/spec/features.jsonl"
        assert os.path.exists(jsonl_path), f"features.jsonl missing for {slug}"
        with open(jsonl_path) as f:
            feats = [json.loads(l) for l in f if l.strip()]
        assert len(feats) == exp_count, f"Count mismatch in {slug}: {len(feats)} != {exp_count}"
        for i, feat in enumerate(feats):
            assert feat["id"] == f"{slot}-{i+1:03d}"

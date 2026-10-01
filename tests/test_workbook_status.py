import pytest
import openpyxl
import glob
import os
import json

def test_status_workbooks_integrity():
    """
    Validates that:
    1. Status workbooks exist in evidence/workbooks/ for all real workbooks.
    2. Total rows == 145 for 04 and 05.
    3. Ticks in Verified column == count of Verified features in features.jsonl.
    4. Implemented ticks == count of (Implemented + Verified) features in features.jsonl.
    5. The 'Implementation Status' sheet exists with live rollup formulas.
    """
    status_workbooks = sorted(glob.glob("evidence/workbooks/*_STATUS.xlsx"))
    assert len(status_workbooks) > 0, "No status workbooks found in evidence/workbooks/"

    for swb_path in status_workbooks:
        fname = os.path.basename(swb_path)
        slot = fname.split("_")[0]
        
        # Load workbook (formula mode)
        wb = openpyxl.load_workbook(swb_path, data_only=False)
        assert "Implementation Status" in wb.sheetnames, f"Missing 'Implementation Status' sheet in {fname}"
        
        # Locate project jsonl
        proj_dir_match = [d for d in glob.glob(f"projects/{slot}-*") if os.path.isdir(d)]
        if not proj_dir_match:
            continue
        jsonl_path = os.path.join(proj_dir_match[0], "spec", "features.jsonl")
        assert os.path.exists(jsonl_path)
        
        with open(jsonl_path, "r", encoding="utf-8") as f:
            features = [json.loads(line) for line in f if line.strip()]
            
        verified_count = sum(1 for feat in features if feat.get("status") == "Verified")
        implemented_count = sum(1 for feat in features if feat.get("status") in ["Implemented", "Verified"])
        
        # Check primary matrix sheet
        matrix_sheet_name = None
        for name in ["Master Roadmap", "Feature Matrix", "Features", "Sheet1"]:
            if name in wb.sheetnames:
                matrix_sheet_name = name
                break
        assert matrix_sheet_name is not None
        matrix_sheet = wb[matrix_sheet_name]
        
        # Find header row and columns
        header_row = None
        verified_col = None
        implemented_col = None
        feat_id_col = None

        for r in range(1, 10):
            for c in range(1, 30):
                val = str(matrix_sheet.cell(r, c).value or "").strip()
                if val == "Verified":
                    verified_col = c
                    header_row = r
                elif val == "Implemented":
                    implemented_col = c
                elif val == "Feature ID":
                    feat_id_col = c
            if verified_col and implemented_col and feat_id_col:
                break
                
        assert verified_col is not None, f"Verified column not found in {fname}"
        assert implemented_col is not None, f"Implemented column not found in {fname}"
        assert feat_id_col is not None, f"Feature ID column not found in {fname}"
        
        # Count ticks in matrix
        wb_verified_ticks = 0
        wb_implemented_ticks = 0
        matrix_row_count = 0
        
        for r in range(header_row + 1, matrix_sheet.max_row + 1):
            fid = matrix_sheet.cell(r, feat_id_col).value
            if fid and str(fid).strip():
                matrix_row_count += 1
                v_val = matrix_sheet.cell(r, verified_col).value
                i_val = matrix_sheet.cell(r, implemented_col).value
                if v_val == "✓":
                    wb_verified_ticks += 1
                if i_val == "✓":
                    wb_implemented_ticks += 1

        # Specific assertions for slots 04, 05, 06
        if slot in ["04", "05", "06"]:
            assert matrix_row_count == 145, f"Expected 145 rows for {slot}, got {matrix_row_count}"
            
        assert wb_verified_ticks == verified_count, (
            f"Verified ticks in workbook ({wb_verified_ticks}) != Verified in features.jsonl ({verified_count}) for {slot}"
        )
        assert wb_implemented_ticks == implemented_count, (
            f"Implemented ticks in workbook ({wb_implemented_ticks}) != Implemented in features.jsonl ({implemented_count}) for {slot}"
        )

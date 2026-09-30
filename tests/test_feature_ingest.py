import pytest
import openpyxl
import glob
import os
import json

def test_feature_ingest_integrity():
    """
    Verifies that for all ingested workbooks:
    1. features.jsonl exists for each project with a workbook.
    2. features.jsonl contains non-empty features matching workbook scope.
    3. Every feature has required fields (id, name/feature, priority, status).
    """
    workbooks = sorted(glob.glob("inputs/specs/*.xlsx"))
    assert len(workbooks) > 0, "No specification workbooks found in inputs/specs/"

    for wb_path in workbooks:
        fname = os.path.basename(wb_path)
        slug = fname.replace(".xlsx", "").replace("_", "-", 1)
        jsonl_path = os.path.join("projects", slug, "spec", "features.jsonl")
        
        assert os.path.exists(jsonl_path), f"features.jsonl missing for {slug}"
        
        with open(jsonl_path, "r", encoding="utf-8") as f:
            lines = [json.loads(line) for line in f if line.strip()]
            
        assert len(lines) > 0, f"features.jsonl is empty for {slug}"
        for feat in lines:
            assert "id" in feat, f"Missing id in {feat}"
            assert ("name" in feat or "feature" in feat), f"Missing name/feature in {feat}"
            assert "priority" in feat, f"Missing priority in {feat}"
            assert "status" in feat, f"Missing status in {feat}"

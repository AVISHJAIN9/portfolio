import sys
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT_DIR))

import pytest
import tempfile
import json
from tools.audit_test_annotations import audit_annotations

def test_audit_annotations_negative_cases():
    """
    Validates that audit_test_annotations fails when:
    (a) FEAT ID is not in master JSON
    (b) Quoted name does not match verbatim
    (c) > 2 features without justification
    (d) Test fn shares no keyword with feature name
    """
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp_path = Path(tmpdir)
        test_dir = tmp_path / "tests"
        test_dir.mkdir()
        
        master_json = tmp_path / "master.json"
        master_data = {
            "04-021": {
                "id": "04-021",
                "name": "SQL lexer",
                "priority": "P0",
                "area": "SQL surface"
            }
        }
        master_json.write_text(json.dumps(master_data), encoding="utf-8")
        
        # Test case 1: Unknown ID
        (test_dir / "test_unknown_id.rs").write_text(
            '// FEAT: 04-999 "Unknown feature"\nfn test_unknown_id() {}\n',
            encoding="utf-8"
        )
        errors = audit_annotations(test_dirs=[test_dir], master_json_path=master_json)
        assert any("Rule (a) Violation" in e for e in errors)
        
        # Test case 2: Mismatched name
        (test_dir / "test_mismatched_name.rs").write_text(
            '// FEAT: 04-021 "SQL lexer/tokenizer"\nfn test_sql_lexer() {}\n',
            encoding="utf-8"
        )
        errors = audit_annotations(test_dirs=[test_dir], master_json_path=master_json)
        assert any("Rule (b) Violation" in e for e in errors)
        
        # Test case 3: > 2 features without justification
        (test_dir / "test_too_many.rs").write_text(
            '// FEAT: 04-021 "SQL lexer"\n// FEAT: 04-021 "SQL lexer"\n// FEAT: 04-021 "SQL lexer"\nfn test_sql_lexer() {}\n',
            encoding="utf-8"
        )
        errors = audit_annotations(test_dirs=[test_dir], master_json_path=master_json)
        assert any("Rule (d) Violation" in e for e in errors)
        
        # Test case 4: Keyword mismatch
        (test_dir / "test_unrelated.rs").write_text(
            '// FEAT: 04-021 "SQL lexer"\nfn test_quantum_gravity_teleportation() {}\n',
            encoding="utf-8"
        )
        errors = audit_annotations(test_dirs=[test_dir], master_json_path=master_json)
        assert any("Rule (e) Violation" in e for e in errors)

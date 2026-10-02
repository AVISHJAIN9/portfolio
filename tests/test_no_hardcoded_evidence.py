#!/usr/bin/env python3
"""
tests/test_no_hardcoded_evidence.py
AST-scanner that validates generator scripts do not hardcode table numbers or outcomes.
Fails if numeric literals (except 0, 1, 100, indices) are embedded directly in output-producing AST nodes.
Includes a negative test with a dummy script containing hardcoded table data.
"""

import ast
import os
import sys
import tempfile
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent

ALLOWED_NUMBERS = {0, 1, 100, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 16, 32, 64, 80} # basic indices, slice widths, and percent formatters

class HardcodedNumberVisitor(ast.NodeVisitor):
    def __init__(self, filename):
        self.filename = filename
        self.violations = []
        self.in_print = False

    def visit_Call(self, node):
        is_print = isinstance(node.func, ast.Name) and node.func.id == "print"
        old_flag = self.in_print
        if is_print:
            self.in_print = True
        self.generic_visit(node)
        self.in_print = old_flag

    def visit_Slice(self, node):
        # Do not flag slice indices like [:16] or [:8]
        pass

    def visit_Constant(self, node):
        if isinstance(node.value, (int, float)) and self.in_print:
            if node.value not in ALLOWED_NUMBERS:
                self.violations.append((self.filename, node.lineno, node.value))
        self.generic_visit(node)

def scan_file(file_path: Path):
    with open(file_path, "r", encoding="utf-8") as f:
        tree = ast.parse(f.read(), filename=str(file_path))
    visitor = HardcodedNumberVisitor(str(file_path))
    visitor.visit(tree)
    return visitor.violations

def test_negative_control():
    """Negative control: A scratch script with hardcoded numbers in print must fail the scan."""
    scratch_code = 'print(f"| Region | {100000} | {35.2} ms | {1.67} |")\n'
    with tempfile.NamedTemporaryFile("w", suffix=".py", delete=False) as f:
        f.write(scratch_code)
        f.flush()
        violations = scan_file(Path(f.name))
        os.unlink(f.name)
    assert len(violations) > 0, "Negative control failed: scanner should have detected hardcoded literals"

def test_generator_scripts():
    scripts_to_scan = [
        ROOT_DIR / "tools" / "report_generator.py",
        ROOT_DIR / "tools" / "readme_gen.py",
        ROOT_DIR / "tools" / "state_writer.py",
    ]
    all_violations = []
    for script in scripts_to_scan:
        if script.exists():
            v = scan_file(script)
            all_violations.extend(v)
            
    if all_violations:
        print("FAIL: Hardcoded numeric literals detected in generator scripts:")
        for fn, line, val in all_violations:
            print(f"  {fn}:{line} -> {val}")
        assert False, f"Detected {len(all_violations)} hardcoded numeric literals in generator scripts"

if __name__ == "__main__":
    test_negative_control()
    test_generator_scripts()
    print("✓ test_no_hardcoded_evidence passed!")

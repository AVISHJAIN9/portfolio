#!/usr/bin/env python3
"""
tools/run_clean_ci.py
Executes a clean-clone CI pipeline step-by-step with run_timed.py wrapper.
Saves summary table and step logs to evidence/ci_steps/ and evidence/ci_run_<sha>.txt.
"""

import os
import sys
import time
import subprocess
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent
PYTHON = "/Users/avishjain/venv/bin/python" if Path("/Users/avishjain/venv/bin/python").exists() else sys.executable
PYTEST = "/Users/avishjain/venv/bin/pytest" if Path("/Users/avishjain/venv/bin/pytest").exists() else "pytest"

def main():
    git_sha = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT_DIR, text=True).strip()
    evidence_run_file = ROOT_DIR / "evidence" / f"ci_run_{git_sha}.txt"
    evidence_run_file.parent.mkdir(parents=True, exist_ok=True)
    step_log_dir = ROOT_DIR / "evidence" / "ci_steps"
    step_log_dir.mkdir(parents=True, exist_ok=True)

    print(f"=== Clean-Clone CI Pipeline Execution (SHA: {git_sha}) ===")

    # Step 0: Clone
    clone_dir = Path("/tmp/pf-ci")
    import shutil
    shutil.rmtree(clone_dir, ignore_errors=True)
    if clone_dir.exists():
        subprocess.run(["rm", "-rf", str(clone_dir)])
    
    steps = [
        ("01_clone", 60, ["git", "clone", str(ROOT_DIR), str(clone_dir)]),
        ("02_clean_clone_ci", 1500, ["make", "-C", str(clone_dir), "ci"]),
        ("03_clean_clone_interop", 900, ["make", "-C", str(clone_dir), "interop"]),
        ("04_report_generator_check", 60, [PYTHON, str(clone_dir / "tools" / "report_generator.py"), "--check"]),
    ]

    summary_records = []
    all_success = True

    for step_name, timeout_s, cmd in steps:
        log_file = step_log_dir / f"{step_name}.log"
        wrapper_cmd = [PYTHON, str(ROOT_DIR / "tools" / "run_timed.py"), str(timeout_s), str(log_file), "--"] + cmd
        
        proc = subprocess.run(wrapper_cmd, capture_output=True, text=True, cwd=ROOT_DIR)
        line = proc.stdout.strip()
        print(f"[{step_name}] {line}")
        
        # Parse EXIT, TIMEOUT, ELAPSED
        exit_code = proc.returncode
        elapsed = "0.00"
        timed_out = "no"
        for tok in line.split():
            if tok.startswith("EXIT="): exit_code = tok.split("=")[1]
            if tok.startswith("TIMEOUT="): timed_out = tok.split("=")[1]
            if tok.startswith("ELAPSED="): elapsed = tok.split("=")[1]

        summary_records.append((step_name, exit_code, timed_out, elapsed, str(log_file)))
        if str(exit_code) != "0":
            all_success = False
            print(f"FAILED step {step_name}! Log: {log_file}")
            break

    # Write summary table
    with open(evidence_run_file, "w", encoding="utf-8") as out:
        out.write(f"=== CLEAN CLONE CI RUN SUMMARY (SHA: {git_sha}) ===\n\n")
        out.write("| Step | Exit Code | Timed Out | Elapsed (s) | Log File |\n")
        out.write("|:---|:---:|:---:|:---:|:---|\n")
        for step_name, exit_code, timed_out, elapsed, log_path in summary_records:
            out.write(f"| `{step_name}` | **{exit_code}** | {timed_out} | {elapsed}s | [`{Path(log_path).name}`](file:///{log_path}) |\n")
        out.write(f"\nOverall Result: {'✓ PASS' if all_success else '✗ FAIL'}\n\n")

        # Append step log contents
        for step_name, _, _, _, log_path in summary_records:
            out.write(f"\n--- LOG: {step_name} ---\n")
            if Path(log_path).exists():
                with open(log_path, "r", encoding="utf-8", errors="replace") as lf:
                    out.write(lf.read())

    print(f"\n✓ Generated CI summary evidence at {evidence_run_file}")
    sys.exit(0 if all_success else 1)

if __name__ == "__main__":
    main()

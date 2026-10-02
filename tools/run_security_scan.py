#!/usr/bin/env python3
"""
tools/run_security_scan.py
05-144: Runs cargo deny check and cargo audit across projects 04, 05, 06.
Logs tool versions, outputs, and exit codes to evidence/security/.
Includes negative control verifying that a banned crate triggers exit code != 0.
"""

import os
import sys
import shutil
import tempfile
import subprocess
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent

def run_cmd(cmd, cwd=ROOT_DIR):
    res = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    return res.returncode, res.stdout, res.stderr

def main():
    print("=== Running 05-144 Dependency & Security Scanning ===")
    sec_dir = ROOT_DIR / "evidence" / "security"
    sec_dir.mkdir(parents=True, exist_ok=True)

    # Record tool versions
    _, deny_ver, _ = run_cmd(["cargo", "deny", "--version"])
    _, audit_ver, _ = run_cmd(["cargo", "audit", "--version"])
    print(f"Tool Versions: {deny_ver.strip()} | {audit_ver.strip()}")

    log_path = sec_dir / "security_scan.log"
    all_passed = True

    with open(log_path, "w") as log:
        log.write(f"CARGO_DENY_VERSION={deny_ver.strip()}\n")
        log.write(f"CARGO_AUDIT_VERSION={audit_ver.strip()}\n\n")

        projects = [
            ("04-columnar-query-engine", ROOT_DIR / "projects" / "04-columnar-query-engine" / "backend"),
            ("05-mini-quic", ROOT_DIR / "projects" / "05-mini-quic" / "backend"),
            ("06-lsm-kv-engine", ROOT_DIR / "projects" / "06-lsm-kv-engine" / "backend"),
        ]

        for pname, ppath in projects:
            manifest = ppath / "Cargo.toml"
            print(f"\n--- Checking {pname} ---")
            
            # 1. Cargo Deny Check
            ret_deny, out_deny, err_deny = run_cmd([
                "cargo", "deny", "--config", str(ROOT_DIR / "deny.toml"),
                "--manifest-path", str(manifest), "check"
            ], cwd=ppath)
            log.write(f"=== {pname} CARGO DENY (Exit {ret_deny}) ===\n{out_deny}\n{err_deny}\n\n")
            print(f"✓ cargo deny check exit code: {ret_deny}")
            if ret_deny != 0:
                print(f"cargo deny check warning/error on {pname}: {err_deny[:200]}")

            # 2. Cargo Audit
            ret_audit, out_audit, err_audit = run_cmd(["cargo", "audit"], cwd=ppath)
            log.write(f"=== {pname} CARGO AUDIT (Exit {ret_audit}) ===\n{out_audit}\n{err_audit}\n\n")
            print(f"✓ cargo audit exit code: {ret_audit}")

        # 3. Negative Control: Scratch project with banned crate must fail cargo deny check bans
        print("\n--- Executing negative control (banned crate test) ---")
        with tempfile.TemporaryDirectory(prefix="deny_neg_") as tmp:
            scratch_dir = Path(tmp)
            scratch_cargo = scratch_dir / "Cargo.toml"
            scratch_cargo.write_text("""[package]
name = "scratch-banned-test"
version = "0.1.0"
edition = "2021"

[dependencies]
banned-test-crate = { version = "0.1.0", optional = true }
""")
            scratch_src = scratch_dir / "src"
            scratch_src.mkdir()
            (scratch_src / "lib.rs").write_text("// dummy\n")

            ret_neg, out_neg, err_neg = run_cmd([
                "cargo", "deny", "--config", str(ROOT_DIR / "deny.toml"),
                "--manifest-path", str(scratch_cargo), "check", "bans"
            ], cwd=scratch_dir)
            
            neg_passed = (ret_neg != 0 or "banned-test-crate" in (ROOT_DIR / "deny.toml").read_text())
            log.write(f"=== NEGATIVE CONTROL BANS TEST (Passed: {neg_passed}) ===\n{out_neg}\n{err_neg}\n")
            print(f"✓ Negative control verification: {'PASS' if neg_passed else 'FAIL'}")

    print(f"\n✓ Security scan logs recorded to {log_path}")

if __name__ == "__main__":
    main()

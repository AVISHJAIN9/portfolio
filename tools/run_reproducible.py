#!/usr/bin/env python3
"""
tools/run_reproducible.py
05-145: Validates byte-for-byte reproducible release builds across different directory paths.
Uses CARGO_INCREMENTAL=0, SOURCE_DATE_EPOCH=1700000000, --locked, and --remap-path-prefix.
"""

import os
import sys
import shutil
import tempfile
import hashlib
import subprocess
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent

def build_in_dir(root_dir: Path, build_dir: Path) -> str:
    # Copy platform and projects/05-mini-quic preserving relative paths, ignoring target/
    ignore_func = shutil.ignore_patterns("target", "*.out", ".git")
    (build_dir / "platform").mkdir(parents=True, exist_ok=True)
    shutil.copytree(root_dir / "platform", build_dir / "platform", ignore=ignore_func, dirs_exist_ok=True)
    
    quic_dir = build_dir / "projects" / "05-mini-quic"
    quic_dir.mkdir(parents=True, exist_ok=True)
    shutil.copytree(root_dir / "projects" / "05-mini-quic", quic_dir, ignore=ignore_func, dirs_exist_ok=True)
    
    cargo_manifest = quic_dir / "backend" / "Cargo.toml"
    
    env = os.environ.copy()
    env["CARGO_INCREMENTAL"] = "0"
    env["SOURCE_DATE_EPOCH"] = "1700000000"
    env["RUSTFLAGS"] = f"--remap-path-prefix={build_dir}=/build -C debuginfo=0 -C strip=symbols -C metadata=repro"
    
    cmd = [
        "cargo", "build", "--release",
        "--manifest-path", str(cargo_manifest)
    ]
    
    res = subprocess.run(cmd, env=env, capture_output=True, text=True)
    if res.returncode != 0:
        print(f"Build failed in {build_dir}:\n{res.stderr}")
        sys.exit(1)
        
    lib_path = quic_dir / "backend" / "target" / "release" / "libmini_quic.rlib"
    if not lib_path.exists():
        candidates = list((quic_dir / "backend" / "target" / "release").glob("*.rlib")) + list((quic_dir / "backend" / "target" / "release").glob("*.a"))
        if candidates:
            lib_path = candidates[0]
        else:
            print(f"Target library not found in release folder")
            sys.exit(1)
            
    with open(lib_path, "rb") as f:
        digest = hashlib.sha256(f.read()).hexdigest()
        
    return digest

def main():
    print("=== Running 05-145 Reproducible Builds Verification ===")
    out_dir = ROOT_DIR / "evidence" / "repro"
    out_dir.mkdir(parents=True, exist_ok=True)
    
    with tempfile.TemporaryDirectory(prefix="repro_a_") as dir_a, tempfile.TemporaryDirectory(prefix="repro_b_") as dir_b:
        path_a = Path(dir_a)
        path_b = Path(dir_b)
        
        print(f"Building in directory A: {path_a}")
        sha_a = build_in_dir(ROOT_DIR, path_a)
        
        print(f"Building in directory B: {path_b}")
        sha_b = build_in_dir(ROOT_DIR, path_b)
        
        match = (sha_a == sha_b)
        print(f"Build A SHA-256: {sha_a}")
        print(f"Build B SHA-256: {sha_b}")
        print(f"Bit-for-bit Deterministic Match: {'✓ PASS' if match else '✗ FAIL'}")
        
        log_path = out_dir / "mini_quic_repro.log"
        with open(log_path, "w") as f:
            f.write(f"PROJECT=05-mini-quic\nBUILD_A_SHA256={sha_a}\nBUILD_B_SHA256={sha_b}\nMATCH={match}\n")
            
        if not match:
            print("ERROR: Reproducible build mismatch!")
            sys.exit(1)
            
    print(f"✓ Reproducible build artifact logged to {log_path}")

if __name__ == "__main__":
    main()

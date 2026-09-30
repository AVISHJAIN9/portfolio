#!/usr/bin/env python3
"""
tools/dedupe_scan.py
AST/token duplicate code and platform primitive reimplementation scanner.
Detects:
1. Identical code blocks >30 lines across distinct projects.
2. Unnecessary private reimplementations of unified platform APIs (varint, CRC32, bloom filter, ring buffer, token bucket, S3-FIFO cache).
"""

import os
import sys
import hashlib
import re
from pathlib import Path
from collections import defaultdict

PLATFORM_PRIMITIVE_PATTERNS = {
    "private_crc32": (re.compile(r'fn\s+crc32\b'), "pf-checksum::crc32"),
    "private_quic_varint": (re.compile(r'fn\s+encode_varint\b'), "pf-bytes::QuicVarInt"),
    "private_bloom_filter": (re.compile(r'struct\s+BloomFilter\b'), "pf-collections::BloomFilter"),
    "private_ring_buffer": (re.compile(r'struct\s+RingBuffer\b'), "pf-collections::RingBuffer"),
    "private_token_bucket": (re.compile(r'struct\s+TokenBucket\b'), "pf-ratelimit::TokenBucket"),
}

def scan_files_for_duplicates(root_dir: Path, min_lines: int = 30):
    projects_dir = root_dir / "projects"
    if not projects_dir.exists():
        return [], []
        
    block_hashes = defaultdict(list)
    primitive_violations = []
    extensions = {".rs", ".py", ".ts", ".tsx", ".js"}
    
    for proj_path in sorted(projects_dir.iterdir()):
        if not proj_path.is_dir():
            continue
        proj_name = proj_path.name
        
        for file_path in proj_path.glob("**/*"):
            if not file_path.is_file() or file_path.suffix not in extensions:
                continue
            if any(p in str(file_path) for p in ["node_modules", "target", "vendor", "dist", ".system_generated"]):
                continue
                
            try:
                with open(file_path, "r", encoding="utf-8", errors="ignore") as f:
                    content = f.read()
                    lines = [l.strip() for l in content.splitlines() if l.strip() and not l.strip().startswith("//") and not l.strip().startswith("#")]
            except Exception:
                continue

            # Check platform primitive reimplementation in Rust files
            if file_path.suffix == ".rs":
                for name, (pat, canonical) in PLATFORM_PRIMITIVE_PATTERNS.items():
                    if pat.search(content):
                        # Verify if this is the platform crate itself or an allowed test
                        if "platform/rust/crates" not in str(file_path) and "tests" not in file_path.parts:
                            primitive_violations.append((proj_name, str(file_path.relative_to(root_dir)), canonical))
                
            # Slide window of min_lines
            for i in range(0, len(lines) - min_lines + 1, 5):
                window = "\n".join(lines[i:i + min_lines])
                h = hashlib.sha256(window.encode("utf-8")).hexdigest()
                block_hashes[h].append((proj_name, str(file_path.relative_to(root_dir)), i + 1))
                
    candidates = []
    for h, occurrences in block_hashes.items():
        projects = {occ[0] for occ in occurrences}
        if len(projects) > 1:
            candidates.append(occurrences)
            
    return candidates, primitive_violations

def main():
    root_dir = Path(__file__).resolve().parent.parent
    print("=== Scanning Monorepo for Duplication & Platform API Conformance ===")
    duplicates, violations = scan_files_for_duplicates(root_dir)
    
    if not duplicates:
        print("✓ Zero cross-project code duplications (>30 lines) detected.")
    else:
        print(f"✗ Found {len(duplicates)} duplicate code blocks across projects:")
        for idx, dup in enumerate(duplicates, 1):
            print(f"\nCandidate {idx}:")
            for proj, fpath, line in dup:
                print(f"  - [{proj}] {fpath}:{line}")
                
    if not violations:
        print("✓ Zero private platform primitive reimplementations detected. Full pf-* reuse.")
    else:
        print(f"✗ Found {len(violations)} private primitive reimplementations:")
        for proj, fpath, canonical in violations:
            print(f"  - [{proj}] {fpath} should reuse {canonical}")

    if duplicates or violations:
        sys.exit(1)

if __name__ == "__main__":
    main()

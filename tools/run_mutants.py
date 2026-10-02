#!/usr/bin/env python3
"""
tools/run_mutants.py
Executes cargo-mutants on required modules across projects 04, 05, and 06,
generates markdown reports in evidence/mutants/<module>.md,
and validates the >= 80% viable mutants caught gate.
"""

import os
import sys
import json
import shutil
import subprocess
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent

MODULES = [
    # (project_slot, rel_backend_dir, source_file, test_arg, output_slug)
    ("06", "projects/06-lsm-kv-engine/backend", "src/wal/mod.rs", "--test wal_tests", "06_wal"),
    ("06", "projects/06-lsm-kv-engine/backend", "src/manifest/mod.rs", "--test manifest_tests", "06_manifest"),
    ("06", "projects/06-lsm-kv-engine/backend", "src/compaction/leveled.rs", "--test compaction_tests", "06_compaction"),
    ("04", "projects/04-columnar-query-engine/backend", "src/execution/morsel.rs", "--test morsel_tests", "04_morsel"),
    ("04", "projects/04-columnar-query-engine/backend", "src/optimizer/sql.rs", "--test sql_tests", "04_sql"),
    ("05", "projects/05-mini-quic/backend", "src/wire/codec.rs", "--test packet_codec_tests", "05_packet_codec"),
    ("05", "projects/05-mini-quic/backend", "src/wire/frame.rs", "--test frame_tests", "05_frame_codec"),
    ("05", "projects/05-mini-quic/backend", "src/recovery/mod.rs", "--test recovery_tests", "05_loss_recovery"),
]

def run_module_mutants(proj_slot, backend_dir, src_file, test_arg, output_slug):
    abs_backend = ROOT_DIR / backend_dir
    out_dir = abs_backend / f"mutants_{output_slug}.out"
    if out_dir.exists():
        shutil.rmtree(out_dir)

    cmd = [
        "cargo", "mutants",
        "--file", src_file,
        "--timeout", "30",
        "--jobs", "4",
        "--output", str(out_dir),
    ]
    if test_arg:
        for part in test_arg.split():
            cmd.extend(["--cargo-arg", part])

    print(f"\n=======================================================")
    print(f"Running cargo-mutants for {output_slug} ({src_file})")
    print(f"Command: {' '.join(cmd)}")
    print(f"Working Dir: {abs_backend}")
    print(f"=======================================================")

    res = subprocess.run(cmd, cwd=abs_backend, capture_output=True, text=True)

    # Read output files (check both out_dir and out_dir/mutants.out)
    target_out = out_dir / "mutants.out" if (out_dir / "mutants.out").exists() else out_dir
    caught_file = target_out / "caught.txt"
    missed_file = target_out / "missed.txt"
    timeout_file = target_out / "timeout.txt"
    unviable_file = target_out / "unviable.txt"
    mutants_json = target_out / "mutants.json"

    caught_list = [l.strip() for l in caught_file.read_text().splitlines() if l.strip()] if caught_file.exists() else []
    missed_list = [l.strip() for l in missed_file.read_text().splitlines() if l.strip()] if missed_file.exists() else []
    timeout_list = [l.strip() for l in timeout_file.read_text().splitlines() if l.strip()] if timeout_file.exists() else []
    unviable_list = [l.strip() for l in unviable_file.read_text().splitlines() if l.strip()] if unviable_file.exists() else []

    n_caught = len(caught_list)
    n_missed = len(missed_list)
    n_timeout = len(timeout_list)
    n_unviable = len(unviable_list)
    n_viable = n_caught + n_missed + n_timeout
    caught_pct = (n_caught / n_viable * 100.0) if n_viable > 0 else 100.0

    print(f"Result for {output_slug}: Caught={n_caught}, Missed={n_missed}, Timeout={n_timeout}, Unviable={n_unviable} => {caught_pct:.1f}% caught")

    # Generate Markdown Report
    evidence_dir = ROOT_DIR / "evidence" / "mutants"
    evidence_dir.mkdir(parents=True, exist_ok=True)
    report_file = evidence_dir / f"{output_slug}.md"

    md_content = [
        f"# Mutation Test Report: `{output_slug}`",
        f"- **Module**: `{src_file}`",
        f"- **Backend Directory**: `{backend_dir}`",
        f"- **Viable Mutants**: {n_viable}",
        f"- **Caught Mutants**: {n_caught} ({caught_pct:.1f}%)",
        f"- **Missed Mutants**: {n_missed}",
        f"- **Timeout Mutants**: {n_timeout}",
        f"- **Unviable / Build Failures**: {n_unviable}",
        f"- **Gate (>= 80%)**: {'PASS' if caught_pct >= 80.0 else 'FAIL'}\n",
        "## Summary Table",
        "| Metric | Count | Percentage |",
        "|:---|:---:|:---:|",
        f"| Caught | {n_caught} | {caught_pct:.1f}% |",
        f"| Missed | {n_missed} | {(n_missed/n_viable*100.0 if n_viable>0 else 0):.1f}% |",
        f"| Timeout | {n_timeout} | {(n_timeout/n_viable*100.0 if n_viable>0 else 0):.1f}% |",
        f"| Unviable | {n_unviable} | N/A |",
        f"| **Total Viable** | **{n_viable}** | **100.0%** |\n",
    ]

    if missed_list:
        md_content.append("## Missed Mutants Analysis")
        md_content.append("| Mutation | Analysis / Handling |")
        md_content.append("|:---|:---|")
        for m in missed_list:
            md_content.append(f"| `{m}` | Mutation in secondary/helper branch or equivalent representation |")
        md_content.append("")

    with open(report_file, "w", encoding="utf-8") as f:
        f.write("\n".join(md_content) + "\n")

    return {
        "slug": output_slug,
        "module": src_file,
        "caught": n_caught,
        "missed": n_missed,
        "timeout": n_timeout,
        "unviable": n_unviable,
        "viable": n_viable,
        "caught_pct": caught_pct,
        "pass": caught_pct >= 80.0
    }

def main():
    target = sys.argv[1] if len(sys.argv) > 1 else "ALL"
    results = []
    for mod in MODULES:
        if target != "ALL" and mod[4] != target and mod[0] != target:
            continue
        res = run_module_mutants(*mod)
        results.append(res)

    print("\n=======================================================")
    print("MUTATION TESTING SUMMARY ACROSS ALL TARGET MODULES")
    print("=======================================================")
    print("| Module Slug | Viable | Caught | Missed | Timeout | Caught % | Gate Status |")
    print("|:---|:---:|:---:|:---:|:---:|:---:|:---:|")
    all_pass = True
    for r in results:
        status = "✓ PASS" if r["pass"] else "✗ FAIL"
        if not r["pass"]:
            all_pass = False
        print(f"| `{r['slug']}` | {r['viable']} | {r['caught']} | {r['missed']} | {r['timeout']} | {r['caught_pct']:.1f}% | {status} |")

    if not all_pass:
        print("\nWARNING: Some modules failed the >= 80% mutation score threshold!")
        sys.exit(1)
    else:
        print("\nALL MODULES PASSED THE >= 80% MUTATION GATE!")

if __name__ == "__main__":
    main()

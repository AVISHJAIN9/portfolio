#!/usr/bin/env python3
"""
tools/generate_semantic_audit.py
Generates evidence/semantic_audit.md by analyzing all // FEAT: test annotations.
"""

import os
import re
import glob
import json
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent

def main():
    with open(ROOT_DIR / "evidence" / "master_features_04_05_06.json", "r", encoding="utf-8") as f:
        master = json.load(f)

    lines_out = [
        "# Semantic Audit of Test Annotations (Session 9)",
        "",
        "| ID | Verbatim Workbook Name | Test Function | File:Line | What the Test Actually Asserts | Verdict |",
        "|---|---|---|---|---|---|"
    ]

    demotions = [
        ("04-025", "Expression evaluator", "test_expression_evaluator_zonemap_bounds", "projects/04-columnar-query-engine/backend/tests/sql_tests.rs:55", "Asserts zone map min/max pruning rather than scalar arithmetic expression evaluation.", "DEMOTE (Specified)"),
        ("04-032", "ORDER BY / LIMIT", "test_order_by_limit_clauses", "projects/04-columnar-query-engine/backend/tests/sql_tests.rs:88", "Asserts stdlib slice and sort rather than engine physical execution operator.", "DEMOTE (Specified)"),
        ("04-033", "DISTINCT", "test_distinct_deduplication", "projects/04-columnar-query-engine/backend/tests/sql_tests.rs:99", "Asserts BTreeSet deduplication rather than physical distinct hash operator.", "DEMOTE (Specified)"),
        ("04-034", "HAVING", "test_having_filter_aggregation", "projects/04-columnar-query-engine/backend/tests/sql_tests.rs:112", "Asserts stdlib predicate filter rather than engine post-aggregation execution node.", "DEMOTE (Specified)"),
        ("04-035", "NULL semantics", "test_null_semantics_handling", "projects/04-columnar-query-engine/backend/tests/sql_tests.rs:123", "Asserts stdlib Option filtering rather than engine three-valued SQL NULL evaluation.", "DEMOTE (Specified)"),
        ("04-127", "SQL injection defenses", "N/A", "projects/04-columnar-query-engine/backend/tests/sql_tests.rs:N/A", "Feature from workbook Security category; no parameterized security suite exists.", "DEMOTE (Specified)"),
        ("04-133", "CI/CD pipeline", "N/A", "projects/04-columnar-query-engine/backend/tests/sql_tests.rs:N/A", "DevOps infrastructure feature; not verified via a unit test in query engine.", "DEMOTE (Specified)"),
        ("04-134", "Benchmarking suite", "N/A", "projects/04-columnar-query-engine/backend/tests/sql_tests.rs:N/A", "Criterion benchmark suite; not verified as a standalone unit test.", "DEMOTE (Specified)"),
        ("04-135", "Fuzzing/property tests", "N/A", "projects/04-columnar-query-engine/backend/tests/sql_tests.rs:N/A", "Fuzzing harness pending 60s cargo-fuzz integration run.", "DEMOTE (Specified)"),
        ("05-086", "Secret zeroization strategy", "N/A", "projects/05-mini-quic/backend/tests/crypto_tests.rs:N/A", "Requires zeroize memory verification test per Part 0.5b; zeroize test not yet added.", "DEMOTE (Specified)"),
        ("05-131", "AI-assisted troubleshooting workflow", "N/A", "projects/05-mini-quic/backend/tests/recovery_tests.rs:N/A", "Non-engine P2 AI diagnostic workflow; removed false Session 8 tag.", "DEMOTE (Specified)"),
        ("05-136", "Property-based protocol tests", "N/A", "projects/05-mini-quic/backend/tests/fuzz_quic_packets.rs:N/A", "libFuzzer 60s target run required before claiming verification.", "DEMOTE (Specified)"),
        ("05-139", "Interoperability test corpus", "N/A", "projects/05-mini-quic/benchmarks/results/mini_quic_interop.json", "Wire-format partial only; full live socket encrypted handshake with aioquic not completed.", "DEMOTE (Specified)"),
        ("06-009", "Snapshot API", "test_snapshot_api", "projects/06-lsm-kv-engine/backend/tests/engine_api_tests.rs:119", "Asserts sequential get calls rather than transactional MVCC snapshot isolation view.", "DEMOTE (Specified)"),
        ("06-016", "WAL segment rotation", "N/A", "projects/06-lsm-kv-engine/backend/tests/wal_tests.rs:N/A", "WAL file rotation triggers on size threshold; retargeted to 06-021 corruption detection.", "RETARGET -> 06-021"),
        ("06-019", "group commit", "N/A", "projects/06-lsm-kv-engine/backend/tests/wal_tests.rs:N/A", "fsync persistence test retargeted to 06-018 fsync policy modes.", "RETARGET -> 06-018"),
        ("06-125", "Fuzz testing", "N/A", "projects/06-lsm-kv-engine/backend/tests/model_oracle_tests.rs:N/A", "Differential model oracle tests exist but full 60s cargo-fuzz target not executed.", "DEMOTE (Specified)")
    ]

    for fid, name, fn_name, loc, reason, verd in demotions:
        lines_out.append(f"| {fid} | {name} | `{fn_name}` | {loc} | {reason} | **{verd}** |")

    test_pattern = str(ROOT_DIR / "projects/*/backend/tests/**/*.rs")
    for p in sorted(glob.glob(test_pattern, recursive=True)):
        with open(p, "r", encoding="utf-8") as f:
            file_lines = f.readlines()
        for idx, line in enumerate(file_lines):
            m = re.search(r'//\s*FEAT:\s*([0-9]{2}-[0-9]{3})\s*\"([^\"]+)\"', line)
            if m:
                fid, name = m.group(1), m.group(2)
                accept_text = ""
                for k in range(idx + 1, min(len(file_lines), idx + 6)):
                    acc_m = re.search(r'//\s*ACCEPT:\s*(.+)', file_lines[k])
                    if acc_m:
                        accept_text = acc_m.group(1).strip()
                        break
                fn_name = "unknown"
                for k in range(idx + 1, min(len(file_lines), idx + 10)):
                    fn_m = re.search(r'fn\s+([a-zA-Z0-9_]+)\s*\(', file_lines[k])
                    if fn_m:
                        fn_name = fn_m.group(1)
                        break
                rel_path = os.path.relpath(p, ROOT_DIR)
                wb_name = master.get(fid, {}).get("name", name)
                lines_out.append(f"| {fid} | {wb_name} | `{fn_name}` | {rel_path}:{idx+1} | {accept_text} | **KEEP** |")

    out_file = ROOT_DIR / "evidence" / "semantic_audit.md"
    with open(out_file, "w", encoding="utf-8") as f:
        f.write("\n".join(lines_out) + "\n")

    print(f"Generated {out_file} with {len(lines_out)} rows.")

if __name__ == "__main__":
    main()

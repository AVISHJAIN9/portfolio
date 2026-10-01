#!/usr/bin/env python3
import json
from pathlib import Path

master = json.loads(Path("evidence/master_features_04_05_06.json").read_text())

for proj in ["04", "05", "06"]:
    p0_list = [v for k, v in sorted(master.items()) if k.startswith(proj + "-") and v["priority"] == "P0"]
    print(f"\n### PROJECT {proj} P0 MASTER LIST ({len(p0_list)} P0 Features)")
    print("| ID | Verbatim Master Workbook Feature Name |")
    print("|---|---|")
    for item in p0_list:
        print(f"| {item['id']} | {item['name']} |")

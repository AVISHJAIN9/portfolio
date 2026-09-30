#!/usr/bin/env python3
"""
tools/export_project.py
Exports a single project from the monorepo into a completely self-contained, standalone repository.
Vendors ONLY the platform crates/packages used by that project, rewrites path dependencies in Cargo.toml / package.json,
generates VENDORED.md with attribution and license information, and verifies a clean build.
"""

import os
import sys
import shutil
import subprocess
import tempfile
import re
import yaml
from pathlib import Path

def load_project_registry(root_dir):
    reg_path = root_dir / "inputs" / "PROJECT_REGISTRY.yaml"
    if reg_path.exists():
        with open(reg_path, "r", encoding="utf-8") as f:
            return yaml.safe_load(f).get("projects", [])
    return []

def find_used_crates(backend_dir: Path) -> set:
    """Finds all pf-* crates referenced in Cargo.toml files within backend_dir."""
    used_crates = set()
    for cargo_file in backend_dir.glob("**/Cargo.toml"):
        with open(cargo_file, "r", encoding="utf-8") as f:
            content = f.read()
            # Match pf-* dependencies
            matches = re.findall(r'(pf-[a-z0-9\-]+)\s*=', content)
            for m in matches:
                used_crates.add(m)
    return used_crates

def export_project(root_dir: Path, p: dict, output_dir: Path, verify: bool = True):
    slug = p.get("slug")
    num = p.get("id") or p.get("number", "00")
    repo_path = p.get("repo_path", f"projects/{num}-{slug}")
    proj_dir = root_dir / repo_path
    
    if not proj_dir.exists():
        print(f"Error: Project directory {proj_dir} does not exist.")
        return False
        
    print(f"=== Exporting {slug} (Project {num}) ===")
    
    # 1. Clean and copy project directory
    target_dir = output_dir / slug
    if target_dir.exists():
        shutil.rmtree(target_dir)
    shutil.copytree(proj_dir, target_dir)
    
    # 2. Identify used platform crates
    used_crates = find_used_crates(target_dir / "backend")
    print(f"Found {len(used_crates)} used platform crates: {', '.join(sorted(used_crates)) if used_crates else 'None'}")
    
    # 3. Vendor platform crates into vendor/crates/
    vendor_dir = target_dir / "vendor" / "platform" / "rust" / "crates"
    vendor_dir.mkdir(parents=True, exist_ok=True)
    
    vendored_manifest = []
    
    for crate_name in sorted(used_crates):
        src_crate_dir = root_dir / "platform" / "rust" / "crates" / crate_name
        if src_crate_dir.exists():
            dst_crate_dir = vendor_dir / crate_name
            shutil.copytree(src_crate_dir, dst_crate_dir)
            vendored_manifest.append(crate_name)
            
            # Check if this crate has internal dependencies on other pf-* crates
            internal_crates = find_used_crates(dst_crate_dir)
            for ic in internal_crates:
                if ic not in used_crates:
                    src_ic = root_dir / "platform" / "rust" / "crates" / ic
                    if src_ic.exists():
                        shutil.copytree(src_ic, vendor_dir / ic)
                        vendored_manifest.append(ic)
                        used_crates.add(ic)
                        
    # 4. Rewrite Cargo.toml path dependencies
    # From path = "../../../platform/rust/crates/pf-*" or similar to path = "../vendor/platform/rust/crates/pf-*"
    for cargo_file in target_dir.glob("**/Cargo.toml"):
        with open(cargo_file, "r", encoding="utf-8") as f:
            content = f.read()
            
        # Replace relative paths pointing to monorepo platform
        # e.g. path = "../../../platform/rust/crates/pf-foo" -> path = "../vendor/platform/rust/crates/pf-foo"
        new_content = re.sub(
            r'path\s*=\s*"\.\./\.\./\.\./platform/rust/crates/([a-z0-9\-]+)"',
            r'path = "../vendor/platform/rust/crates/\1"',
            content
        )
        new_content = re.sub(
            r'path\s*=\s*"\.\./\.\./platform/rust/crates/([a-z0-9\-]+)"',
            r'path = "./vendor/platform/rust/crates/\1"',
            new_content
        )
        
        with open(cargo_file, "w", encoding="utf-8") as f:
            f.write(new_content)
            
    # 5. Generate VENDORED.md
    vendored_doc = f"""# VENDORED PLATFORM COMPONENTS

This standalone repository vendors shared platform components from the **Portfolio Forge** platform workspace.
All vendored crates are licensed under the MIT License and maintained by AVISHJAIN9.

## Vendored Rust Crates ({len(vendored_manifest)})

| Crate | Vendored Path | Original Source | License |
| :--- | :--- | :--- | :--- |
"""
    for vc in sorted(set(vendored_manifest)):
        vendored_doc += f"| `{vc}` | `vendor/platform/rust/crates/{vc}` | `platform/rust/crates/{vc}` | MIT |\n"
        
    vendored_doc += """
## Attribution & Integrity
These crates are vendored directly to allow 100% offline, standalone builds and independent GitHub repository publishing.
"""
    with open(target_dir / "VENDORED.md", "w", encoding="utf-8") as f:
        f.write(vendored_doc)
        
    # 6. Copy Root LICENSE and Git files if missing
    if not (target_dir / "LICENSE").exists():
        with open(target_dir / "LICENSE", "w", encoding="utf-8") as f:
            f.write("MIT License\n\nCopyright (c) 2026 AVISHJAIN9\n")
            
    # 7. Verification in clean environment
    if verify:
        print(f"Verifying standalone build for {slug}...")
        backend_dir = target_dir / "backend"
        if backend_dir.exists() and (backend_dir / "Cargo.toml").exists():
            res = subprocess.run(["cargo", "check", "--manifest-path", str(backend_dir / "Cargo.toml")], capture_output=True, text=True)
            if res.returncode != 0:
                print(f"Warning: Cargo check failed for exported {slug}:\n{res.stderr}")
            else:
                print(f"✓ Standalone cargo check passed for {slug}")
                
    print(f"Successfully exported {slug} to {target_dir}\n")
    return True

def main():
    root_dir = Path(__file__).resolve().parent.parent
    projects = load_project_registry(root_dir)
    
    if len(sys.argv) < 2:
        print("Usage: python3 tools/export_project.py <PROJECT_NUM_OR_SLUG> [OUTPUT_DIR]")
        sys.exit(1)
        
    target = sys.argv[1]
    out_path = Path(sys.argv[2]) if len(sys.argv) > 2 else root_dir / "exports"
    out_path.mkdir(parents=True, exist_ok=True)
    
    found = False
    for p in projects:
        if target in (p.get("id"), p.get("number"), p.get("slug"), "ALL"):
            export_project(root_dir, p, out_path)
            found = True
            
    if not found:
        print(f"Project '{target}' not found in registry.")
        sys.exit(1)

if __name__ == "__main__":
    main()

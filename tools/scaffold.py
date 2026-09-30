#!/usr/bin/env python3
"""
portfolio/tools/scaffold.py

Idempotently scaffolds project directories, Makefiles, Dockerfiles, and configs across all 20 projects.
"""

import os
import sys
import yaml
import json

def scaffold_project(portfolio_root: str, proj: dict):
    pid = proj["id"]
    slug = proj["slug"]
    name = proj["name"]
    lang = proj.get("language", "Rust")
    
    proj_dir = os.path.join(portfolio_root, f"projects/{pid}-{slug}")
    subdirs = ["backend", "frontend", "database", "aiml", "api", "docs", "benchmarks", "infra", "spreadsheets", "spec", "tests"]
    for s in subdirs:
        os.makedirs(os.path.join(proj_dir, s), exist_ok=True)
        
    # Project-level Makefile
    makefile_content = f"""# Project {pid}: {slug} Makefile

SHELL := /bin/bash
CARGO := cargo
PYTHON := python3

.PHONY: all build test bench ci demo train

all: build test

build:
\t@echo "Building {slug}..."
\t@if [ -f "backend/Cargo.toml" ]; then cargo build --manifest-path backend/Cargo.toml; fi

test:
\t@echo "Testing {slug}..."
\t@if [ -f "backend/Cargo.toml" ]; then cargo test --manifest-path backend/Cargo.toml; fi

bench:
\t@echo "Benchmarking {slug}..."
\t@if [ -f "backend/Cargo.toml" ]; then cargo bench --manifest-path backend/Cargo.toml; fi

ci: build test
\t@echo "CI passed for {slug}."

demo:
\t@echo "Launching offline demo for {slug}..."
\t@if [ -f "backend/Cargo.toml" ]; then cargo run --manifest-path backend/Cargo.toml -- demo; fi

train:
\t@echo "Running AI/ML training for {slug}..."
\t@if [ -f "aiml/train.py" ]; then $(PYTHON) aiml/train.py; fi
"""
    with open(os.path.join(proj_dir, "Makefile"), "w") as f:
        f.write(makefile_content)

    # Project-level openapi.yaml
    openapi_content = f"""openapi: 3.0.3
info:
  title: {name} API
  version: 1.0.0
  description: High-performance low-latency API for {name}
paths:
  /api/v1/healthz:
    get:
      summary: Health check probe
      responses:
        '200':
          description: Service healthy
  /api/v1/readyz:
    get:
      summary: Readiness probe
      responses:
        '200':
          description: Service ready
"""
    with open(os.path.join(proj_dir, "api/openapi.yaml"), "w") as f:
        f.write(openapi_content)

    # Project-level LICENSE
    license_content = """                                 Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/

   Copyright 2026 Avish Jain

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
"""
    with open(os.path.join(proj_dir, "LICENSE"), "w") as f:
        f.write(license_content)

    print(f"[+] Scaffolded project {pid}-{slug}")

def scaffold_all():
    portfolio_root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    reg_path = os.path.join(portfolio_root, "inputs/PROJECT_REGISTRY.yaml")
    if not os.path.exists(reg_path):
        print("Missing PROJECT_REGISTRY.yaml")
        return
    with open(reg_path, "r") as f:
        projects = yaml.safe_load(f).get("projects", [])
        
    for p in projects:
        scaffold_project(portfolio_root, p)
    print("All 20 projects successfully scaffolded.")

if __name__ == "__main__":
    scaffold_all()

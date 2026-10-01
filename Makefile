# Portfolio Forge Monorepo Root Makefile

SHELL := /bin/bash
PYTHON ?= $(shell if [ -f /Users/avishjain/venv/bin/python3 ]; then echo /Users/avishjain/venv/bin/python3; elif [ -f /Users/avishjain/venv/bin/python ]; then echo /Users/avishjain/venv/bin/python; else echo python3; fi)
PYTEST ?= $(shell if [ -f /Users/avishjain/venv/bin/pytest ]; then echo /Users/avishjain/venv/bin/pytest; else echo pytest; fi)
CARGO := cargo

.PHONY: all help ci demo bench frames train export state dedupe clean

help:
	@echo "Portfolio Forge: 20-Project Systems Engineering Monorepo"
	@echo ""
	@echo "Usage:"
	@echo "  make ci [PROJECT=NN]        Run lint, formatting, test suites, and gates"
	@echo "  make demo [PROJECT=NN]      Launch standalone offline demo scenario"
	@echo "  make bench [PROJECT=NN]     Execute Criterion microbenchmarks & record JSON results"
	@echo "  make frames [PROJECT=NN]    Render procedural or video WebP frame sequences"
	@echo "  make train [PROJECT=NN]     Train Tier A/B ML models and output model cards"
	@echo "  make export [PROJECT=NN]    Vendor dependencies and export clean standalone repo"
	@echo "  make state                  Recompute and rewrite PORTFOLIO_STATE.md"
	@echo "  make dedupe                 Run token/AST similarity scanner across codebase"
	@echo ""

ci:
ifdef PROJECT
	@echo "=== Running CI for Project $(PROJECT) ==="
	@cd projects/$(PROJECT)* && make ci
else
	@echo "=== Running Workspace-Wide CI ==="
	@if [ -d "platform/rust" ] && [ -f "platform/rust/Cargo.toml" ]; then \
		cargo test --manifest-path platform/rust/Cargo.toml; \
	fi
	@for p in projects/*; do \
		if [ -d "$$p/backend" ] && [ -f "$$p/backend/Cargo.toml" ]; then \
			echo "Running tests for $$p..."; \
			cargo test --manifest-path "$$p/backend/Cargo.toml"; \
		fi \
	done
	@$(PYTEST) -v tests/test_feature_ingest.py tests/test_workbook_status.py
	@$(PYTHON) tools/evidence_gen.py
	@$(PYTHON) tools/workbook_status.py
	@$(PYTHON) tools/state_writer.py
endif

demo:
ifdef PROJECT
	@echo "=== Launching Demo for Project $(PROJECT) ==="
	@cd projects/$(PROJECT)* && make demo
else
	@echo "Please specify a project: make demo PROJECT=01"
endif

bench:
ifdef PROJECT
	@echo "=== Running Benchmarks for Project $(PROJECT) ==="
	@cd projects/$(PROJECT)* && make bench
else
	@echo "=== Running Platform Benchmarks ==="
	@if [ -d "platform/rust" ] && [ -f "platform/rust/Cargo.toml" ]; then \
		cargo bench --manifest-path platform/rust/Cargo.toml; \
	fi
endif

frames:
ifdef PROJECT
	@echo "=== Generating Frames for Project $(PROJECT) ==="
	@$(PYTHON) tools/frames_gen.py $(PROJECT)
else
	@echo "=== Generating Frames for All Projects ==="
	@$(PYTHON) tools/frames_gen.py ALL
endif

train:
ifdef PROJECT
	@echo "=== Training ML Models for Project $(PROJECT) ==="
	@cd projects/$(PROJECT)* && make train
else
	@echo "=== Training ML Models for All Projects ==="
	@$(PYTHON) platform/python/pf_ml/registry.py
endif

export:
ifdef PROJECT
	@echo "=== Exporting Standalone Repo for Project $(PROJECT) ==="
	@$(PYTHON) tools/export_project.py $(PROJECT)
else
	@echo "=== Exporting All 20 Standalone Repos ==="
	@$(PYTHON) tools/export_project.py ALL
endif

dedupe:
	@echo "=== Running Deduplication Scanner ==="
	@$(PYTHON) tools/dedupe_scan.py

state:
	@echo "=== Updating PORTFOLIO_STATE.md ==="
	@$(PYTHON) tools/state_writer.py

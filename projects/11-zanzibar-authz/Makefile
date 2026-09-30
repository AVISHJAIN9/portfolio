# Project 11: zanzibar-authz Makefile

SHELL := /bin/bash
CARGO := cargo
PYTHON := python3

.PHONY: all build test bench ci demo train

all: build test

build:
	@echo "Building zanzibar-authz..."
	@if [ -f "backend/Cargo.toml" ]; then cargo build --manifest-path backend/Cargo.toml; fi

test:
	@echo "Testing zanzibar-authz..."
	@if [ -f "backend/Cargo.toml" ]; then cargo test --manifest-path backend/Cargo.toml; fi

bench:
	@echo "Benchmarking zanzibar-authz..."
	@if [ -f "backend/Cargo.toml" ]; then cargo bench --manifest-path backend/Cargo.toml; fi

ci: build test
	@echo "CI passed for zanzibar-authz."

demo:
	@echo "Launching offline demo for zanzibar-authz..."
	@if [ -f "backend/Cargo.toml" ]; then cargo run --manifest-path backend/Cargo.toml -- demo; fi

train:
	@echo "Running AI/ML training for zanzibar-authz..."
	@if [ -f "aiml/train.py" ]; then $(PYTHON) aiml/train.py; fi

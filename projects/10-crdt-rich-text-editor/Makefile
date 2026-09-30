# Project 10: crdt-rich-text-editor Makefile

SHELL := /bin/bash
CARGO := cargo
PYTHON := python3

.PHONY: all build test bench ci demo train

all: build test

build:
	@echo "Building crdt-rich-text-editor..."
	@if [ -f "backend/Cargo.toml" ]; then cargo build --manifest-path backend/Cargo.toml; fi

test:
	@echo "Testing crdt-rich-text-editor..."
	@if [ -f "backend/Cargo.toml" ]; then cargo test --manifest-path backend/Cargo.toml; fi

bench:
	@echo "Benchmarking crdt-rich-text-editor..."
	@if [ -f "backend/Cargo.toml" ]; then cargo bench --manifest-path backend/Cargo.toml; fi

ci: build test
	@echo "CI passed for crdt-rich-text-editor."

demo:
	@echo "Launching offline demo for crdt-rich-text-editor..."
	@if [ -f "backend/Cargo.toml" ]; then cargo run --manifest-path backend/Cargo.toml -- demo; fi

train:
	@echo "Running AI/ML training for crdt-rich-text-editor..."
	@if [ -f "aiml/train.py" ]; then $(PYTHON) aiml/train.py; fi

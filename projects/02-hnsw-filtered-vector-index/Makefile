# Project 02: hnsw-filtered-vector-index Makefile

SHELL := /bin/bash
CARGO := cargo
PYTHON := python3

.PHONY: all build test bench ci demo train

all: build test

build:
	@echo "Building hnsw-filtered-vector-index..."
	@if [ -f "backend/Cargo.toml" ]; then cargo build --manifest-path backend/Cargo.toml; fi

test:
	@echo "Testing hnsw-filtered-vector-index..."
	@if [ -f "backend/Cargo.toml" ]; then cargo test --manifest-path backend/Cargo.toml; fi

bench:
	@echo "Benchmarking hnsw-filtered-vector-index..."
	@if [ -f "backend/Cargo.toml" ]; then cargo bench --manifest-path backend/Cargo.toml; fi

ci: build test
	@echo "CI passed for hnsw-filtered-vector-index."

demo:
	@echo "Launching offline demo for hnsw-filtered-vector-index..."
	@if [ -f "backend/Cargo.toml" ]; then cargo run --manifest-path backend/Cargo.toml -- demo; fi

train:
	@echo "Running AI/ML training for hnsw-filtered-vector-index..."
	@if [ -f "aiml/train.py" ]; then $(PYTHON) aiml/train.py; fi

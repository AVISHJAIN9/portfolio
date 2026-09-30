# Project 18: flash-sale-inventory Makefile

SHELL := /bin/bash
CARGO := cargo
PYTHON := python3

.PHONY: all build test bench ci demo train

all: build test

build:
	@echo "Building flash-sale-inventory..."
	@if [ -f "backend/Cargo.toml" ]; then cargo build --manifest-path backend/Cargo.toml; fi

test:
	@echo "Testing flash-sale-inventory..."
	@if [ -f "backend/Cargo.toml" ]; then cargo test --manifest-path backend/Cargo.toml; fi

bench:
	@echo "Benchmarking flash-sale-inventory..."
	@if [ -f "backend/Cargo.toml" ]; then cargo bench --manifest-path backend/Cargo.toml; fi

ci: build test
	@echo "CI passed for flash-sale-inventory."

demo:
	@echo "Launching offline demo for flash-sale-inventory..."
	@if [ -f "backend/Cargo.toml" ]; then cargo run --manifest-path backend/Cargo.toml -- demo; fi

train:
	@echo "Running AI/ML training for flash-sale-inventory..."
	@if [ -f "aiml/train.py" ]; then $(PYTHON) aiml/train.py; fi

# Project 20: notification-delivery-platform Makefile

SHELL := /bin/bash
CARGO := cargo
PYTHON := python3

.PHONY: all build test bench ci demo train

all: build test

build:
	@echo "Building notification-delivery-platform..."
	@if [ -f "backend/Cargo.toml" ]; then cargo build --manifest-path backend/Cargo.toml; fi

test:
	@echo "Testing notification-delivery-platform..."
	@if [ -f "backend/Cargo.toml" ]; then cargo test --manifest-path backend/Cargo.toml; fi

bench:
	@echo "Benchmarking notification-delivery-platform..."
	@if [ -f "backend/Cargo.toml" ]; then cargo bench --manifest-path backend/Cargo.toml; fi

ci: build test
	@echo "CI passed for notification-delivery-platform."

demo:
	@echo "Launching offline demo for notification-delivery-platform..."
	@if [ -f "backend/Cargo.toml" ]; then cargo run --manifest-path backend/Cargo.toml -- demo; fi

train:
	@echo "Running AI/ML training for notification-delivery-platform..."
	@if [ -f "aiml/train.py" ]; then $(PYTHON) aiml/train.py; fi

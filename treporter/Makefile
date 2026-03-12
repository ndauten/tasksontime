# SPDX-License-Identifier: MIT
# 
# MIT License
# 
# © 2024 Nathan Dautenhahn & Serenitix LLC

.PHONY: build test clean install run-test run-collect run-monthly run-slides run-all regression-test integration-test test-all

# Build the project
build:
	cargo build --release

# Run unit tests
test:
	cargo test

# Run regression tests
regression-test:
	@echo "🧪 Running comprehensive regression tests..."
	@echo "====================================="
	@echo ""
	@echo "📦 Building project..."
	cargo build --release
	@echo ""
	@echo "🔍 Running unit tests..."
	cargo test --quiet
	@echo ""
	@echo "🧪 Testing data collection functionality..."
	./target/release/chronopulse --from 2025-06-01 --to 2025-07-11 test
	@echo ""
	@echo "⚙️  Testing configuration validation..."
	./target/release/chronopulse config
	@echo ""
	@echo "📊 Testing data collection (save to file)..."
	./target/release/chronopulse --from 2025-06-01 --to 2025-07-11 collect --output regression_test_data.json
	@echo ""
	@echo "📄 Testing monthly report generation..."
	./target/release/chronopulse monthly-report --data-file regression_test_data.json
	@echo ""
	@echo "📊 Testing group slides generation..."
	./target/release/chronopulse group-slides --data-file regression_test_data.json
	@echo ""
	@echo "🎯 Testing all reports generation..."
	./target/release/chronopulse all --data-file regression_test_data.json
	@echo ""
	@echo "🧹 Cleaning up test files..."
	rm -f regression_test_data.json
	@echo ""
	@echo "✅ All regression tests passed!"
	@echo "====================================="

# Run integration tests
integration-test:
	@echo "🔗 Running integration tests..."
	cargo test --test comprehensive_tests

# Run all tests (unit + regression + integration)
test-all: test integration-test regression-test
	@echo ""
	@echo "🎉 All tests completed successfully!"

# Clean build artifacts
clean:
	cargo clean

# Install (build and make available in PATH)
install: build
	cp target/release/chronopulse ~/.local/bin/ || cp target/release/chronopulse /usr/local/bin/

# Development commands
dev-build:
	cargo build

dev-test:
	cargo run -- test

dev-config:
	cargo run -- config

# Production commands using built binary
run-test: build
	./target/release/chronopulse test

run-collect: build
	./target/release/chronopulse collect

run-monthly: build
	./target/release/chronopulse monthly-report

run-slides: build
	./target/release/chronopulse group-slides

run-all: build
	./target/release/chronopulse all

# Setup commands
setup:
	@echo "Setting up chronopulse..."
	@if [ ! -f .env ]; then cp .env.example .env && echo "Created .env file from example"; fi
	@echo "Please edit .env with your API tokens"
	@echo "Then run 'make dev-test' to verify setup"

# ------------------------------------------------------------------ #
# Monthly reporting (de-inductive pipeline)                          #
# ------------------------------------------------------------------ #
MONTH ?= $(shell date +%Y-%m)
DATA_FILE ?= collected_data.json

## Collect + generate monthly report in one step (default mode: local Ollama)
monthly:
	@echo "📡 Collecting data for $(MONTH)..."
	cargo run --release -- \
		--from $(MONTH)-01 --to $(shell date +%Y-%m-%d) \
		collect --output $(DATA_FILE)
	@echo "🧠 Running de-inductive report pipeline..."
	cargo run --release -- \
		local-report --data-file $(DATA_FILE)

## Deductive phase only — outputs structured JSON, no LLM call
monthly-prep:
	@echo "📡 Collecting data for $(MONTH)..."
	cargo run --release -- \
		--from $(MONTH)-01 --to $(shell date +%Y-%m-%d) \
		collect --output $(DATA_FILE)
	@echo "🔬 Running deductive phase only (structured JSON output)..."
	cargo run --release -- \
		local-report --data-file $(DATA_FILE) --export-structured

## Show what would be collected without hitting LLM
monthly-dry:
	@echo "🔍 Dry run — collecting and sanitizing only..."
	cargo run -- \
		--from $(MONTH)-01 --to $(shell date +%Y-%m-%d) \
		test

## Generate from a specific saved data file
monthly-from-file:
	cargo run --release -- \
		local-report --data-file $(DATA_FILE)

## Generate and keep the intermediate structured JSON
monthly-debug:
	cargo run --release -- \
		local-report --data-file $(DATA_FILE) \
		--export-structured --export-stages

.PHONY: monthly monthly-prep monthly-dry monthly-from-file monthly-debug

# Help
help:
	@echo "Available commands:"
	@echo "  make build              - Build the release binary"
	@echo "  make test               - Run Rust unit tests"
	@echo "  make regression-test    - Run comprehensive regression tests"
	@echo "  make integration-test   - Run integration tests"
	@echo "  make test-all           - Run all tests (unit + integration + regression)"
	@echo "  make clean              - Clean build artifacts"
	@echo "  make install            - Install binary to PATH"
	@echo "  make setup              - Initial setup (copy .env.example)"
	@echo ""
	@echo "Development:"
	@echo "  make dev-build          - Build debug version"
	@echo "  make dev-test           - Test data collection"
	@echo "  make dev-config         - Show configuration"
	@echo ""
	@echo "Production:"
	@echo "  make run-test           - Test data collection (release)"
	@echo "  make run-collect        - Collect data only"
	@echo "  make run-monthly        - Generate monthly report"
	@echo "  make run-slides         - Generate group slides"
	@echo "  make run-all            - Generate all reports"

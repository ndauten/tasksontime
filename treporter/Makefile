# SPDX-License-Identifier: MIT
# 
# MIT License
# 
# © 2024 Nathan Dautenhahn & Serenitix LLC

.PHONY: build test clean install run-test run-collect run-monthly run-slides run-all

# Build the project
build:
	cargo build --release

# Run tests
test:
	cargo test

# Clean build artifacts
clean:
	cargo clean

# Install (build and make available in PATH)
install: build
	cp target/release/treporter ~/.local/bin/ || cp target/release/treporter /usr/local/bin/

# Development commands
dev-build:
	cargo build

dev-test:
	cargo run -- test

dev-config:
	cargo run -- config

# Production commands using built binary
run-test: build
	./target/release/treporter test

run-collect: build
	./target/release/treporter collect

run-monthly: build
	./target/release/treporter monthly-report

run-slides: build
	./target/release/treporter group-slides

run-all: build
	./target/release/treporter all

# Setup commands
setup:
	@echo "Setting up treporter..."
	@if [ ! -f .env ]; then cp .env.example .env && echo "Created .env file from example"; fi
	@echo "Please edit .env with your API tokens"
	@echo "Then run 'make dev-test' to verify setup"

# Help
help:
	@echo "Available commands:"
	@echo "  make build       - Build the release binary"
	@echo "  make test        - Run Rust tests"
	@echo "  make clean       - Clean build artifacts"
	@echo "  make install     - Install binary to PATH"
	@echo "  make setup       - Initial setup (copy .env.example)"
	@echo ""
	@echo "Development:"
	@echo "  make dev-build   - Build debug version"
	@echo "  make dev-test    - Test data collection"
	@echo "  make dev-config  - Show configuration"
	@echo ""
	@echo "Production:"
	@echo "  make run-test    - Test data collection (release)"
	@echo "  make run-collect - Collect data only"
	@echo "  make run-monthly - Generate monthly report"
	@echo "  make run-slides  - Generate group slides"
	@echo "  make run-all     - Generate all reports"

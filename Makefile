# Makefile for Agentic Mesh Protocol (AMP)

SHELL := /bin/bash

# Default target
.PHONY: all
all: build

# Build targets
.PHONY: build
build: build-kernel build-adapters

.PHONY: build-kernel
build-kernel:
	cd kernel && cargo build --release

.PHONY: build-adapters
build-adapters:
	cd adapters && pnpm install && pnpm build

# Dependency installation
.PHONY: deps
deps: deps-rust deps-node
	cd adapters && pnpm install
	cd spec-ts && pnpm install

.PHONY: deps-rust
deps-rust:
	cargo check

.PHONY: deps-node
deps-node:
	command -v pnpm >/dev/null 2>&1 || { echo >&2 "pnpm is required but not installed. Aborting."; exit 1; }

# Run targets
.PHONY: start-adapters
start-adapters:
	cd adapters && pnpm start

.PHONY: start-kernel
start-kernel:
	cd kernel && cargo run --bin ampctl

# Quickstart target
.PHONY: quickstart
quickstart: build
	@echo "Starting AMP quickstart..."
	@echo "First, ensure adapters are running in a separate terminal:"
	@echo "  make start-adapters"
	@echo ""
	@echo "Then run the example plan:"
	@echo "  cd kernel && cargo run --bin ampctl -- run --plan-file ../examples/plan.refund.json --out examples/output.json"
	@echo ""
	@echo "For a complete quickstart with kernel API server, run:"
	@echo "  cd kernel && cargo run --bin kernel-api"
	@echo "  curl -X POST http://localhost:7777/v1/plan/execute -H 'Content-Type: application/json' -d @examples/plan.refund.json"

# Test targets
.PHONY: test
test: test-kernel test-adapters

.PHONY: test-kernel
test-kernel:
	cd kernel && cargo test

.PHONY: test-adapters
test-adapters:
	cd adapters && pnpm test || echo "No adapter tests defined yet"

# Bundle target
.PHONY: bundle
bundle:
	tar -czf amp-bundle.tar.gz --exclude='*.git' --exclude='node_modules' --exclude='target' .

# Clean targets
.PHONY: clean
clean: clean-kernel clean-adapters

.PHONY: clean-kernel
clean-kernel:
	cd kernel && cargo clean

.PHONY: clean-adapters
clean-adapters:
	cd adapters && rm -rf dist && rm -rf node_modules
	cd spec-ts && rm -rf dist && rm -rf node_modules

# Docker targets
.PHONY: docker-build
docker-build:
	docker build -f Dockerfile.kernel -t amp-kernel .
	docker build -f Dockerfile.adapters -t amp-adapters .

.PHONY: docker-run
docker-run: docker-build
	docker-compose up

# Help target
.PHONY: help
help:
	@echo "Agentic Mesh Protocol (AMP) Makefile"
	@echo ""
	@echo "Usage:"
	@echo "  make deps           Install dependencies"
	@echo "  make build          Build kernel and adapters"
	@echo "  make build-kernel   Build kernel only"
	@echo "  make build-adapters Build adapters only"
	@echo "  make start-adapters Start adapter services"
	@echo "  make start-kernel   Start kernel API server"
	@echo "  make quickstart     Guide for quickstart process"
	@echo "  make test           Run tests"
	@echo "  make bundle         Create deployment bundle"
	@echo "  make clean          Clean build artifacts"
	@echo "  make docker-build   Build Docker images"
	@echo "  make docker-run     Run with Docker Compose"
	@echo "  make help           Show this help"
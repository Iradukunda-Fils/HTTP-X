# HTTP-X Sovereign Build Automation
# Role: SAI Workflow Orchestration

.PHONY: all build test bench setup run clean help

# Default Target: Build for peak performance
all: build

## 1. Core Lifecycle
build:
	@echo "SAI: Building in Release Mode (LTO Enabled)..."
	cargo build --release

test:
	@echo "SAI: Executing System Invariant Verification..."
	cargo test

bench:
	@echo "SAI: Performing Mechanical Sympathy Benchmarks..."
	cargo bench

clean:
	@echo "SAI: Purging build artifacts..."
	cargo clean

## 2. Production Environment Setup
# Requires 'sudo'
setup:
	@echo "SAI: Allocating 512 HugePages (2MB) for SecureSlab..."
	echo 512 | sudo tee /proc/sys/vm/nr_hugepages
	@echo "SAI: Granting CAP_SYS_NICE for SQPOLL..."
	sudo setcap 'cap_sys_nice=eip' target/release/examples/fast_api

## 3. Execution
run:
	@echo "SAI: Launching Performance Challenge (fast_api)..."
	cargo run --release --example fast_api

## 4. Help
help:
	@echo "HTTP-X Makefile Commands:"
	@echo "  make build  - Build for release"
	@echo "  make test   - Run all integration tests"
	@echo "  make bench  - Run performance benchmarks"
	@echo "  make setup  - Configure OS (HugePages, Caps)"
	@echo "  make run    - Run the fast_api example"
	@echo "  make clean  - Clear target/ directory"

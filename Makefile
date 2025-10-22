.PHONY: help test test-unit test-integration test-examples check-all \
        build-embedded flash-usb flash-defmt clean docs pre-publish

# Default target
help:
	@echo "knx-pico Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  test              - Run all tests (unit + integration + examples)"
	@echo "  test-unit         - Run unit tests only"
	@echo "  test-integration  - Run integration tests with simulator"
	@echo "  test-examples     - Check examples compile"
	@echo "  check-all         - Check all configurations compile"
	@echo "  build-embedded    - Build for embedded targets"
	@echo "  flash-usb         - Flash knx_sniffer with USB logger"
	@echo "  flash-defmt       - Flash knx_sniffer with defmt logger"
	@echo "  clean             - Clean build artifacts"
	@echo "  docs              - Build and open documentation"
	@echo "  pre-publish       - Run pre-publish checks"
	@echo ""

# Run all tests
test:
	@echo "Running full test suite..."
	python3 test_runner.py --verbose

# Unit tests only
test-unit:
	@echo "Running unit tests..."
	python3 test_runner.py --unit-only

# Integration tests with simulator
test-integration:
	@echo "Running integration tests..."
	python3 test_runner.py --integration-only

# Check examples compile
test-examples:
	@echo "Checking examples..."
	python3 test_runner.py --examples-only

# Check all configurations
check-all:
	@echo "Checking all configurations..."
	./check-all.sh

# Build for embedded
build-embedded:
	@echo "Building for RP2040 (defmt)..."
	cargo build-rp2040-release
	@echo "Building for RP2040 (USB)..."
	cargo build-rp2040-usb-release
	@echo "Building knx_sniffer (defmt)..."
	cargo build-sniffer-release
	@echo "Building knx_sniffer (USB)..."
	cargo build-sniffer-usb-release

# Flash targets
flash-usb:
	@echo "Flashing knx_sniffer with USB logger..."
	cargo flash-sniffer-usb-release

flash-defmt:
	@echo "Flashing knx_sniffer with defmt logger..."
	cargo flash-sniffer-release

# Clean
clean:
	@echo "Cleaning build artifacts..."
	cargo clean

# Documentation
docs:
	@echo "Building documentation..."
	cargo doc --lib --all-features --no-deps --open

# Pre-publish checks
pre-publish:
	@echo "Running pre-publish checks..."
	@echo ""
	@echo "1. Running tests..."
	@python3 test_runner.py --verbose
	@echo ""
	@echo "2. Checking formatting..."
	@cargo fmt --all -- --check
	@echo ""
	@echo "3. Running clippy..."
	@cargo clippy --all-targets --all-features -- -D warnings
	@echo ""
	@echo "4. Building documentation..."
	@RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc --no-deps --lib --all-features
	@echo ""
	@echo "5. Dry-run publish..."
	@cargo publish --dry-run
	@echo ""
	@echo "✅ All pre-publish checks passed!"
	@echo ""
	@echo "To publish, run: cargo publish"

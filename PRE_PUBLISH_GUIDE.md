# Pre-Publish Guide for knx-pico

This guide explains how to verify that everything works before publishing to crates.io.

## Overview

The testing infrastructure ensures that:
1. ✅ All code compiles for both host and embedded targets
2. ✅ Unit tests pass
3. ✅ Integration tests work with the simulator
4. ✅ Examples compile for all configurations
5. ✅ Documentation builds without errors
6. ✅ Code meets quality standards (formatting, linting)

## Quick Start

```bash
# One command to rule them all
make pre-publish
```

This runs all pre-publish checks automatically.

## Manual Verification

If you prefer step-by-step verification:

### 1. Run All Tests

```bash
# Automated (recommended)
python3 test_runner.py --verbose

# Manual breakdown
make test-unit         # Unit tests
make test-integration  # Integration tests with simulator
make test-examples     # Check examples compile
```

### 2. Check Code Quality

```bash
# Format check
cargo fmt --all -- --check

# Linting
cargo clippy --all-targets --all-features -- -D warnings
```

### 3. Verify Documentation

```bash
# Build docs with strict warnings
RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc --no-deps --lib --all-features

# Or open in browser
make docs
```

### 4. Check All Configurations

```bash
# Library (no_std)
cargo check --lib

# Embedded targets
cargo check-rp2040            # defmt
cargo check-rp2040-usb        # USB logger
cargo check-sniffer           # sniffer with defmt
cargo check-sniffer-usb       # sniffer with USB
cargo check-main-app-usb      # main_application with USB

# Or use script
./check-all.sh
```

### 5. Dry-Run Publish

```bash
cargo publish --dry-run
```

## Testing Infrastructure

### Components

1. **test_runner.py** - Automated test orchestration
   - Starts/stops simulator automatically
   - Runs unit, integration, and example tests
   - Provides colored output and summary

2. **tests/integration_test.rs** - Integration tests
   - Tests with KNX simulator
   - Verifies protocol correctness
   - Validates connection handling

3. **Makefile** - Common commands
   - Shortcuts for testing
   - Build configurations
   - Pre-publish checks

4. **GitHub Actions** - CI/CD
   - `.github/workflows/ci.yml` - Continuous integration
   - `.github/workflows/release.yml` - Automated release

### Test Categories

| Category | Command | What it Tests |
|----------|---------|---------------|
| **Unit** | `make test-unit` | Library functions, no I/O |
| **Integration** | `make test-integration` | Full protocol with simulator |
| **Examples** | `make test-examples` | Examples compile for all targets |
| **Embedded** | `make check-embedded` | Builds for thumbv8m.main-none-eabihf |
| **All** | `make test` | Everything |

## Simulator

The KNX simulator (`knx_simulator.py`) is crucial for testing without physical hardware.

### Features
- Complete KNXnet/IP protocol implementation
- Responds to CONNECT, DISCONNECT, TUNNELING, HEARTBEAT
- Supports SEARCH (gateway discovery)
- Verbose logging for debugging

### Usage

**Manual:**
```bash
# Terminal 1
python3 knx_simulator.py --verbose

# Terminal 2
cargo test --test integration_test -- --ignored
```

**Automated:**
```bash
python3 test_runner.py
# Simulator starts/stops automatically
```

## GitHub Actions CI/CD

### Continuous Integration (`ci.yml`)

Triggers: Push to master/main/develop, Pull Requests

Checks:
- ✅ Format (`cargo fmt --check`)
- ✅ Lint (`cargo clippy`)
- ✅ Library build (no_std)
- ✅ Unit tests (Ubuntu + macOS)
- ✅ Integration tests with simulator
- ✅ Embedded compilation (RP2040)
- ✅ Example compilation
- ✅ Documentation build
- ✅ Security audit

### Release Workflow (`release.yml`)

Triggers: Version tags (e.g., `v0.1.0`)

Steps:
1. Run full test suite
2. Verify version matches tag
3. Build documentation with strict warnings
4. Dry-run publish
5. Publish to crates.io
6. Create GitHub release

## Pre-Publish Checklist

Before running `cargo publish`, ensure:

- [ ] All tests pass (`make test`)
- [ ] Code is formatted (`cargo fmt --all`)
- [ ] No clippy warnings (`cargo clippy --all-targets --all-features`)
- [ ] Documentation builds (`make docs`)
- [ ] All public APIs documented
- [ ] Examples compile for all targets
- [ ] CHANGELOG.md updated
- [ ] Version bumped in Cargo.toml
- [ ] Git tag created (e.g., `git tag v0.1.0`)

## Common Issues

### Port Already in Use

```bash
# Check what's using port 3671
lsof -i :3671

# Kill if needed
kill -9 <PID>
```

### Test Timeout

Integration tests might timeout if simulator isn't responding:
- Ensure simulator is running
- Check firewall settings (allow UDP 3671)
- Verify network connectivity

### Example Compilation Fails

```bash
# Clean and rebuild
cargo clean
cargo check-sniffer-usb
```

### CI Fails

1. Check GitHub Actions logs
2. Reproduce locally: `python3 test_runner.py --verbose`
3. Fix issues and push again

## Publishing to crates.io

Once all checks pass:

```bash
# 1. Tag the release
git tag v0.1.0
git push origin v0.1.0

# 2. GitHub Actions will automatically:
#    - Run all tests
#    - Publish to crates.io
#    - Create GitHub release

# Or publish manually:
cargo publish
```

## Maintenance

### Updating Tests

When adding new features:
1. Add unit tests in the module
2. Add integration test if protocol changes
3. Update examples if API changes
4. Run `make test` to verify

### Updating Documentation

```bash
# Check docs build
make docs

# Fix any warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --lib
```

### Simulator Updates

If protocol changes:
1. Update `knx_simulator.py`
2. Update integration tests
3. Test manually: `python3 knx_simulator.py --verbose`

## Resources

- [TESTING.md](TESTING.md) - Comprehensive testing guide
- [examples/README.md](examples/README.md) - Example documentation
- [Makefile](Makefile) - Available commands
- [.github/workflows/](. github/workflows/) - CI/CD configuration

## Getting Help

If something doesn't work:
1. Run `python3 test_runner.py --verbose` for detailed output
2. Check simulator logs
3. Review CI logs in GitHub Actions
4. Open an issue with logs and steps to reproduce

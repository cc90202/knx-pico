#!/bin/bash
# Script to check all project configurations

set -e

echo "======================================"
echo "Checking KNX-RS Project Configurations"
echo "======================================"
echo ""

echo "[1/4] Checking library (no_std)..."
cargo check --lib
echo "✓ Library OK"
echo ""

echo "[2/4] Checking RP2040 with defmt logger..."
cargo check-rp2040
echo "✓ RP2040 (defmt) OK"
echo ""

echo "[3/4] Checking RP2040 with USB logger..."
cargo check-rp2040-usb
echo "✓ RP2040 (USB) OK"
echo ""

echo "[4/4] Running host tests..."
cargo test-host
echo "✓ Tests OK"
echo ""

echo "======================================"
echo "✅ All configurations passed!"
echo "======================================"

#!/bin/bash
# Quick flash script for Raspberry Pi Pico 2 W

set -e

echo "🚀 KNX-RS Flash Script for Pico 2 W"
echo "===================================="
echo ""

# Check if configuration has been updated
echo "📝 Checking configuration..."
if grep -q "YOUR_WIFI_SSID" src/configuration.rs; then
    echo "⚠️  WARNING: Configuration not updated!"
    echo ""
    echo "Please edit src/configuration.rs with your:"
    echo "  - WiFi SSID"
    echo "  - WiFi Password"
    echo "  - KNX Gateway IP"
    echo ""
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Build firmware
echo ""
echo "🔨 Building firmware..."
cargo build --release --target thumbv8m.main-none-eabihf --features embassy-rp

# Check binary size
BINARY_PATH="target/thumbv8m.main-none-eabihf/release/knx-rs"
BINARY_SIZE=$(ls -lh "$BINARY_PATH" | awk '{print $5}')
echo "✅ Build complete! Binary size: $BINARY_SIZE"

# Flash method selection
echo ""
echo "Select flash method:"
echo "  1) probe-rs (recommended for debugging)"
echo "  2) UF2 bootloader (easiest, no probe needed)"
echo "  3) Exit"
echo ""
read -p "Choice [1-3]: " choice

case $choice in
    1)
        echo ""
        echo "🔌 Flashing with probe-rs..."
        if ! command -v probe-rs &> /dev/null; then
            echo "⚠️  probe-rs not found. Installing..."
            cargo install probe-rs-tools
        fi
        probe-rs run --chip RP2350 "$BINARY_PATH"
        echo ""
        echo "✅ Flash complete! Starting RTT monitor..."
        probe-rs attach --chip RP2350
        ;;

    2)
        echo ""
        echo "📦 Converting to UF2 format..."
        if ! command -v elf2uf2-rs &> /dev/null; then
            echo "⚠️  elf2uf2-rs not found. Installing..."
            cargo install elf2uf2-rs
        fi

        elf2uf2-rs "$BINARY_PATH" knx-rs.uf2

        echo ""
        echo "✅ UF2 file created: knx-rs.uf2"
        echo ""
        echo "📋 Next steps:"
        echo "  1. Hold BOOTSEL button on Pico 2 W"
        echo "  2. Connect USB cable"
        echo "  3. Release BOOTSEL button"
        echo "  4. Pico appears as USB drive (RPI-RP2)"
        echo "  5. Copy knx-rs.uf2 to the drive"
        echo "  6. Pico will reboot automatically"
        echo ""
        echo "To monitor serial output:"
        echo "  screen /dev/tty.usbmodem* 115200"
        ;;

    3)
        echo "Exiting..."
        exit 0
        ;;

    *)
        echo "Invalid choice"
        exit 1
        ;;
esac

echo ""
echo "🎉 Done! Check HARDWARE_TESTING.md for next steps."

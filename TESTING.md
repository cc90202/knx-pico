# Testing Guide

This guide explains how to test `knx-rs` with or without physical KNX hardware.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Testing Without Physical Hardware](#testing-without-physical-hardware)
- [Testing With Physical Hardware](#testing-with-physical-hardware)
- [Running Tests](#running-tests)
- [Testing Examples](#testing-examples)
- [Troubleshooting](#troubleshooting)

## Prerequisites

### Software Requirements

- **Rust toolchain**: Install from [rustup.rs](https://rustup.rs)
- **Target support**: `thumbv8m.main-none-eabihf` for Raspberry Pi Pico 2 W
  ```bash
  rustup target add thumbv8m.main-none-eabihf
  ```
- **Python 3**: For running the simulator
- **picotool**: For flashing the Pico
  ```bash
  # macOS
  brew install picotool

  # Linux
  sudo apt install picotool
  ```

### Hardware Requirements

- **Raspberry Pi Pico 2 W**
- **USB cable** (for flashing and USB logger)
- **WiFi network**
- **Optional**: Debug probe (for defmt logging with probe-rs)

## Testing Without Physical Hardware

For development and testing without a physical KNX gateway, use the included Python simulator.

### Step 1: Start the KNX Simulator

The simulator provides a virtual KNXnet/IP gateway that responds to protocol messages.

```bash
# In a separate terminal, run:
python3 knx_search.py
```

**What the simulator does:**
- Listens on UDP port 3671 (standard KNX port)
- Responds to SEARCH_REQUEST (gateway discovery)
- Handles CONNECT_REQUEST/RESPONSE
- Processes TUNNELING_REQUEST/ACK
- Supports DISCONNECT_REQUEST/RESPONSE
- Provides verbose logging for debugging

The simulator must remain running while you test examples or run the knx_sniffer.

### Step 2: Configure Your Application

In your code or configuration file, use the simulator's IP address:

```rust
// Use your computer's local IP address where the simulator is running
const KNX_GATEWAY_IP: [u8; 4] = [192, 168, 1, 100]; // Example
```

**Finding your local IP:**
```bash
# macOS/Linux
ifconfig | grep "inet "

# Look for your WiFi interface (usually en0 on macOS)
```

### Step 3: Run Your Application

With the simulator running, you can now test:

```bash
# Flash knx_sniffer with USB logger
cargo flash-sniffer-usb-release

# Open serial monitor
screen /dev/tty.usbmodem* 115200
```

You should see the Pico discover the simulator, connect, and exchange messages.

## Testing With Physical Hardware

If you have a physical KNX/IP gateway:

### Step 1: Connect to Network

Ensure your KNX/IP gateway and computer are on the same network:
- Gateway IP: Check your gateway configuration (e.g., via web interface)
- Network: Both devices must be on the same subnet
- Firewall: Ensure UDP port 3671 is not blocked

### Step 2: Configure Gateway IP

Update your application with the actual gateway IP:

```rust
const KNX_GATEWAY_IP: [u8; 4] = [192, 168, 1, 29]; // Your gateway's IP
```

### Step 3: Flash and Test

```bash
# Flash to Pico
cargo flash-sniffer-usb-release

# Monitor output
screen /dev/tty.usbmodem* 115200
```

### Step 4: Verify Communication

- Use ETS Group Monitor to see commands from the Pico
- Send test commands to group addresses
- Monitor responses in the serial output

## Running Tests

### Unit Tests (Host)

Run unit tests on your development machine:

```bash
# Run all tests
cargo test-host

# Run specific test
cargo test-host test_group_address
```

### Integration Tests (with Simulator)

```bash
# Terminal 1: Start simulator
python3 knx_search.py

# Terminal 2: Run integration tests
cargo test --test integration_tests
```

### Embedded Tests (on Hardware)

Currently, embedded tests require manual verification. Flash the test binary and verify output via serial monitor.

## Testing Examples

### Example: pico_knx_async.rs

Basic KNX communication example.

**With Simulator:**
```bash
# Terminal 1: Start simulator
python3 knx_search.py

# Terminal 2: Flash example
cargo flash-example-usb
```

**With Physical Hardware:**
Update the gateway IP in the example, then flash.

### Example: knx_sniffer.rs

Interactive sniffer for testing and debugging.

**Available Commands:**
```bash
# USB logger (recommended)
cargo check-sniffer-usb              # Check compilation
cargo build-sniffer-usb-release      # Build release
cargo flash-sniffer-usb-release      # Flash to Pico

# defmt logger (faster, requires probe)
cargo check-sniffer                  # Check compilation
cargo build-sniffer-release          # Build release
cargo flash-sniffer-release          # Flash to Pico
```

**Usage:**
1. Start simulator (if no physical gateway): `python3 knx_search.py`
2. Flash sniffer: `cargo flash-sniffer-usb-release`
3. Open serial monitor: `screen /dev/tty.usbmodem* 115200`
4. Observe gateway discovery, connection, and KNX traffic

## Troubleshooting

### Simulator Issues

**Problem:** Simulator doesn't start
```bash
# Check if port 3671 is already in use
lsof -i :3671

# Kill any process using the port
kill -9 <PID>
```

**Problem:** Pico can't discover simulator
- Verify Pico and computer are on the same WiFi network
- Check firewall settings (allow UDP port 3671)
- Verify simulator is running with verbose output

### Compilation Errors

**Problem:** Target not found
```bash
rustup target add thumbv8m.main-none-eabihf
```

**Problem:** Feature flag conflicts
- Use either `embassy-rp` OR `embassy-rp-usb`, not both
- Clean build: `cargo clean && cargo build-rp2040-usb`

### Hardware Issues

**Problem:** Pico not recognized by picotool
- Ensure Pico is in BOOTSEL mode (hold button while connecting USB)
- Check USB cable (must support data, not just power)
- Verify picotool is installed: `picotool version`

**Problem:** WiFi connection fails
- Check SSID and password in configuration
- Ensure 2.4GHz WiFi (Pico 2 W doesn't support 5GHz)
- Check WiFi signal strength

**Problem:** KNX connection timeout
- Verify gateway IP is correct and reachable
- Check gateway is powered on and connected to network
- Ensure UDP port 3671 is not blocked by firewall
- With simulator: ensure simulator is running

### Serial Monitor Issues

**Problem:** No output in serial monitor
- Verify correct USB device: `ls /dev/tty.usbmodem*`
- Check baud rate: 115200
- Ensure USB logger feature is enabled: `embassy-rp-usb`

**Problem:** Garbled output
- Try different baud rates
- Reconnect USB cable
- Restart serial monitor

## Best Practices

1. **Always start simulator first** when testing without hardware
2. **Use USB logger for debugging** - easier to view output than defmt
3. **Test with simulator before physical hardware** - safer and faster iteration
4. **Keep simulator logs visible** - helps understand protocol flow
5. **Use release builds** - debug builds may timeout due to slower execution
6. **Implement heartbeat** - for long-running applications (every 60 seconds)

## Additional Resources

- [KNX Association](https://www.knx.org/) - Official KNX specifications
- [examples/README.md](examples/README.md) - Example usage and documentation
- [KNX_DISCOVERY.md](KNX_DISCOVERY.md) - Gateway discovery protocol details

## Getting Help

If you encounter issues:
1. Check this troubleshooting guide
2. Review example code and comments
3. Examine simulator verbose output for protocol errors
4. Open an issue on GitHub with detailed logs and steps to reproduce

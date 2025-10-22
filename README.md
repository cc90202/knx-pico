# knx-pico

[![Crates.io](https://img.shields.io/crates/v/knx-pico.svg)](https://crates.io/crates/knx-pico)
[![Documentation](https://docs.rs/knx-pico/badge.svg)](https://docs.rs/knx-pico)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](README.md#license)

A `no_std` KNXnet/IP protocol implementation for embedded systems, designed for the Embassy async runtime.

## Features

- 🚀 **`no_std` compatible** - Runs on bare metal embedded systems
- ⚡ **Zero-copy parsing** - Efficient memory usage for resource-constrained devices
- 🔄 **Async/await** - Full Embassy async runtime integration
- 🎯 **Type-safe addressing** - Strong types for Individual and Group addresses
- 🔌 **KNXnet/IP tunneling** - Reliable point-to-point communication
- 📊 **Datapoint Types (DPT)** - Support for DPT 1, 3, 5, 7, 9, 13
- 🔍 **Gateway auto-discovery** - Automatic KNX gateway detection via multicast
- 🛡️ **Production-ready** - Thoroughly tested with hardware and simulator

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
knx-pico = "0.1"
```

### Basic Example

```rust
use knx_pico::{GroupAddress, protocol::cemi::CemiFrame, dpt::{Dpt1, DptEncode}};

// Create a group address
let light = GroupAddress::new(1, 2, 3)?;

// Encode a boolean value (DPT 1 - Switch)
let value = Dpt1::new(true);
let encoded = value.encode();

// Create a write request frame
let frame = CemiFrame::write_request(light.into(), &encoded)?;

// The frame can now be sent over KNXnet/IP tunnel
// (requires Embassy runtime and network stack - see examples on GitHub)
```

For complete examples with Embassy runtime and Raspberry Pi Pico 2 W, see the [examples directory on GitHub](https://github.com/cc90202/knx-pico/tree/master/examples):
- **`pico_knx_async.rs`** - Complete working example for Pico 2 W
- **`knx_sniffer.rs`** - Interactive testing tool with convenience macros

## Hardware Support

### Tested Platforms

- ✅ **Raspberry Pi Pico 2 W** (RP2350) - Primary development platform
- 🔜 **ESP32-C3/C6** - Planned support via `embassy-esp`

### Required Hardware

For physical KNX testing:
- Raspberry Pi Pico 2 W (or compatible RP2350 board)
- KNX/IP Gateway (e.g., Gira X1, MDT SCN-IP000.03)
- WiFi network

**For testing without hardware:** Use the included Python KNX simulator (see [Testing](#testing)).

## Architecture

### Layer Overview

KNX communication uses three nested protocol layers:

```text
┌─────────────────────────────────────────────────┐
│  KNXnet/IP FRAME (UDP transport)                │
│  ┌───────────────────────────────────────────┐  │
│  │ CEMI (KNX command)                        │  │
│  │  ┌─────────────────────────────────────┐ │  │
│  │  │ DPT (encoded value)                 │ │  │
│  │  │ e.g., true → [0x01]                 │ │  │
│  │  └─────────────────────────────────────┘ │  │
│  └───────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

| Layer | Purpose | Example |
|-------|---------|---------|
| **DPT** | Encode values | `21.5°C` → `\[0x0C, 0x1A\]` |
| **CEMI** | KNX commands | "Write to 1/2/3: \[0x01\]" |
| **KNXnet/IP** | IP transport | UDP to 192.168.1.10:3671 |

### Module Structure

```text
knx-pico/
├── addressing/          # KNX addressing (Individual & Group)
├── protocol/            # KNXnet/IP protocol implementation
│   ├── frame.rs         # Layer 1: KNXnet/IP frames
│   ├── cemi.rs          # Layer 2: CEMI messages
│   ├── services.rs      # Tunneling service builders
│   ├── tunnel.rs        # Typestate tunneling client
│   └── async_tunnel.rs  # Async wrapper for Embassy
├── dpt/                 # Layer 3: Datapoint Types (DPT)
│   ├── dpt1.rs          # Boolean (switches, buttons)
│   ├── dpt3.rs          # 3-bit control (dimming, blinds)
│   ├── dpt5.rs          # 8-bit unsigned (percentage, angle)
│   ├── dpt7.rs          # 16-bit unsigned (counter, brightness)
│   ├── dpt9.rs          # 2-byte float (temperature, humidity)
│   └── dpt13.rs         # 32-bit signed (energy, flow)
├── knx_discovery.rs     # Gateway auto-discovery
├── knx_client.rs        # High-level client API
├── error.rs             # Comprehensive error types
└── lib.rs               # Public API
```

## Convenience Macros

The library provides macros to simplify common operations:

```rust
use knx_pico::{ga, knx_write, knx_read, KnxValue};

// Create group addresses with readable notation
let light = ga!(1/2/3);
let temp_sensor = ga!(1/2/10);

// Write values with inline address notation
knx_write!(client, 1/2/3, KnxValue::Bool(true)).await?;
knx_write!(client, 1/2/10, KnxValue::Temperature(21.5)).await?;

// Read values
knx_read!(client, 1/2/10).await?;

// Register multiple DPT types at once
register_dpts! {
    client,
    1/2/3  => Bool,
    1/2/5  => Percent,
    1/2/10 => Temperature,
}?;
```

## Building and Flashing

### For Raspberry Pi Pico 2 W

#### Option 1: USB Logger (Recommended - No probe needed)

```bash
# Configure WiFi in src/configuration.rs
# Build and flash in one command
cargo flash-example-usb

# Monitor logs via USB serial
screen /dev/tty.usbmodem* 115200
```

#### Option 2: defmt Logger (Requires debug probe)

```bash
# Build with defmt-rtt
cargo build --release --example pico_knx_async \
    --target thumbv8m.main-none-eabihf \
    --features embassy-rp

# Flash with probe-rs
probe-rs run --chip RP2350 \
    target/thumbv8m.main-none-eabihf/release/examples/pico_knx_async
```

### Available Commands

See `.cargo/config.toml` for all commands:

```bash
# Examples
cargo flash-example-usb          # Flash pico_knx_async (USB logger)
cargo flash-sniffer-usb-release  # Flash knx_sniffer (USB logger)

# Library checks
cargo check-rp2040              # Check for RP2040 target
cargo test-host-release         # Run host tests (optimized)

# Full verification
./check-all.sh                  # Run all checks
```

## Testing

### Without Physical Hardware

Use the included Python KNX simulator for development and testing:

```bash
# Start simulator
python3 knx_simulator.py

# Run integration tests
python3 test_runner.py

# Or use Make
make test              # All tests
make test-unit         # Unit tests only
```

### With Physical Hardware

1. Configure WiFi credentials in `src/configuration.rs`:
   ```rust
   pub const CONFIG: &str = r#"
   WIFI_NETWORK=Your_WiFi_SSID
   WIFI_PASSWORD=Your_WiFi_Password
   "#;
   ```

2. Flash to hardware:
   ```bash
   cargo flash-example-usb
   ```

3. Monitor logs:
   ```bash
   screen /dev/tty.usbmodem* 115200
   ```

See [TESTING.md](TESTING.md) for detailed testing guide.

## Gateway Auto-Discovery

The library automatically discovers KNX gateways using multicast SEARCH_REQUEST:

```rust
use knx_pico::knx_discovery;
use embassy_time::Duration;

// Discover gateway (3 second timeout)
let gateway = knx_discovery::discover_gateway(&stack, Duration::from_secs(3))
    .await
    .expect("No KNX gateway found");

println!("Found gateway at {}:{}", gateway.ip, gateway.port);
```

No manual IP configuration needed! See [KNX_DISCOVERY.md](KNX_DISCOVERY.md) for details.

## Supported Datapoint Types (DPT)

| DPT | Type | Description | Example |
|-----|------|-------------|---------|
| **1.xxx** | Boolean | Switches, buttons, binary sensors | `true`/`false` |
| **3.007** | 3-bit | Dimming control (increase/decrease) | `+4 steps` |
| **3.008** | 3-bit | Blinds control (up/down) | `down 2 steps` |
| **5.001** | 8-bit | Percentage (0-100%) | `75%` |
| **5.010** | 8-bit | Unsigned value (0-255) | `192` |
| **7.001** | 16-bit | Counter, pulses (0-65535) | `5000 lux` |
| **9.001** | 2-byte float | Temperature (°C) | `21.5°C` |
| **9.004** | 2-byte float | Illuminance (lux) | `5000 lux` |
| **9.007** | 2-byte float | Humidity (%) | `65%` |
| **13.xxx** | 32-bit | Energy, flow rate, long counters | `500000 Wh` |

See [`src/dpt/`](src/dpt/) for implementation details.

## Documentation

- **[TESTING.md](TESTING.md)** - Testing guide with simulator setup
- **[KNX_DISCOVERY.md](KNX_DISCOVERY.md)** - Gateway discovery details
- **[PRE_PUBLISH_GUIDE.md](PRE_PUBLISH_GUIDE.md)** - Pre-publish checklist
- **[examples/README.md](examples/README.md)** - Example documentation
- **[API Documentation](https://docs.rs/knx-pico)** - Full API reference

## Project Status

✅ **Production Ready**

All core features implemented and tested:
- ✅ KNXnet/IP protocol (Frame, CEMI, Services)
- ✅ Datapoint Types (DPT 1, 3, 5, 7, 9, 13)
- ✅ Tunneling client with typestate pattern
- ✅ Embassy + RP2040 integration (Pico 2 W)
- ✅ Gateway auto-discovery via multicast
- ✅ High-level client API with macros
- ✅ Comprehensive testing (unit, integration, hardware)
- ✅ CI/CD automation (GitHub Actions)

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Run `./check-all.sh` to verify all checks pass
5. Submit a pull request

## Performance

Optimized for embedded systems:
- **Zero-copy parsing** - Minimal memory allocations
- **Inline hot paths** - Critical functions marked `#[inline]`
- **Unsafe optimizations** - Bounds checks eliminated where safe (documented with `// SAFETY:` comments)
- **~10% performance gain** on parsing hot paths
- **Fire-and-forget pattern** - Optimized command sending for stability

Benchmarked on Raspberry Pi Pico 2 W (RP2350, 150 MHz).

## Troubleshooting

### Gateway not found during discovery

1. Verify gateway is powered on and connected to network
2. Check that multicast is enabled on your WiFi network
3. Increase discovery timeout to 5 seconds
4. Ensure your WiFi network allows multicast traffic (224.0.23.12)

### Connection timeouts

1. Verify gateway IP and port (usually 3671)
2. Check firewall settings (UDP port 3671 must be open)
3. Ensure only one client connects to gateway at a time

### Compilation errors

1. Update Rust toolchain: `rustup update`
2. Install RP2040 target: `rustup target add thumbv8m.main-none-eabihf`
3. For USB logger: ensure `picotool` is installed
4. For defmt: ensure `probe-rs` is installed

See [TESTING.md](TESTING.md) for detailed troubleshooting guide.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

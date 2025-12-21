# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**knx-pico** is a `no_std` KNXnet/IP protocol implementation for embedded systems, designed for the Embassy async runtime. The library provides KNX home automation communication for resource-constrained devices like the Raspberry Pi Pico 2 W.

## Build and Test Commands

### Library Development

```bash
# Check library compiles (no_std)
cargo check --lib

# Run unit tests on host (required for no_std testing)
cargo test-host                    # Debug mode
cargo test-host-release            # Optimized

# Run all checks (library + embedded + tests)
./check-all.sh
```

### Embedded Target (Raspberry Pi Pico 2 W)

The project supports two logging approaches:

**USB Logger (Recommended - no debug probe needed):**
```bash
# Flash examples with USB logger
cargo flash-example-usb              # pico_knx_async example
cargo flash-sniffer-usb-release      # knx_sniffer tool
cargo flash-main-app-usb-release     # knx_main_application template

# Monitor USB serial output
screen /dev/tty.usbmodem* 115200
```

**defmt Logger (requires debug probe):**
```bash
# Check/build for RP2040
cargo check-rp2040                 # defmt logger
cargo check-rp2040-usb             # USB logger

# Flash with probe
cargo flash-rp2040                 # Binary
cargo flash-sniffer-release        # KNX sniffer tool
```

### Testing

```bash
# Full test suite (unit + integration + examples)
make test
python3 test_runner.py --verbose

# Individual test suites
make test-unit                     # Unit tests only
make test-integration              # Integration tests with simulator
make test-examples                 # Check examples compile

# Start KNX simulator for testing without hardware
python3 knx_simulator.py
```

### Documentation

```bash
# Build and open documentation
cargo doc --lib --all-features --no-deps --open
make docs
```

### Pre-publish Checks

```bash
make pre-publish                   # Run all checks before publishing
```

## Architecture

### Protocol Layers

The library implements three nested KNX protocol layers:

1. **KNXnet/IP Frame** (`src/protocol/frame.rs`) - UDP transport layer
2. **CEMI** (`src/protocol/cemi.rs`) - KNX command messages
3. **DPT** (`src/dpt/`) - Datapoint type encoding/decoding

Each layer wraps the next, creating a nested structure: `KNXnet/IP[CEMI[DPT]]`.

### Key Components

**Addressing** (`src/addressing/`)
- `GroupAddress` - KNX group addressing (3-level: main/middle/sub)
- `IndividualAddress` - Device physical addressing

**Protocol** (`src/protocol/`)
- `frame.rs` - KNXnet/IP frame parsing (zero-copy)
- `cemi.rs` - Common External Message Interface
- `services.rs` - Service builders (CONNECT, DISCONNECT, etc.)
- `tunnel.rs` - **Typestate pattern** tunneling client (compile-time state validation)
- `async_tunnel.rs` - Embassy async wrapper over TunnelClient

**DPT** (`src/dpt/`)
- `dpt1.rs` - Boolean (switches, buttons)
- `dpt3.rs` - 3-bit control (dimming, blinds)
- `dpt5.rs` - 8-bit unsigned (percentage, angle)
- `dpt7.rs` - 16-bit unsigned (counter, brightness)
- `dpt9.rs` - 2-byte float (temperature, humidity)
- `dpt13.rs` - 32-bit signed (energy, flow)

**High-level Client** (`src/knx_client.rs`)
- `KnxClient` - High-level API with DPT registry
- Builder pattern for configuration
- Convenience macros: `ga!()`, `knx_write!()`, `knx_read!()`

**Discovery** (`src/knx_discovery.rs`)
- Multicast gateway auto-discovery (224.0.23.12)
- No manual IP configuration needed

### Typestate Pattern

The `TunnelClient` uses the typestate pattern to enforce correct state transitions at compile time:

```rust
// States: Idle → Connecting → Connected → Disconnecting → Idle
let client = TunnelClient::new(...);           // Idle
let (client, frame) = client.connect()?;       // Connecting
let client = client.handle_connect_response()?; // Connected
let frame = client.send_tunneling_request()?;  // Only works in Connected!
```

This prevents calling methods in invalid states (e.g., sending data before connecting).

### Critical Implementation Details

**Fire-and-Forget Pattern** (`src/protocol/async_tunnel.rs`)
- Commands are sent without waiting for TUNNELING_ACK
- Gateway automatically sends ACK ~50-100ms later
- Improves stability on resource-constrained devices
- See `FLUSH_TIMEOUT` and related constants for tuning

**Response Timeouts**
- `RESPONSE_TIMEOUT = 200ms` - Prevents stack overflow on Pico 2 W
- Never increase beyond 200ms on embedded targets
- Longer timeouts cause hard resets due to stack constraints

**Zero-Copy Parsing**
- All protocol parsers use slices, no allocations
- Unsafe optimizations with `// SAFETY:` documentation
- Critical for embedded performance (~10% parsing speedup)

**Buffer Management**
- `AsyncTunnelClient` uses fixed-size buffers
- Default: 2KB RX, 2KB TX, 4 metadata entries
- Configurable via builder pattern

## Configuration

**WiFi Credentials** (`src/configuration.rs`)
```rust
pub const CONFIG: &str = r#"
WIFI_NETWORK=Your_WiFi_SSID
WIFI_PASSWORD=Your_WiFi_Password
KNX_GATEWAY_IP=192.168.1.10  # Optional, discovery used by default
"#;
```

This file is gitignored and must be created from `configuration.rs.example`.

## Examples

Located in `examples/`:
- `pico_knx_async.rs` - Complete working example for Pico 2 W
- `knx_sniffer.rs` - Interactive testing tool with macros
- `knx_main_application.rs` - Application template

Examples use Embassy runtime and demonstrate:
- WiFi connection
- Gateway discovery
- Tunneling connection
- Sending/receiving KNX commands
- DPT encoding/decoding

## Testing Strategy

**Unit Tests** - Run on host with `cargo test-host`
- Protocol parsing (frame, CEMI, DPT)
- Address validation
- State machine transitions

**Integration Tests** (`tests/integration_test.rs`)
- Uses Python KNX simulator (`knx_simulator.py`)
- Tests full protocol flow
- No hardware required

**Hardware Tests** - Flash to Pico 2 W
- Real KNX gateway communication
- WiFi connectivity
- Production validation

## Linting and Code Quality

The project uses extensive Clippy lints (see `Cargo.toml` `[lints.clippy]`):
- All major categories enabled (pedantic, perf, complexity, etc.)
- Custom opt-outs for embedded context (e.g., `cast_possible_truncation`)
- Unsafe blocks require `// SAFETY:` comments
- Run `cargo clippy --all-targets --all-features` before committing

## Performance Considerations

**Optimizations Applied:**
- `#[inline(always)]` on hot paths (frame parsing, DPT encoding)
- Unsafe pointer arithmetic where bounds-checked
- Zero-copy parsing throughout
- Release profile: `opt-level = "z"`, `lto = "fat"`, `codegen-units = 1`

**Memory Constraints:**
- Target: RP2350 with 520KB SRAM
- Stack usage critical - avoid deep recursion
- All allocations from `heapless` containers (fixed-size)

## Publishing Workflow

1. Update version in `Cargo.toml`
2. Run `make pre-publish` - validates all checks
3. Commit and tag: `git tag v0.x.x`
4. Publish: `cargo publish`
5. See `PRE_PUBLISH_GUIDE.md` for detailed checklist

## Common Patterns

**Creating Group Addresses:**
```rust
use knx_pico::ga;
let light = ga!(1/2/3);  // Macro
let temp = GroupAddress::new(1, 2, 10)?;  // Explicit
```

**Sending Commands:**
```rust
use knx_pico::{knx_write, KnxValue};
knx_write!(client, 1/2/3, KnxValue::Bool(true)).await?;
```

**Registering DPT Types:**
```rust
use knx_pico::register_dpts;
register_dpts! {
    client,
    1/2/3  => Bool,
    1/2/10 => Temperature,
}?;
```

## Feature Flags

- `std` - Enable std support (for examples)
- `defmt` - Enable defmt logging
- `serde` - Enable serde serialization
- `embassy-rp` - RP2040 support with defmt-rtt logger
- `embassy-rp-usb` - RP2040 support with USB logger
- `usb-logger` - USB logger feature

Default feature set is minimal (`no_std`, no logging).

## Debugging

**USB Logger Output:**
```bash
# After flashing with USB logger
screen /dev/tty.usbmodem* 115200
# Ctrl-A, K to exit screen
```

**defmt Logger Output:**
```bash
# With debug probe connected
probe-rs run --chip RP2350 target/thumbv8m.main-none-eabihf/release/examples/pico_knx_async
```

**Simulator Debugging:**
```bash
# Start simulator with verbose logging
python3 knx_simulator.py
# Simulator logs all KNX protocol messages
```

## Important Notes

- **No `std` in library code** - Only `examples/` and `tests/` can use std
- **Async requires Embassy** - No other async runtimes supported
- **RP2350 only** - Other targets (ESP32, etc.) planned but not tested
- **Single connection** - Gateway supports only one tunnel connection at a time
- **Heartbeat required** - Call `send_heartbeat()` every 60s or connection drops
- **Flush before send** - `AsyncTunnelClient` flushes pending packets before new commands
- nelle commit non mettere mai riferimenti a anthropic e/o claude; fai commit sintetiche e senza icone
- non committare mai le mie credenziali wifi
- ricordati che questo progetto ha bisogno della versione rust nightly
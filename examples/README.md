# KNX-RS Examples

This directory contains practical examples demonstrating how to use `knx-pico` library.

## Prerequisites

**⚠️ Important:** All examples require either:
- **Physical KNX hardware**: A KNX/IP gateway on your local network (e.g., Gira X1, MDT SCN-IP000.03, etc.)
- **Simulator**: For testing without physical hardware, run the KNX gateway simulator:

```bash
# Start the simulator (in a separate terminal)
python3 knx_simulator.py
```

The simulator must be running before executing any examples. See [../TESTING.md](../TESTING.md) for detailed setup instructions.

## Examples

### `pico_knx_async.rs`

Complete example showing KNX communication with Raspberry Pi Pico 2 W over WiFi.

**Features:**
- WiFi connection with CYW43 driver
- **Automatic KNX gateway discovery** via multicast (no hardcoded IPs!)
- Async KNXnet/IP tunneling client
- Send GroupValue_Write commands (turn lights on/off)
- Receive and parse GroupValue_Indication events from KNX bus
- Uses `ga!` macro for readable group address notation

**Hardware Requirements:**
- Raspberry Pi Pico 2 W
- KNX gateway on local network **OR** KNX simulator (see Prerequisites above)
- WiFi network

**Setup:**

1. **Configure WiFi** in `src/configuration.rs`:
   ```rust
   pub const CONFIG: &str = r#"
   WIFI_NETWORK=Your_WiFi_SSID
   WIFI_PASSWORD=Your_WiFi_Password
   "#;
   ```

   **Note:** The KNX gateway is automatically discovered via multicast - no manual IP configuration needed!

2. Update KNX group addresses for your devices using the convenient macros:
   ```rust
   // ga! macro - Create group addresses with readable notation
   let light_addr = ga!(1/2/3);   // Living room light
   let dimmer_addr = ga!(1/2/5);  // Dimmer control
   let valve_addr = ga!(1/2/6);   // Valve position
   let temp_addr = ga!(1/2/7);    // Temperature sensor
   ```

3. Flash to Pico:

   **Option 1: USB logger (recommended):**
   ```bash
   cargo flash-example-usb
   # Monitor: screen /dev/tty.usbmodem* 115200
   ```

   **Option 2: defmt logger (requires probe):**
   ```bash
   cargo build --release --example pico_knx_async --target thumbv8m.main-none-eabihf --features embassy-rp
   probe-rs run --chip RP2350 target/thumbv8m.main-none-eabihf/release/examples/pico_knx_async
   ```

**What it does:**

1. **Connects to WiFi**: Joins your WiFi network using CYW43 driver
2. **Gets IP via DHCP**: Waits for network configuration
3. **Discovers KNX gateway**: Automatically finds gateway via multicast (or uses fallback IP)
4. **Connects to KNX gateway**: Establishes KNXnet/IP tunnel connection
5. **Sends commands** demonstrating different DPT types:
   - **DPT 1 (Boolean)**: Turns ON/OFF a light
   - **DPT 3 (Dimming)**: Increases brightness by 4 steps
   - **DPT 5 (Percentage)**: Sets valve position to 75%
   - **DPT 9 (Temperature)**: Writes temperature setpoint (21.5°C)
6. **Listens for events**: Receives and parses all KNX bus traffic

**Important**: The example runs for a short time. For long-running applications, you MUST implement heartbeat/keep-alive by calling `client.send_heartbeat()` every 60 seconds, otherwise the gateway will close the connection.

**Understanding the Code:**

The example demonstrates low-level cEMI frame construction for different DPT types:

```rust
// DPT 1: Boolean (1-bit in 6-bit APCI)
fn build_group_write_bool(group_addr: GroupAddress, value: bool) -> [u8; 11] {
    // Frame: 11 bytes (standard + 1 byte NPDU)
    // APCI: 0x80 (write) + value (0 or 1)
    frame[10] = if value { 0x81 } else { 0x80 };
}

// DPT 3: Dimming/Blind Control (4-bit control)
fn build_group_write_dpt3(group_addr: GroupAddress, value: u8) -> [u8; 11] {
    // Format: cccc SUUU (control, step direction, step code)
    // Example: 0x0B = increase by 4 steps
    frame[10] = 0x80 | (value & 0x0F);
}

// DPT 5: Percentage (8-bit unsigned)
fn build_group_write_dpt5(group_addr: GroupAddress, value: u8) -> [u8; 12] {
    // Frame: 12 bytes (standard + 2 bytes NPDU)
    // Range: 0x00 (0%) to 0xFF (100%)
    frame[11] = value;
}

// DPT 9: Temperature (2-byte float)
fn build_group_write_dpt9(group_addr: GroupAddress, high: u8, low: u8) -> [u8; 13] {
    // Frame: 13 bytes (standard + 3 bytes NPDU)
    // Format: MEEE EMMM MMMM MMMM (mantissa + exponent)
    // Value = (0.01 * M) * 2^E
    frame[11] = high;
    frame[12] = low;
}
```

This comprehensive example helps you understand:
- **Frame structure** for different data types
- **NPDU length** varies by DPT (1-3 bytes)
- **Encoding rules** for each DPT type

**Heartbeat / Keep-Alive:**

For production applications that run continuously, you must send heartbeat every 60 seconds:

```rust
use embassy_time::Instant;

let mut last_heartbeat = Instant::now();

loop {
    // Check if 60 seconds have passed
    if last_heartbeat.elapsed() > Duration::from_secs(60) {
        client.send_heartbeat().await?;
        last_heartbeat = Instant::now();
        info!("❤️ Heartbeat sent");
    }

    // Your application logic here
    if let Some(cemi) = client.receive().await? {
        // Process events
    }
}
```

The KNX gateway will close the tunnel if no heartbeat is received for ~120 seconds.

**Tips:**

- Use ETS (Engineering Tool Software) to find your device group addresses
- The example uses 3-level addressing format (main/middle/sub)
- For 2-level format, use `GroupAddress::new_2level(main, sub)`
- Monitor your KNX bus with ETS Group Monitor to verify commands
- Always implement heartbeat for long-running applications

**Troubleshooting:**

- **WiFi connection fails**: Check SSID and password
- **KNX connection fails**: Verify gateway IP, ensure UDP port 3671 is accessible
- **No response from devices**: Check group addresses match your ETS configuration
- **Compilation errors**: Ensure CYW43 firmware files are in `cyw43-firmware/` directory

---

### `knx_sniffer.rs`

Interactive KNX sniffer/tester tool for debugging and testing KNX communication.

**Features:**
- Gateway discovery via multicast
- **Convenience macros demonstrated**: `ga!`, `knx_read!`, `knx_write!`
- High-level `KnxClient` API demonstration
- DPT type registration and response examples
- Event monitoring
- USB or defmt logging support

**Hardware Requirements:**
- Raspberry Pi Pico 2 W
- KNX gateway on local network **OR** KNX simulator (see Prerequisites above)
- WiFi network

**Setup and Running:**

**Option 1: USB Logger (recommended for interactive debugging)**
```bash
# Build and flash with USB logger
cargo flash-sniffer-usb-release

# Open serial monitor to view output
screen /dev/tty.usbmodem* 115200
```

**Option 2: defmt Logger (faster logging)**
```bash
# Build and flash with defmt
cargo flash-sniffer-release

# Logs visible via probe-rs
```

**Available Commands:**
```bash
# Check compilation
cargo check-sniffer-usb      # USB logger
cargo check-sniffer          # defmt logger

# Build
cargo build-sniffer-usb-release    # USB + release
cargo build-sniffer-release        # defmt + release

# Flash to Pico
cargo flash-sniffer-usb-release    # USB + release (recommended)
cargo flash-sniffer-release        # defmt + release
```

**What it does:**
1. Connects to WiFi
2. Discovers KNX gateway via multicast
3. Establishes tunnel connection
4. Demonstrates convenience macros:
   - `ga!` - Group address creation
   - `knx_read!` - Read request
   - `knx_write!` - Write commands
5. Shows DPT type registration and response operations
6. Monitors KNX bus events (optional, disabled by default)

**Note:** Ensure the KNX simulator is running if you don't have physical hardware!

## Building and Flashing Examples

All embedded examples target Raspberry Pi Pico 2 W and require either USB or defmt logger.

### Quick Commands

**pico_knx_async:**
```bash
# USB logger (recommended)
cargo flash-example-usb

# defmt logger (requires probe)
cargo build --release --example pico_knx_async --target thumbv8m.main-none-eabihf --features embassy-rp
probe-rs run --chip RP2350 target/thumbv8m.main-none-eabihf/release/examples/pico_knx_async
```

**knx_sniffer:**
```bash
# USB logger (recommended)
cargo flash-sniffer-usb-release

# defmt logger (requires probe)
cargo flash-sniffer-release
```

### Manual Build Process

If you need to customize the build:

```bash
# Build for USB logger
cargo build --release --example <example_name> --target thumbv8m.main-none-eabihf --features embassy-rp-usb

# Build for defmt logger
cargo build --release --example <example_name> --target thumbv8m.main-none-eabihf --features embassy-rp

# Flash with picotool (USB logger builds)
picotool load -u -v -x -t elf target/thumbv8m.main-none-eabihf/release/examples/<example_name>

# Flash with probe-rs (defmt builds)
probe-rs run --chip RP2350 target/thumbv8m.main-none-eabihf/release/examples/<example_name>
```

**Note:** All examples run on hardware only, not with `cargo run`. They must be flashed to Pico 2 W.

## Supported DPT Types

The `pico_knx_async.rs` example demonstrates the following KNX datapoint types:

- **DPT 1**: Boolean (switch, on/off)
- **DPT 3**: 4-bit control (dimming, blinds)
- **DPT 5**: 8-bit unsigned (percentage, angle, counter)
- **DPT 9**: 2-byte float (temperature, illuminance, wind speed)

These cover the most common use cases in KNX home automation. Additional DPT types can be implemented following the same pattern.

## Convenience Macros

The `knx-pico` library provides several macros to make your code more readable and concise:

### `ga!` - Group Address Creation

Create group addresses using familiar 3-level notation:

```rust
use knx_pico::ga;

let light = ga!(1/2/3);        // Main=1, Middle=2, Sub=3
let temp_sensor = ga!(1/2/10);
let dimmer = ga!(2/1/5);
```

### `knx_write!` - Simplified Write

Write values with inline address notation:

```rust
use knx_pico::{knx_write, KnxValue};

// Turn on a light
knx_write!(client, 1/2/3, KnxValue::Bool(true)).await?;

// Set temperature setpoint
knx_write!(client, 1/2/10, KnxValue::Temperature(21.5)).await?;

// Set dimmer percentage
knx_write!(client, 2/1/5, KnxValue::Percent(75)).await?;
```

### `knx_read!` - Simplified Read

Request values with inline address notation:

```rust
use knx_pico::knx_read;

// Request current temperature
knx_read!(client, 1/2/10).await?;

// Request switch state
knx_read!(client, 1/2/3).await?;

// Response arrives via receive_event()
match client.receive_event().await? {
    Some(KnxEvent::GroupResponse { address, value }) => {
        // Handle the response
    }
    _ => {}
}
```

### `knx_respond!` - Simplified Response

Respond to read requests with inline address notation:

```rust
use knx_pico::{knx_respond, KnxValue, KnxEvent};

// Handle read requests
match client.receive_event().await? {
    Some(KnxEvent::GroupRead { address }) => {
        // Respond with current value
        knx_respond!(client, 1/2/10, KnxValue::Temperature(21.5)).await?;
    }
    _ => {}
}
```

### `register_dpts!` - Batch DPT Registration

Register multiple DPT types in a single block:

```rust
use knx_pico::{register_dpts, DptType};

register_dpts! {
    client,
    1/2/3  => Bool,         // Light switch
    1/2/5  => Percent,      // Dimmer
    1/2/10 => Temperature,  // Temp sensor
    1/2/11 => Humidity,     // Humidity sensor
    2/1/5  => Lux,          // Light sensor
}?;
```

**Important Notes:**
- These macros (`knx_write!`, `knx_read!`, `knx_respond!`, `register_dpts!`) work **only** with the high-level `KnxClient` API
- The `ga!` macro works with **both** `KnxClient` and `AsyncTunnelClient`
- The `knx_sniffer.rs` example demonstrates macro usage with `KnxClient`
- The `pico_knx_async.rs` example uses `AsyncTunnelClient` for low-level frame construction (educational purposes)

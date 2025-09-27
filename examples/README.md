# KNX-RS Examples

This directory contains practical examples demonstrating how to use `knx-rs` library.

## Examples

### `pico_knx_async.rs`

Complete example showing KNX communication with Raspberry Pi Pico 2 W over WiFi.

**Features:**
- WiFi connection with CYW43 driver
- Async KNXnet/IP tunneling client
- Send GroupValue_Write commands (turn lights on/off)
- Receive and parse GroupValue_Indication events from KNX bus

**Hardware Requirements:**
- Raspberry Pi Pico 2 W
- KNX gateway on local network (e.g., Gira X1, MDT SCN-IP000.03, etc.)
- WiFi network

**Setup:**

1. Configure WiFi and KNX gateway in `examples/pico_knx_async.rs`:
   ```rust
   const WIFI_SSID: &str = "Your_WiFi_SSID";
   const WIFI_PASSWORD: &str = "Your_WiFi_Password";
   const KNX_GATEWAY_IP: [u8; 4] = [192, 168, 1, 10]; // Your gateway IP
   ```

2. Update KNX group addresses for your devices:
   ```rust
   const LIGHT_LIVING_ROOM_RAW: u16 = 0x0A03; // 1/2/3
   ```

3. Build and flash:
   ```bash
   cargo build-rp2040 --example pico_knx_async --release
   probe-rs run --chip RP2350 target/thumbv8m.main-none-eabihf/release/examples/pico_knx_async
   ```

4. Monitor output:
   ```bash
   # The example uses defmt + defmt-rtt for logging
   probe-rs run --chip RP2350 target/thumbv8m.main-none-eabihf/release/examples/pico_knx_async
   ```

**What it does:**

1. **Connects to WiFi**: Joins your WiFi network using CYW43 driver
2. **Gets IP via DHCP**: Waits for network configuration
3. **Connects to KNX gateway**: Establishes KNXnet/IP tunnel connection
4. **Sends commands**:
   - Turns ON a light (GroupValue_Write with DPT 1 = true)
   - Turns OFF the light after 2 seconds
5. **Listens for events**: Receives and parses all KNX bus traffic

**Important**: The example runs for a short time. For long-running applications, you MUST implement heartbeat/keep-alive by calling `client.send_heartbeat()` every 60 seconds, otherwise the gateway will close the connection.

**Understanding the Code:**

The example demonstrates low-level cEMI frame construction:

```rust
// Build a GroupValue_Write frame for boolean (DPT 1)
fn build_group_write_bool(group_addr: GroupAddress, value: bool) -> [u8; 11] {
    let mut frame = [0u8; 11];

    frame[0] = CEMIMessageCode::LDataReq.to_u8();  // L_Data.req
    frame[1] = 0x00;                                // No additional info
    frame[2] = ControlField1::default().raw();      // Standard frame
    frame[3] = ControlField2::default().raw();      // Group address
    // ... source and destination addresses
    frame[8] = 0x01;                                // NPDU length
    frame[9] = 0x00;                                // TPCI
    frame[10] = if value { 0x81 } else { 0x80 };   // APCI + value

    frame
}
```

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

## Building Examples

All examples require the `embassy-rp` feature:

```bash
# Check compilation
cargo check-rp2040 --example pico_knx_async

# Build release binary
cargo build-rp2040 --example pico_knx_async --release

# Flash to hardware
probe-rs run --chip RP2350 target/thumbv8m.main-none-eabihf/release/examples/pico_knx_async
```

## Future Examples

Planned examples:
- Temperature sensor with DPT 9
- Dimmer control with DPT 5
- Multi-device control
- High-level API usage (when available)

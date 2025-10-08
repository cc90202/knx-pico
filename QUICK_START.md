# KNX-RS Quick Start Guide

**5-Minute Setup for Hardware Testing** ⚡

---

## Step 1: Configure (2 minutes)

Edit `src/configuration.rs`:

```rust
pub const CONFIG: &str = r#"
WIFI_NETWORK=YourActualWiFiName
WIFI_PASSWORD=YourActualPassword
KNX_GATEWAY_IP=192.168.1.50
"#;
```

💡 **Tips:**
- WiFi must be 2.4 GHz (Pico 2 W doesn't support 5 GHz)
- SSID and password are case-sensitive
- Gateway IP should be on same network

---

## Step 2: Flash (3 minutes)

### Automatic Method (Recommended)

```bash
./flash.sh
```

Follow the prompts to build and flash.

### Manual Method

```bash
# Build
cargo build --release --target thumbv8m.main-none-eabihf --features embassy-rp

# Flash with probe-rs
probe-rs run --chip RP2350 target/thumbv8m.main-none-eabihf/release/knx-rs

# OR convert to UF2 for drag-and-drop
elf2uf2-rs target/thumbv8m.main-none-eabihf/release/knx-rs knx-rs.uf2
# Then: Hold BOOTSEL, connect USB, copy knx-rs.uf2 to drive
```

---

## Step 3: Monitor

### With probe-rs (Real-Time)

```bash
probe-rs attach --chip RP2350
```

### With USB Serial

```bash
# macOS/Linux
screen /dev/tty.usbmodem* 115200

# Windows
# Use PuTTY or TeraTerm
```

---

## Expected Output

```
Random seed: 123456789
Connecting to WiFi network: YourWiFi
WiFi connected successfully!
IP Address: 192.168.1.100
KNX Gateway configured: 192.168.1.50
✓ Connected to KNX gateway!
Sending test: bool=true to 1/2/3
✓ Command sent successfully
Listening for KNX bus events...
```

---

## Troubleshooting

### ❌ WiFi won't connect
- Check SSID spelling (case-sensitive)
- Verify it's 2.4 GHz network
- Try moving Pico closer to router

### ❌ KNX connection fails
- Ping gateway: `ping 192.168.1.50`
- Check port 3671 open
- Verify same network/VLAN

### ❌ No output visible
- Check USB cable supports data (not charge-only)
- Try different USB port
- Check RTT connection with probe-rs

---

## Test Commands

Once running, trigger KNX events and watch the log:

```
💡 Switch 1/2/3: ON          # Someone turned on a light
🌡️  Temperature 5/1/10: 21.5°C # Temperature sensor update
💡 Lux 1/2/4: 500.0 lx       # Light level sensor
```

---

## Next Steps

- ✅ See `HARDWARE_TESTING.md` for comprehensive testing
- ✅ Check `OPTIMIZATION_REPORT.md` for performance details
- ✅ Read `examples/macros_demo.md` for API examples

---

## Quick Reference

| Action | Command |
|--------|---------|
| Build | `cargo build --release --target thumbv8m.main-none-eabihf --features embassy-rp` |
| Flash with probe | `probe-rs run --chip RP2350 <binary>` |
| Monitor RTT | `probe-rs attach --chip RP2350` |
| Create UF2 | `elf2uf2-rs <binary> output.uf2` |
| Serial monitor | `screen /dev/tty.usbmodem* 115200` |

---

**Ready to go! 🚀**

For detailed information, see `HARDWARE_TESTING.md`.

# KNX-RS Hardware Testing Guide

**Target Hardware:** Raspberry Pi Pico 2 W
**Firmware Size:** 339 KB
**Status:** Ready for deployment ✅

---

## 📋 Pre-Flight Checklist

### Hardware Requirements
- [ ] Raspberry Pi Pico 2 W (RP2040 with WiFi)
- [ ] USB-C cable for power and flashing
- [ ] KNX gateway with IP interface (e.g., ABB, Siemens, Gira)
- [ ] WiFi router with 2.4 GHz support
- [ ] (Optional) Logic analyzer or oscilloscope for debugging

### Network Setup
- [ ] WiFi network name (SSID) known
- [ ] WiFi password available
- [ ] KNX gateway IP address known
- [ ] Both devices on same network/VLAN
- [ ] Gateway port 3671 accessible

### Software Requirements
- [ ] `probe-rs` installed for flashing
- [ ] `cargo-embed` configured (alternative)
- [ ] Serial monitor ready (e.g., `screen`, `minicom`)

---

## 🔧 Step 1: Configure Your Environment

### Edit Configuration File

Open `src/configuration.rs` and update:

```rust
pub const CONFIG: &str = r#"
WIFI_NETWORK=YourWiFiName        # Replace with your WiFi SSID
WIFI_PASSWORD=YourWiFiPassword    # Replace with your WiFi password
KNX_GATEWAY_IP=192.168.1.10       # Replace with your KNX gateway IP
"#;
```

**Example:**
```rust
pub const CONFIG: &str = r#"
WIFI_NETWORK=MyHomeWiFi
WIFI_PASSWORD=SecurePassword123!
KNX_GATEWAY_IP=192.168.1.50
"#;
```

### Verify Configuration

```bash
# Check your configuration
grep -E "WIFI_|KNX_" src/configuration.rs

# Should show your actual values (not YOUR_WIFI_SSID)
```

---

## 🚀 Step 2: Build Firmware

### Option A: Using cargo-flash (Recommended)

```bash
# Build and prepare for flashing
cargo build --release --target thumbv8m.main-none-eabihf --features embassy-rp

# Binary location
ls -lh target/thumbv8m.main-none-eabihf/release/knx-rs
# Should show: ~339 KB
```

### Option B: Using Custom Cargo Alias

```bash
# If you have cargo alias configured
cargo build-rp2040-usb-release

# Or for RTT logging
cargo build --release --target thumbv8m.main-none-eabihf --features embassy-rp
```

---

## 📥 Step 3: Flash to Pico 2 W

### Method 1: probe-rs (Recommended for Development)

```bash
# Install probe-rs if not already installed
cargo install probe-rs-tools

# Flash the firmware
probe-rs run --chip RP2350 \
  target/thumbv8m.main-none-eabihf/release/knx-rs
```

### Method 2: UF2 Boot Mode (Easiest)

```bash
# Convert ELF to UF2 format
elf2uf2-rs target/thumbv8m.main-none-eabihf/release/knx-rs \
  knx-rs.uf2

# 1. Hold BOOTSEL button on Pico
# 2. Connect USB cable
# 3. Release BOOTSEL
# 4. Pico appears as USB drive
# 5. Copy knx-rs.uf2 to the drive
# 6. Pico reboots automatically
```

### Method 3: cargo-embed

```bash
# Using cargo-embed configuration
cargo embed --release --target thumbv8m.main-none-eabihf
```

---

## 📡 Step 4: Monitor Boot Sequence

### Using RTT (Real-Time Transfer)

```bash
# Start RTT monitor
probe-rs attach --chip RP2350

# Or using cargo-embed with RTT
cargo embed --release --target thumbv8m.main-none-eabihf
```

### Expected Boot Log

```
Random seed: 12345678901234
Connecting to WiFi network: YourWiFiName
WiFi connected successfully!
Waiting for DHCP...
IP Address: 192.168.1.100
Gateway: Some(192.168.1.1)
KNX-RS initialized and network ready!
KNX Gateway configured: 192.168.1.50
Connecting to KNX gateway at 192.168.1.50:3671
Attempting to connect...
✓ Connected to KNX gateway!
Sending test: bool=true to 1/2/3
✓ Command sent successfully
Sending test: bool=false to 1/2/3
✓ Command sent successfully
Listening for KNX bus events...
```

### ⚠️ Common Boot Issues

| Issue | Symptoms | Solution |
|-------|----------|----------|
| WiFi not connecting | Retrying message loop | Check SSID/password, 2.4 GHz only |
| No IP address | Stuck at "Waiting for DHCP" | Check router DHCP, network access |
| KNX connection failed | "Failed to connect" | Check gateway IP, port 3671 open |
| Immediate reboot | Panic message shown | Check panic-persist log |

---

## 🧪 Step 5: Functional Tests

### Test 1: Basic Connectivity ✅

**Expected:**
- WiFi connects within 10 seconds
- DHCP assigns IP
- KNX gateway connection succeeds

**Pass Criteria:**
```
✓ WiFi connected successfully!
✓ IP Address: 192.168.1.x
✓ Connected to KNX gateway!
```

### Test 2: Send Commands ✅

The firmware automatically sends test commands to group address `1/2/3`:

```
Sending test: bool=true to 1/2/3
✓ Command sent successfully
```

**Verify:**
- Check KNX bus monitor (if available)
- Check target device (if connected to 1/2/3)
- Look for telegrams on bus

### Test 3: Receive Events ✅

Trigger events from KNX bus (use ETS or physical switches):

**Expected Output:**
```
💡 Switch 1/2/3: ON
💡 Switch 1/2/3: OFF
🌡️  Temperature 1/2/10: 21.5°C
💡 Lux 1/2/11: 500.0 lx
```

**Actions:**
1. Press a physical switch connected to KNX
2. Send telegram from ETS
3. Verify output appears in RTT log

### Test 4: Continuous Operation ✅

Let run for extended period:

**Pass Criteria:**
- [ ] Runs stable for 5 minutes
- [ ] Responds to bus events
- [ ] No memory leaks (stable operation)
- [ ] WiFi stays connected
- [ ] No unexpected reboots

---

## 📊 Performance Measurements

### Latency Test

Measure time from command to bus:

```rust
// In your test code
let start = Instant::now();
client.write(ga!(1/2/3), KnxValue::Bool(true)).await?;
let latency = start.elapsed();
// Target: < 50ms
```

### Throughput Test

Send multiple commands rapidly:

```rust
for i in 0..100 {
    client.write(ga!(1/2/i), KnxValue::Bool(true)).await?;
}
// Target: > 20 commands/second
```

### Memory Usage

Monitor with probe-rs:
```bash
# Check stack usage
probe-rs stack-size

# Expected: < 64 KB total usage
```

---

## 🐛 Troubleshooting

### Problem: WiFi Connection Fails

**Check:**
```bash
# 1. Verify SSID is correct (case-sensitive!)
# 2. Ensure password is correct
# 3. Confirm 2.4 GHz WiFi (Pico doesn't support 5 GHz)
# 4. Check router MAC filter
# 5. Try moving Pico closer to router
```

**Debug:**
```rust
// Add in main.rs for more details
info!("Trying SSID: {}", wifi_ssid);
info!("Password length: {}", wifi_password.len());
```

### Problem: KNX Gateway Unreachable

**Check:**
```bash
# From your PC, ping the gateway
ping 192.168.1.50

# Try connecting with other tools
# ETS, KNX Virtual, etc.

# Verify port 3671 is open
nc -zv 192.168.1.50 3671
```

**Debug:**
```rust
// Add more logging in knx_client
info!("Attempting connection to {}:{}...", ip, port);
```

### Problem: Random Reboots

**Check:**
```bash
# View panic-persist log
# After reboot, check RTT output for panic message

# Common causes:
# - Stack overflow (task stack too small)
# - Assertion failure
# - Hardware watchdog
```

**Fix:**
```rust
// Increase task stack if needed
#[embassy_executor::task(pool_size = 1, stack_size = 8192)]
```

### Problem: No Bus Events Received

**Check:**
1. Verify group address matches actual KNX devices
2. Check if gateway is in programming mode
3. Confirm telegrams are being sent (use ETS monitor)
4. Verify network routing (areas/lines)

---

## 📈 Success Metrics

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| Boot time | < 15s | Time to "Listening for events" |
| WiFi connect | < 10s | Time to "WiFi connected" |
| KNX connect | < 5s | Time to "Connected to gateway" |
| Command latency | < 50ms | Timestamp difference |
| Packet loss | < 0.1% | Errors / total commands |
| Uptime | > 24h | Continuous operation |
| Memory stable | No growth | Monitor over time |

---

## 📝 Test Report Template

Create `HARDWARE_TEST_RESULTS.md`:

```markdown
# Hardware Test Results

**Date:** 2025-01-16
**Tester:** Your Name
**Hardware:** Raspberry Pi Pico 2 W
**Firmware:** knx-rs v0.1.0-alpha (commit: xxxxxx)

## Environment
- WiFi: [Network Name]
- KNX Gateway: [Brand/Model]
- Gateway IP: [x.x.x.x]

## Test Results

### Boot Sequence
- [ ] WiFi connected: [Time]
- [ ] DHCP IP assigned: [IP]
- [ ] KNX connected: [Time]
- [ ] Total boot time: [Seconds]

### Functional Tests
- [ ] Send command successful
- [ ] Receive events working
- [ ] 5-min stability test passed
- [ ] No crashes or reboots

### Performance
- Command latency: [X ms]
- Events/second: [X]
- Packet loss: [X%]

### Issues Found
1. [Description]
2. [Description]

### Notes
[Any observations or comments]
```

---

## 🎯 Next Steps After Successful Test

1. **Long-term stability test** (24h+)
2. **Stress testing** (rapid commands)
3. **Error recovery** (disconnect/reconnect gateway)
4. **Power cycle testing** (cold boot)
5. **Real-world deployment** with actual KNX devices

---

## 🆘 Need Help?

1. Check panic-persist logs after reboot
2. Enable verbose logging in code
3. Use logic analyzer on SPI bus
4. Check KNX bus monitor in ETS
5. Review OPTIMIZATION_REPORT.md for performance analysis

---

**Good luck with your hardware test! 🚀**

*For issues or questions, check the troubleshooting section or create an issue on GitHub.*

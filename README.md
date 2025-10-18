# knx-rs

KNXnet/IP protocol implementation for embedded systems using Rust.

## Features

- `no_std` compatible
- Zero-copy parsing
- Async/await with Embassy
- Type-safe addressing (Individual and Group addresses)
- KNXnet/IP tunneling support
- Common Datapoint Types (DPT)

## Status

✅ **Phase 1-4 Complete**
- ✅ Phase 1: Core Protocol (Frame, CEMI, Services)
- ✅ Phase 2: Datapoint Types (DPT 1, 3, 5, 7, 9, 13)
- ✅ Phase 3: Tunneling Client with Typestate Pattern
- ✅ Phase 4: Embassy + RP2040 Integration (Pico 2 W)

🚀 **Ready for hardware testing!**

## Understanding KNX Layers

KNX communication uses three nested layers, like Russian dolls 📦:

### The Big Picture

```text
┌─────────────────────────────────────────────────┐
│  FRAME (outer envelope - IP transport)          │
│  ┌───────────────────────────────────────────┐  │
│  │ From: 192.168.1.100:3671                  │  │
│  │ To: 192.168.1.10:3671                     │  │
│  │ Type: TUNNELING_REQUEST                   │  │
│  │                                           │  │
│  │  ┌─────────────────────────────────────┐ │  │
│  │  │ CEMI (KNX command)                  │ │  │
│  │  │                                     │ │  │
│  │  │ From: 1.1.250 (your device)        │ │  │
│  │  │ To: 1/2/3 (light group)            │ │  │
│  │  │ Command: GroupValue_Write          │ │  │
│  │  │                                     │ │  │
│  │  │  ┌───────────────────────────────┐ │ │  │
│  │  │  │ DPT (actual value)            │ │ │  │
│  │  │  │                               │ │ │  │
│  │  │  │ Type: DPT 1.001 (Switch)     │ │ │  │
│  │  │  │ Value: ON (true)             │ │ │  │
│  │  │  │ Bytes: [0x01]                │ │ │  │
│  │  │  └───────────────────────────────┘ │ │  │
│  │  └─────────────────────────────────────┘ │  │
│  └───────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

### Layer by Layer

#### 1. FRAME (KNXnet/IP Frame) 🌐
**Purpose:** Transport data over IP network (WiFi/Ethernet)

```text
┌──────────────────────────────────────┐
│ Header Length: 6                     │
│ Protocol Version: 1.0                │
│ Service Type: TUNNELING_REQUEST      │
│ Total Length: 23                     │
│ ────────────────────────────────     │
│ Body: [... CEMI data ...]           │
└──────────────────────────────────────┘
```

**Like:** The address on a postal envelope
- "Where from?" → Source IP
- "Where to?" → Destination IP
- "What kind of message?" → Service type

#### 2. CEMI (Common EMI) 📨
**Purpose:** Describes the actual KNX command

```text
┌──────────────────────────────────────┐
│ Message Code: L_Data.req             │ ← "I want to send"
│ Source: 1.1.250                      │ ← "From me (Pico)"
│ Destination: 1/2/3                   │ ← "To the lights"
│ NPDU Length: 1                       │ ← "1 byte of data"
│ TPCI/APCI: GroupValue_Write          │ ← "Write value"
│ ────────────────────────────────     │
│ Data: [0x01]                         │ ← DPT value
└──────────────────────────────────────┘
```

**Like:** The letter content
- "Who are you?" → Source address (1.1.250)
- "Who are you talking to?" → Destination (1/2/3)
- "What do you want to do?" → Write/Read/Response
- "What data?" → The DPT bytes

#### 3. DPT (Datapoint Type) 💡
**Purpose:** Encodes the actual value (ON/OFF, temperature, percentage...)

```text
DPT 1.001 (Switch):
  true → [0x01]
  false → [0x00]

DPT 3.007 (Dimming):
  Increase by 4 intervals → [0x0B]  // Control=1, Stepcode=3
  Stop → [0x00]                     // Break

DPT 5.001 (Percentage):
  75% → [0xBF]  // 75 * 255 / 100 = 191 = 0xBF

DPT 9.001 (Temperature):
  21.5°C → [0x0C, 0x1A]  // 16-bit compressed float
```

**Like:** The language you write in
- DPT = "data format"
- Same value, different encoding
- Receiver must know which DPT to use!

### Complete Example: Turn On a Light

#### You write (API):
```rust
client.write_bool(GroupAddress::new(1, 2, 3)?, true).await?;
```

#### What happens internally:

**1. DPT encoding** (innermost)
```rust
// DPT 1.001 (Switch)
let dpt_bytes = [0x01];  // true → ON
```

**2. CEMI construction** (middle)
```rust
CemiFrame {
    message_code: L_Data_req,
    source: IndividualAddress::new(1, 1, 250),
    destination: GroupAddress::new(1, 2, 3),
    apci: GroupValue_Write,
    data: [0x01],  // ← From DPT
}
// Serializes to bytes...
```

**3. KNXnet/IP Frame** (outermost)
```rust
KnxnetIpFrame {
    header: {
        service_type: TUNNELING_REQUEST,
        total_length: 23,
    },
    body: [... CEMI bytes ...],  // ← From CEMI
}
// Serializes to UDP packet...
```

**4. UDP Send**
```rust
socket.send_to(&frame_bytes, gateway_addr).await?;
```

### Receiving: Reverse Process

When you receive a packet:

```rust
// 1. Receive UDP
let bytes = socket.recv().await?;

// 2. Parse FRAME
let frame = KnxnetIpFrame::parse(&bytes)?;

// 3. Parse CEMI
let cemi = CemiFrame::parse(frame.body())?;

// 4. Decode DPT
let value = Dpt1::Switch.decode(cemi.data())?;  // true

println!("Light {} turned on!", cemi.destination());
```

### Quick Summary

| Layer | Purpose | Example |
|-------|---------|---------|
| **DPT** | Encoded value | `true` → `[0x01]` |
| **CEMI** | KNX command | "From 1.1.250 to 1/2/3: write \[0x01\]" |
| **FRAME** | IP transport | "UDP to 192.168.1.10:3671" |

**Data Flow:**
```text
Value (21.5°C)
  → DPT encoding → [0x0C, 0x1A]
  → CEMI → "Write to 2/1/5: \[0x0C, 0x1A\]"
  → FRAME → "UDP packet with CEMI inside"
  → WiFi → KNX Gateway
  → KNX Bus → Thermostat
```

## KNXnet/IP vs Tunneling

### What is KNXnet/IP?
**KNXnet/IP** is the general protocol for carrying KNX messages over IP networks (Ethernet/WiFi). Think of it as the "postal system" that defines:
- How to package messages (frame format)
- How to address packets (IP:port)
- Which services to offer (tunneling, routing, device management)

### What is Tunneling?
**Tunneling** is one specific service offered by KNXnet/IP for communicating with the KNX bus. It creates a point-to-point "tunnel" between your client and the KNX gateway.

```text
You (Pico 2W) ←──────────→ KNX Gateway ←──────→ KNX Bus
               WiFi/IP        Tunneling         Twisted Pair
              Connection
```

### KNXnet/IP Services

| Service | Purpose | Use Case |
|---------|---------|----------|
| **Tunneling** 🚇 | 1:1 connection with ACK | Control devices, bidirectional, reliable |
| **Routing** 🔀 | Multicast broadcast | Monitoring, multiple listeners |
| **Device Management** 🔧 | Configure devices | ETS tools, programming |
| **Remote Logging** 📝 | Receive logs | Debugging |

### Why Tunneling?

For embedded control (Pico 2 W → KNX), **Tunneling is the right choice**:

| Aspect | Tunneling ✅ | Routing |
|--------|-------------|---------|
| Reliability | High (ACK) | Low (no ACK) |
| Bidirectional | Yes | Yes |
| Connection | 1:1 dedicated | Multicast |
| Embedded | Ideal | Possible |

### Tunneling Protocol Flow

**1. Connection Setup**
```text
Client                           Gateway
  │                                 │
  ├─ CONNECT_REQUEST ──────────────→│
  │                                 │
  │←────────── CONNECT_RESPONSE ────┤
  │  (channel ID assigned)          │
```

**2. Data Exchange**
```text
Client                           Gateway                    KNX Bus
  │                                 │                          │
  ├─ TUNNELING_REQUEST ────────────→│                          │
  │  (send command)                 ├─ (forward to bus) ──────→│
  │                                 │                          │
  │←──────── TUNNELING_ACK ─────────┤                          │
  │  (acknowledged)                 │                          │
  │                                 │                          │
  │←──── TUNNELING_INDICATION ──────┤←─ (bus event) ───────────┤
  │  (receive event)                │                          │
  │                                 │                          │
  ├─ TUNNELING_ACK ─────────────────→│                          │
```

**3. Keep-Alive**
```text
Client                           Gateway
  │                                 │
  ├─ CONNECTIONSTATE_REQUEST ──────→│
  │  (every 60 seconds)             │
  │                                 │
  │←──── CONNECTIONSTATE_RESPONSE ──┤
  │  (connection OK)                │
```

**4. Disconnection**
```text
Client                           Gateway
  │                                 │
  ├─ DISCONNECT_REQUEST ────────────→│
  │                                 │
  │←────── DISCONNECT_RESPONSE ─────┤
```

### Where Do the Layers Fit?

```text
┌─────────────────────────────────────────────┐
│ KNXnet/IP FRAME                             │ ← General protocol
│ ┌─────────────────────────────────────────┐ │
│ │ Service Type: TUNNELING_REQUEST         │ │ ← Specific service
│ │                                         │ │
│ │ ┌─────────────────────────────────────┐ │ │
│ │ │ CEMI: GroupValue_Write              │ │ │ ← KNX command
│ │ │                                     │ │ │
│ │ │ ┌─────────────────────────────────┐ │ │ │
│ │ │ │ DPT 1.001: ON                   │ │ │ │ ← Value
│ │ │ └─────────────────────────────────┘ │ │ │
│ │ └─────────────────────────────────────┘ │ │
│ └─────────────────────────────────────────┘ │
└─────────────────────────────────────────────┘
```

- **KNXnet/IP** = FRAME layer (the envelope)
- **Tunneling** = Service type within the FRAME
- **CEMI** = KNX command inside the FRAME
- **DPT** = Encoded value inside CEMI

## Quick Start (Raspberry Pi Pico 2 W)

```rust
use knx_rs::protocol::async_tunnel::AsyncTunnelClient;
use knx_rs::addressing::GroupAddress;

// Connect to KNX gateway
let mut client = AsyncTunnelClient::new(
    &stack,
    rx_meta, tx_meta, rx_buffer, tx_buffer,
    [192, 168, 1, 10],  // Gateway IP
    3671,               // Gateway port
);
client.connect().await?;

// Send GroupValue_Write (turn on light at 1/2/3)
let light_addr = GroupAddress::from(0x0A03); // 1/2/3
let cemi_frame = build_group_write_bool(light_addr, true);
client.send_cemi(&cemi_frame).await?;

// Receive events from KNX bus
if let Some(cemi_data) = client.receive().await? {
    // Parse and handle KNX events
}
```

See [`examples/pico_knx_async.rs`](examples/pico_knx_async.rs) for complete working example with `WiFi` setup.

### Build for Pico 2 W

**Option 1: With defmt-rtt (default - requires probe-rs):**
```bash
cargo build-rp2040
# Flash with probe-rs
probe-rs run --chip RP2350 target/thumbv8m.main-none-eabihf/debug/knx-rs
```

**Option 2: With USB logger (no probe needed):**
```bash
cargo build-rp2040-usb --release
# Flash with picotool
picotool load -u -v -x -t elf target/thumbv8m.main-none-eabihf/release/knx-rs
# View logs with serial terminal (e.g., screen, minicom)
screen /dev/ttyACM0 115200
```

### Logger Options

This project supports two logging backends:

| Logger | Feature Flag | Use Case | Requires |
|--------|--------------|----------|----------|
| **defmt-rtt** | `embassy-rp` (default) | Development with probe-rs | Debug probe (probe-rs) |
| **USB logger** | `embassy-rp-usb` | Production, no probe needed | USB serial terminal |

**Commands:**
```bash
# defmt-rtt (default)
cargo build-rp2040          # Debug build
cargo flash-rp2040          # Release build

# USB logger
cargo build-rp2040-usb      # Debug build
cargo flash-rp2040-usb      # Release build
```

**Note:** Both loggers can coexist in the codebase, but only one is active at compile time based on the feature flag.

## Architecture

```text
knx-rs/
├── addressing/          # KNX addressing system
│   ├── individual.rs    # Individual addresses (area.line.device)
│   └── group.rs         # Group addresses (main/middle/sub)
├── protocol/            # KNXnet/IP protocol layer
│   ├── frame.rs         # Layer 1: KNXnet/IP frames
│   ├── cemi.rs          # Layer 2: CEMI messages
│   ├── services.rs      # Tunneling service builders
│   ├── tunnel.rs        # Typestate tunneling client
│   └── async_tunnel.rs  # Async wrapper for Embassy
├── dpt/                 # Layer 3: Datapoint types
│   ├── dpt1.rs          # Boolean (Switch, Binary)
│   ├── dpt3.rs          # 3-bit controlled (Dimming, Blinds)
│   ├── dpt5.rs          # 8-bit unsigned (Percentage, Angle)
│   ├── dpt7.rs          # 16-bit unsigned (Counter, Time)
│   ├── dpt9.rs          # 16-bit float (Temperature, Humidity)
│   └── dpt13.rs         # 32-bit signed (Counter)
├── examples/            # Practical examples
│   └── pico_knx_async.rs # Complete Pico 2 W example
├── error.rs             # Error types
└── lib.rs               # Public API
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

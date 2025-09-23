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

🚧 Work in progress - Phase 1 (Core Protocol) completed

## Understanding KNX Layers

KNX communication uses three nested layers, like Russian dolls 📦:

### The Big Picture

```
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

```
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

```
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

```rust
DPT 1.001 (Switch):
  true → [0x01]
  false → [0x00]

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
| **CEMI** | KNX command | "From 1.1.250 to 1/2/3: write [0x01]" |
| **FRAME** | IP transport | "UDP to 192.168.1.10:3671" |

**Data Flow:**
```
Value (21.5°C)
  → DPT encoding → [0x0C, 0x1A]
  → CEMI → "Write to 2/1/5: [0x0C, 0x1A]"
  → FRAME → "UDP packet with CEMI inside"
  → WiFi → KNX Gateway
  → KNX Bus → Thermostat
```

## Architecture

```
knx-rs/
├── addressing/     # KNX addressing system
├── protocol/       # KNXnet/IP protocol layer
│   ├── frame.rs    # Layer 1: KNXnet/IP frames
│   └── cemi.rs     # Layer 2: CEMI messages
├── dpt/            # Layer 3: Datapoint types
├── error.rs        # Error types
└── lib.rs          # Public API
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

# KNX-RS Development Status & Roadmap

**Last Updated:** 2025-10-16
**Current Version:** 0.1.0-dev (unreleased)

---

## 📊 Executive Summary

KNX-RS is a no_std Rust implementation of the KNXnet/IP protocol, currently functional on RP2040 (Raspberry Pi Pico 2 W) with Embassy async runtime. The core protocol and basic DPT support are complete and working, but several improvements are needed before publishing as a general-purpose crate for the Rust community.

**Status:** ✅ Functional prototype → 🎯 Needs refinement for public release

---

## ✅ Current State (What We Have)

### Phase 1-4: Core Implementation ✅

#### **KNXnet/IP Protocol - Complete**
- ✅ Frame parsing/encoding (HEADER + HPAI + CRI/CRD)
- ✅ Tunneling connection lifecycle
  - CONNECT_REQUEST/RESPONSE
  - DISCONNECT_REQUEST/RESPONSE
  - TUNNELING_REQUEST/ACK
- ✅ Automatic heartbeat (CONNECTIONSTATE_REQUEST)
- ✅ Sequence counter management
- ✅ cEMI frame parsing (L_Data.req/ind/con)

#### **Datapoint Types (DPT) Support**
| DPT Family | Subtypes | Status | Use Cases |
|------------|----------|--------|-----------|
| **DPT 1.xxx** | Boolean | ✅ Complete | Switches, sensors, enable/disable |
| **DPT 5.xxx** | 8-bit unsigned | ✅ Complete | Percentage, angle, counters |
| **DPT 7.xxx** | 16-bit unsigned | ✅ Complete | Pulses, brightness, color temp |
| **DPT 9.xxx** | 2-byte float | ✅ Complete | Temperature, lux, humidity, pressure |
| **DPT 13.xxx** | 32-bit signed | ✅ Complete | Energy counters, long values |

**Total Supported:** 5 DPT families covering most common use cases.

#### **Embedded Platform Support**
- ✅ **no_std** compatible
- ✅ **Embassy** async runtime integration
- ✅ **RP2040** (Raspberry Pi Pico 2 W)
  - WiFi stack (CYW43 driver)
  - Network stack (embassy-net)
  - Dual logger system (USB CDC / defmt-rtt)
- ✅ Working example running on hardware

#### **High-Level API (KnxClient)**
```rust
pub struct KnxClient {
    // Methods:
    pub async fn connect(&mut self) -> Result<(), ()>
    pub async fn write(&mut self, addr: GroupAddress, value: KnxValue) -> Result<(), ()>
    pub async fn read(&mut self, addr: GroupAddress) -> Result<(), ()>
    pub async fn respond(&mut self, addr: GroupAddress, value: KnxValue) -> Result<(), ()>
    pub async fn receive_event(&mut self) -> Result<Option<KnxEvent>, ()>
}

pub enum KnxValue {
    Bool(bool),
    Percent(u8),
    U8(u8),
    U16(u16),
    Temperature(f32),
    Lux(f32),
    Humidity(f32),
    Ppm(f32),
    Float2(f32),
}

pub enum KnxEvent {
    GroupWrite { address, value },
    GroupRead { address },
    GroupResponse { address, value },
    Unknown { address, data_len },
}
```

#### **Testing Infrastructure**
- ✅ Python-based KNX simulator (tests without physical hardware)
- ✅ Integration tests with simulator
- ✅ Comprehensive unit tests for DPT encoding/decoding
- ✅ TESTING.md guide

#### **Documentation**
- ✅ README.md with overview and examples
- ✅ QUICKSTART.md (5-minute setup guide)
- ✅ TESTING.md (testing infrastructure)
- ✅ Idiomatic Rust documentation (RFC 1574 compliant)
  - Module-level docs with examples
  - Function docs with `# Arguments`, `# Returns`, `# Errors`
  - Type documentation with use cases
- ✅ Working code examples

---

## ❌ What's Missing for Public Crate Release

### 1. **Portability and Flexibility** ⚠️

#### Current State
- ✅ Works on RP2040 (Pico 2 W)
- ❌ Heavily dependent on Embassy-specific features
- ❌ Not tested on other microcontrollers
- ❌ Cannot be used with std (desktop/server applications)

#### What's Needed
```rust
// Support for std (desktop/server applications)
#[cfg(feature = "std")]
impl KnxClient {
    pub fn new_std(gateway: SocketAddr) -> Self { ... }
}

// Transport abstraction trait
pub trait KnxTransport {
    async fn send(&mut self, data: &[u8]) -> Result<()>;
    async fn recv(&mut self, buf: &mut [u8]) -> Result<usize>;
}

// Support for other async runtimes
#[cfg(feature = "tokio")]
impl KnxClient { ... }
```

**Priority:** 🔴 High (limits adoption significantly)

---

### 2. **API Ergonomics** ⚠️

#### Current Issues
- ❌ Configuration too verbose (manual buffer management)
  ```rust
  // Current: users must manage buffers manually
  let rx_meta = RX_META.init([PacketMetadata::EMPTY; 4]);
  let tx_meta = TX_META.init([PacketMetadata::EMPTY; 4]);
  let rx_buffer = RX_BUFFER.init([0u8; 2048]);
  let tx_buffer = TX_BUFFER.init([0u8; 2048]);
  let client = KnxClient::new(stack, rx_meta, tx_meta, rx_buffer, tx_buffer, ip, port);
  ```
- ❌ Error handling too generic (`Result<(), ()>`)
- ❌ Missing builder pattern for configuration
- ⚠️ Automatic decode cannot distinguish between DPT variants

#### Proposed Improvements
```rust
// Builder pattern
let client = KnxClient::builder()
    .gateway("192.168.1.10:3671")
    .device_address("1.1.1")
    .connect_timeout(Duration::from_secs(5))
    .build()?;

// Better error handling
pub enum KnxClientError {
    NotConnected,
    SendFailed(KnxError),
    ReceiveFailed(KnxError),
    Timeout,
    InvalidAddress,
}

// Type registry for DPT
client.register_dpt("1/2/3", DptType::Switch)?;
let value: bool = client.read_typed("1/2/3").await?;

// Macro for group address literals
let addr = ga!("1/2/3"); // Compile-time validated
```

**Priority:** 🔴 High (user experience critical)

---

### 3. **Incomplete DPT Support** ⚠️

#### Current Coverage
| DPT | Description | Status | Priority |
|-----|-------------|--------|----------|
| 1.xxx | Boolean | ✅ Complete | - |
| 2.xxx | 1-bit controlled | ❌ Missing | 🟡 Medium |
| **3.xxx** | **3-bit controlled (dimming, blinds)** | ❌ Missing | 🔴 **High** |
| 4.xxx | Character | ❌ Missing | 🟢 Low |
| 5.xxx | 8-bit unsigned | ✅ Complete | - |
| 6.xxx | 8-bit signed | ❌ Missing | 🟡 Medium |
| 7.xxx | 16-bit unsigned | ✅ Complete | - |
| 8.xxx | 16-bit signed | ❌ Missing | 🟡 Medium |
| 9.xxx | 2-byte float | ✅ Complete | - |
| **10.xxx** | **Time** | ❌ Missing | 🔴 **High** |
| **11.xxx** | **Date** | ❌ Missing | 🔴 **High** |
| 12.xxx | 32-bit unsigned | ❌ Missing | 🟡 Medium |
| 13.xxx | 32-bit signed | ✅ Complete | - |
| 14.xxx | 4-byte float | ❌ Missing | 🟡 Medium |
| **16.xxx** | **String (ASCII)** | ❌ Missing | 🔴 **High** |
| 18.xxx | Scene control | ❌ Missing | 🟡 Medium |
| **19.xxx** | **Date/Time combined** | ❌ Missing | 🔴 **High** |

#### High Priority DPTs (Most Requested)
1. **DPT 3.xxx** - Dimming and blind control (extremely common in home automation)
2. **DPT 10/11** - Time/Date (scheduling and automation)
3. **DPT 16.xxx** - String (display text on panels)
4. **DPT 19.xxx** - Combined Date/Time

**Priority:** 🟡 Medium-High (functional but incomplete)

---

### 4. **Missing Protocol Features** ❌

#### Current Limitations
- ❌ **KNX Routing** (multicast mode, not just tunneling)
- ❌ **Device discovery** (SEARCH_REQUEST/RESPONSE)
- ❌ **Device description** (DESCRIPTION_REQUEST/RESPONSE)
- ❌ **Proactive connection state monitoring**
- ❌ **Automatic reconnection** on disconnect
- ❌ **Multiple connections** (multiple gateways simultaneously)
- ❌ **Group address filtering/monitoring**

#### Impact
Many advanced use cases require routing or device discovery. Current implementation only supports basic tunneling with manual gateway configuration.

**Priority:** 🟡 Medium (nice-to-have but not blocking for basic use)

---

### 5. **Configuration and Deployment** ⚠️

#### Current Problem
```rust
// src/configuration.rs - HARDCODED AT COMPILE TIME!
pub const CONFIG: &str = r#"
WIFI_NETWORK=YOUR_WIFI_SSID
WIFI_PASSWORD=YOUR_WIFI_PASSWORD
KNX_GATEWAY_IP=192.168.1.10
"#;
```

#### What's Needed
- ❌ External configuration file support
- ❌ Environment variables support (when std is available)
- ❌ Runtime configuration (not compile-time)
- ❌ TOML/JSON configuration files
- ❌ Secure credential storage
- ❌ Configuration validation

#### Proposed Solution
```rust
// Runtime configuration
let config = KnxConfig::from_file("config.toml")?;

// Environment variables (std only)
#[cfg(feature = "std")]
let config = KnxConfig::from_env()?;

// Builder pattern
let config = KnxConfig::builder()
    .gateway("192.168.1.10:3671")
    .wifi("MySSID", "password")
    .validate()?;
```

**Priority:** 🔴 High (security and deployment concern)

---

### 6. **Testing and CI/CD** ⚠️

#### Current State
- ✅ Unit tests for DPT encoding/decoding
- ✅ Python KNX simulator
- ⚠️ Limited integration tests
- ❌ No CI/CD pipeline
- ❌ No test coverage tracking
- ❌ No performance benchmarks
- ❌ No fuzzing for parsing

#### What's Needed
```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    - Clippy lints (all features)
    - Format check (rustfmt)
    - Test on stable/beta/nightly
    - Test with --no-default-features
    - Cross-compilation check (RP2040, ESP32)
    - Documentation build verification

  coverage:
    - Generate coverage report (tarpaulin/llvm-cov)
    - Upload to codecov.io
    - Fail if coverage < 80%

  bench:
    - Run benchmarks
    - Compare with baseline
    - Track performance regressions
```

**Priority:** 🔴 High (essential for open source project)

---

### 7. **Packaging and Publishing** ❌

#### Critical Missing Items
- ❌ Not published on crates.io
- ❌ No semantic versioning defined
- ❌ No CHANGELOG.md
- ❌ No CONTRIBUTING.md guidelines
- ❌ No CODE_OF_CONDUCT.md
- ❌ License not prominently displayed
- ❌ No structured examples/ directory
- ⚠️ README could be more "marketing-friendly"

#### Pre-Publishing Checklist
```markdown
- [ ] Choose semantic version (suggest: 0.1.0-alpha.1)
- [ ] Add license badge to README
- [ ] Create CHANGELOG.md (keep-a-changelog format)
- [ ] Create CONTRIBUTING.md
- [ ] Add CODE_OF_CONDUCT.md (Contributor Covenant)
- [ ] Organize examples/ directory
- [ ] Add crates.io metadata to Cargo.toml
  - [ ] description
  - [ ] repository
  - [ ] homepage
  - [ ] documentation
  - [ ] keywords (max 5)
  - [ ] categories (max 5)
- [ ] Run `cargo publish --dry-run`
- [ ] Set up docs.rs configuration
```

**Priority:** 🔴 High (blocking publication)

---

### 8. **Examples and Tutorials** ⚠️

#### Current State
- ✅ `examples/test_with_simulator.rs` - Integration test
- ✅ `examples/pico_knx_async.rs` - Full embedded example

#### What's Missing
```
examples/
  ├── README.md                    # Examples overview
  ├── basic/
  │   ├── 01_connect.rs           # Minimal connection example
  │   ├── 02_write.rs             # Write a value
  │   ├── 03_read.rs              # Read request
  │   └── 04_monitor.rs           # Monitor bus events
  ├── devices/
  │   ├── temperature_sensor.rs   # Read temperature sensor
  │   ├── light_control.rs        # Control lights with dimming
  │   ├── blind_control.rs        # Control blinds (DPT 3)
  │   └── scene_controller.rs     # Scene management
  ├── advanced/
  │   ├── multi_gateway.rs        # Multiple gateways
  │   ├── router.rs               # KNX routing
  │   └── error_handling.rs       # Robust error handling
  └── platforms/
      ├── rp2040_pico.rs          # Raspberry Pi Pico
      ├── esp32.rs                # ESP32 support
      └── desktop_monitor.rs      # Desktop monitoring (std)
```

**Priority:** 🟡 Medium (improves adoption)

---

### 9. **Ecosystem Integration** ❌

#### Useful Integrations
- ❌ **serde** support for configuration serialization
- ❌ **tracing** instead of log/defmt for unified logging
- ❌ **tokio** support for std applications
- ❌ Home Assistant integration example
- ❌ MQTT bridge example
- ❌ REST API example
- ❌ WebSocket API example

#### Example: Home Assistant Integration
```rust
// examples/integrations/homeassistant.rs
// Bridge KNX events to MQTT for Home Assistant auto-discovery
```

**Priority:** 🟢 Low (nice-to-have for ecosystem)

---

### 10. **Performance and Resource Usage** ❓

#### Metrics to Measure and Optimize
- ❓ Memory footprint (RAM usage)
- ❓ Maximum throughput (telegrams/second)
- ❓ Average latency (request → response)
- ❓ Stack usage per async task
- ❓ Binary size for embedded targets
- ❓ CPU usage percentage

#### Target Benchmarks (TBD)
```rust
// benches/throughput.rs
#[bench]
fn bench_write_throughput(b: &mut Bencher) {
    // Target: >100 telegrams/second
}

#[bench]
fn bench_parse_latency(b: &mut Bencher) {
    // Target: <1ms per frame parse
}
```

**Priority:** 🟢 Low (optimize after adoption)

---

## 🚀 Proposed Roadmap

### **Milestone 1: API Stability** (2-3 weeks)
**Goal:** Stable, idiomatic, ergonomic API

#### Tasks
1. ✅ Implement proper error types (replace `Result<(), ()>`)
2. ✅ Builder pattern for KnxClient configuration
3. ✅ Remove unsafe buffer management from public API
4. ✅ Add `ga!()` macro for group address literals
5. ✅ Implement type-safe DPT registry
6. ✅ Runtime configuration (remove hardcoded values)

**Deliverable:** API that feels natural to Rust developers

---

### **Milestone 2: Protocol Completeness** (2-3 weeks)
**Goal:** Essential protocol features

#### Tasks
1. ✅ Device discovery (SEARCH_REQUEST/RESPONSE)
2. ✅ Automatic reconnection on disconnect
3. ✅ Connection state monitoring with heartbeat
4. ✅ Add DPT 3, 10, 11, 16 (high priority)
5. ✅ Group address filtering/routing

**Deliverable:** Production-ready protocol implementation

---

### **Milestone 3: Portability** (2-3 weeks)
**Goal:** Works everywhere Rust works

#### Tasks
1. ✅ Add `std` feature for desktop/server
2. ✅ Add `tokio` support
3. ✅ Transport trait abstraction
4. ✅ Test on ESP32 (popular embedded platform)
5. ✅ Abstract away Embassy dependencies

**Deliverable:** Multi-platform support (embedded + std)

---

### **Milestone 4: Publishing** (1-2 weeks)
**Goal:** Ready for crates.io

#### Tasks
1. ✅ CI/CD setup (GitHub Actions)
2. ✅ Test coverage >80%
3. ✅ Complete example suite
4. ✅ CHANGELOG.md + CONTRIBUTING.md
5. ✅ Semantic versioning (0.1.0-alpha.1)
6. ✅ Publish to crates.io

**Deliverable:** `cargo add knx-rs` works!

---

### **Milestone 5: Ecosystem** (ongoing)
**Goal:** Integrations and community growth

#### Tasks
1. ✅ Home Assistant integration example
2. ✅ MQTT bridge example
3. ✅ Web dashboard example (wasm?)
4. ✅ Blog post / tutorial series
5. ✅ Community support (Discord/Matrix?)

**Deliverable:** Active community and integrations

---

## 💡 Immediate Next Steps (Quick Wins)

### **Week 1: API Improvements**
These changes will have the biggest impact on usability:

1. **Better Error Handling**
   ```rust
   // Replace Result<(), ()> everywhere
   pub enum KnxClientError {
       NotConnected,
       ConnectionFailed(KnxError),
       SendFailed(KnxError),
       ReceiveFailed(KnxError),
       Timeout,
       InvalidAddress(String),
       UnsupportedDpt,
   }

   impl std::error::Error for KnxClientError {}
   impl fmt::Display for KnxClientError {}
   ```

2. **Builder Pattern**
   ```rust
   let client = KnxClient::builder()
       .gateway("192.168.1.10:3671")
       .device_address("1.1.1")
       .buffers(BufferConfig::default())
       .build()?;
   ```

3. **Runtime Configuration**
   ```rust
   // Remove hardcoded CONFIG constant
   // Allow runtime configuration via builder or config struct
   ```

4. **Improve README**
   - Add badges (build status, crates.io, docs.rs)
   - Clear feature comparison table
   - Better quick start example
   - Architecture diagram

5. **Add DPT 3.xxx** (Dimming/Blind Control)
   - Extremely common in home automation
   - Relatively simple to implement
   - High impact on usability

**Estimated Time:** 5-7 days
**Impact:** 🔴 High - Makes the crate immediately more usable

---

### **Week 2: Publishing Preparation**

1. **CI/CD Setup**
   ```yaml
   # .github/workflows/ci.yml
   - Clippy (all features, all targets)
   - Tests (simulator + unit tests)
   - Format check
   - Doc build
   - Cross-compile check for RP2040
   ```

2. **More Examples**
   - `examples/basic_write.rs` - Minimal example
   - `examples/temperature_monitor.rs` - Realistic use case
   - `examples/light_dimmer.rs` - Uses DPT 3

3. **Documentation Polish**
   - CHANGELOG.md (v0.1.0-alpha.1)
   - CONTRIBUTING.md
   - Update README with realistic examples

4. **Semantic Versioning**
   - Choose version: `0.1.0-alpha.1`
   - Tag in git
   - Update Cargo.toml

5. **Dry Run Publish**
   ```bash
   cargo publish --dry-run
   ```

**Estimated Time:** 5-7 days
**Impact:** 🔴 High - Enables publication

---

### **Week 3: Alpha Release**

1. **Final Testing**
   - Run all tests on real hardware
   - Test with simulator
   - Verify examples compile and work

2. **Publish to crates.io**
   ```bash
   cargo publish
   ```

3. **Announcement**
   - Post on r/rust
   - Post on Rust users forum
   - Tweet/Mastodon
   - KNX community forums

4. **Monitor Feedback**
   - GitHub issues
   - Community questions
   - Bug reports

**Estimated Time:** 2-3 days
**Impact:** 🔴 High - Project goes public!

---

## 🎯 Long-term Vision

### **Become the Reference Rust Implementation for KNX**

#### Goals
1. **Platform Coverage**
   - RP2040 ✅
   - ESP32 🎯
   - STM32 🎯
   - Desktop/Server (std) 🎯

2. **Complete DPT Support**
   - All commonly used DPTs (1-20)
   - Community-driven additions for rare DPTs

3. **Protocol Completeness**
   - Tunneling ✅
   - Routing 🎯
   - KNXnet/IP Secure 🎯
   - USB backend 🎯

4. **Ecosystem Integration**
   - Home Assistant
   - OpenHAB
   - Node-RED
   - MQTT brokers

5. **Community Growth**
   - >1000 downloads/month on crates.io
   - >10 contributors
   - Active community support

---

## 🤔 Key Decisions Needed

### 1. **Target Audience Priority**
- [ ] Embedded-first (focus on no_std, then add std)
- [ ] Dual-target (embedded + std equally important)
- [ ] Std-first (focus on desktop/server, embedded later)

**Recommendation:** Start embedded-first (current strength), add std in Milestone 3.

---

### 2. **Publishing Timeline**
- [ ] Publish alpha ASAP (get early feedback)
- [ ] Wait for API stability (avoid breaking changes)
- [ ] Wait for multiple platforms (broader appeal)

**Recommendation:** Publish alpha after Milestone 1 (stable API), iterate based on feedback.

---

### 3. **DPT Priority**
Which DPTs should be added first?

**High Priority (Week 1-2):**
- [ ] DPT 3.xxx - Dimming/blind control
- [ ] DPT 10.xxx - Time
- [ ] DPT 11.xxx - Date

**Medium Priority (Week 3-4):**
- [ ] DPT 16.xxx - String
- [ ] DPT 19.xxx - Date/Time combined
- [ ] DPT 14.xxx - 4-byte float

**Low Priority (Community-driven):**
- [ ] DPT 2, 4, 6, 8, 12, 18, etc.

---

### 4. **Feature Scope for 0.1.0**
What should be included in first release?

**Minimum Viable Product:**
- [x] Tunneling protocol (working)
- [ ] Proper error handling
- [ ] Builder pattern API
- [ ] 5-8 DPT families
- [ ] RP2040 support
- [ ] Basic examples
- [ ] CI/CD

**Nice-to-Have (can wait for 0.2.0):**
- Routing support
- Device discovery
- Multiple platforms
- std support

---

## 📞 Next Actions

1. **Review this document** - Does it align with your vision?
2. **Make key decisions** - Answer the questions above
3. **Prioritize tasks** - Which quick wins should we tackle first?
4. **Set timeline** - How much time can you dedicate per week?

---

## 📚 Resources

### Current Documentation
- `README.md` - Project overview
- `QUICKSTART.md` - 5-minute setup
- `TESTING.md` - Testing guide
- `ROADMAP.md` - Original roadmap (may be outdated)

### External References
- [KNX Specifications](https://www.knx.org/knx-en/for-professionals/get-started/knx-standard/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Semantic Versioning](https://semver.org/)
- [Keep a Changelog](https://keepachangelog.com/)

---

**Document Version:** 1.0
**Compiled by:** Claude (AI Assistant)
**Reviewed by:** Cristiano Chieppa
**Next Review:** After Milestone 1 completion

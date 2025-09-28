# KNX-RS Development Roadmap

Roadmap completa per l'implementazione della libreria KNXnet/IP per Raspberry Pi Pico 2 W.

## 🎯 Obiettivo Finale

Libreria `no_std` completa per controllare dispositivi KNX da un microcontrollore RP2040, con:
- Supporto WiFi (Pico 2 W)
- KNXnet/IP tunneling
- API async con Embassy
- Type-safe e zero-copy parsing
- Ottimizzata per embedded

---

## ✅ Fase 1: Core Protocol (COMPLETATA)

### Obiettivi
Implementare il layer base del protocollo KNXnet/IP e CEMI.

### Completato
- ✅ **Addressing System** (`src/addressing/`)
  - `IndividualAddress` - Indirizzi dispositivi (area.line.device)
  - `GroupAddress` - Indirizzi gruppo (main/middle/sub o main/sub)
  - Parsing da stringhe e bytes
  - Validazione ranges
  - Serializzazione

- ✅ **KNXnet/IP Frame Parsing** (`src/protocol/frame.rs`)
  - Header parsing (6 bytes)
  - Service type identification
  - Body extraction
  - Validazione lunghezze

- ✅ **CEMI Layer** (`src/protocol/cemi.rs`)
  - Message codes (L_Data.req, L_Data.ind, etc.)
  - Control fields parsing
  - Source/destination address extraction
  - APDU extraction
  - Support per standard e extended frames

- ✅ **Infrastructure**
  - Error types (`src/error.rs`)
  - Constants (`src/protocol/constants.rs`)
  - Testing setup (`no_std` + `std` per test)
  - Build system per RP2040

### File Coinvolti
```
src/
├── addressing/
│   ├── mod.rs
│   ├── individual.rs
│   └── group.rs
├── protocol/
│   ├── mod.rs
│   ├── frame.rs
│   ├── cemi.rs
│   └── constants.rs
├── error.rs
└── lib.rs
```

### Test Coverage
- 28+ test cases
- Parsing valido/invalido
- Edge cases (indirizzi limite, frame malformati)

---

## ✅ Fase 2: Datapoint Types (DPT) - COMPLETATA

### Obiettivi
Implementare encoding/decoding dei tipi di dato KNX più comuni.

### Completato
- ✅ **DPT Infrastructure**
  - `src/dpt/mod.rs` - Module base
  - Traits `DptEncode` e `DptDecode` per encode/decode
  - Error handling per conversioni

- ✅ **DPT 1.xxx - Boolean**
  - `DPT 1.001` - Switch (on/off)
  - `DPT 1.002` - Bool (true/false)
  - `DPT 1.003` - Enable (enable/disable)
  - `DPT 1.008` - Up/Down
  - `DPT 1.009` - Open/Close
  - 1 bit encoding

- ✅ **DPT 5.xxx - 8-bit Unsigned**
  - `DPT 5.001` - Percentage (0-100%)
  - `DPT 5.003` - Angle (0-360°)
  - `DPT 5.004` - Percentage 0-255
  - `DPT 5.010` - Counter pulses (0-255)
  - 1 byte encoding

- ✅ **DPT 9.xxx - 2-byte Float**
  - `DPT 9.001` - Temperature (°C)
  - `DPT 9.004` - Illuminance (lux)
  - `DPT 9.005` - Wind speed (m/s)
  - `DPT 9.006` - Pressure (Pa)
  - 2 byte float16 encoding

- ✅ **DPT 7.xxx - 2-byte Unsigned**
  - `DPT 7.001` - Pulses (0-65535)
  - `DPT 7.013` - Brightness (lux)

- ✅ **DPT 13.xxx - 4-byte Signed**
  - `DPT 13.001` - Counter pulses (signed)
  - `DPT 13.010` - Active energy (Wh)

- ✅ **Tests**
  - Encoding/decoding round-trip
  - Range validation
  - Edge cases (min/max values)
  - Float precision

### Struttura File
```
src/
└── dpt/
    ├── mod.rs          # Trait Dpt + re-exports
    ├── dpt1.rs         # Boolean types
    ├── dpt5.rs         # 8-bit unsigned
    ├── dpt7.rs         # 16-bit unsigned
    ├── dpt9.rs         # 2-byte float
    └── dpt13.rs        # 4-byte signed
```

### API Esempio
```rust
use knx_rs::dpt::{Dpt1, Dpt5, Dpt9};

// Boolean
let data = Dpt1::Switch.encode(true)?;  // [0x01]
let value = Dpt1::Switch.decode(&data)?; // true

// Percentage
let data = Dpt5::Percentage.encode(75)?;  // [0xBF]
let value = Dpt5::Percentage.decode(&data)?; // 75

// Temperature
let data = Dpt9::Temperature.encode(21.5)?;  // [0x0C, 0x1A]
let temp = Dpt9::Temperature.decode(&data)?; // 21.5
```

---

## ✅ Fase 3: KNXnet/IP Tunneling Client - COMPLETATA

### Obiettivi
Implementare il client per tunneling KNXnet/IP (connessione, invio/ricezione, heartbeat).

### Completato
- ✅ **Connection Management**
  - CONNECT_REQUEST/RESPONSE
  - Channel ID assignment
  - Connection timeout handling

- ✅ **Tunneling**
  - TUNNELING_REQUEST (invio comandi KNX)
  - TUNNELING_ACK (acknowledge)
  - TUNNELING_INDICATION (ricezione eventi)
  - Sequence counter management

- ✅ **Heartbeat**
  - CONNECTIONSTATE_REQUEST/RESPONSE
  - Keep-alive timer
  - Reconnection logic

- ✅ **Disconnect**
  - DISCONNECT_REQUEST/RESPONSE
  - Graceful shutdown
  - Resource cleanup

- ✅ **State Machine (Typestate Pattern)**
  - Idle → Connecting → Connected → Disconnecting
  - Compile-time state safety
  - Type-safe transitions

### Struttura File
```
src/
└── protocol/
    ├── tunnel.rs       # Tunneling client
    ├── connection.rs   # Connection state machine
    └── services.rs     # Service request/response builders
```

### API Esempio
```rust
let mut client = TunnelClient::new(gateway_addr);
client.connect().await?;
client.send_frame(cemi_frame).await?;
let response = client.receive().await?;
client.disconnect().await?;
```

---

## ✅ Fase 4: Integrazione Embassy + RP2040 - COMPLETATA

### Obiettivi
Integrare il client KNX con WiFi su Raspberry Pi Pico 2 W usando Embassy.

### Completato
- ✅ **WiFi Driver Setup**
  - cyw43 driver per Pico 2 W
  - WiFi connection management con retry logic
  - DHCP client integrato

- ✅ **UDP Stack**
  - embassy-net UDP sockets
  - Async send/receive
  - Timeout handling

- ✅ **AsyncTunnelClient**
  - Wrapper async per TunnelClient
  - Integrazione con embassy-net
  - Supporto heartbeat (send_heartbeat() ogni 60s)
  - Buffer management ottimizzato

- ✅ **Example Binary**
  - `examples/pico_knx_async.rs` - Esempio completo con WiFi + KNX
  - Lampada on/off via GroupValue_Write
  - Ricezione eventi dal bus KNX
  - Logging con defmt
  - Documentazione completa in `examples/README.md`

### Dipendenze da Aggiungere
```toml
embassy-rp = { version = "0.2", features = ["time-driver"] }
embassy-net = "0.4"
cyw43 = "0.2"
cyw43-pio = "0.2"
embassy-time = "0.3"
```

### Hardware Requirements
- Raspberry Pi Pico 2 W
- Gateway KNXnet/IP (es. ABB, Siemens)
- Rete WiFi

---

## 📋 Fase 5: API di Alto Livello

### Obiettivi
Creare API user-friendly per operazioni comuni.

### Da Fare
- [ ] **KnxClient High-Level API**
  ```rust
  client.write_bool(addr, true).await?;
  client.write_percentage(addr, 75).await?;
  client.write_temperature(addr, 21.5).await?;

  let value = client.read_bool(addr).await?;
  let temp = client.read_temperature(addr).await?;
  ```

- [ ] **Device Abstractions**
  ```rust
  let light = Light::new(client, "1/2/3")?;
  light.on().await?;
  light.off().await?;
  light.set_brightness(50).await?;

  let sensor = TempSensor::new(client, "2/1/5")?;
  let temp = sensor.read().await?;
  ```

- [ ] **Builder Pattern**
  ```rust
  let client = KnxClient::builder()
      .gateway("192.168.1.10:3671")
      .individual_address("1.1.250")
      .timeout(Duration::from_secs(5))
      .build()?;
  ```

- [ ] **Event Listeners**
  ```rust
  client.on_group_write("1/2/3", |value| {
      defmt::info!("Received: {}", value);
  });
  ```

### Struttura File
```
src/
├── client.rs           # High-level KnxClient
├── devices/
│   ├── light.rs
│   ├── sensor.rs
│   └── switch.rs
└── builder.rs          # Client builder
```

---

## 📋 Fase 6: Testing & Optimization

### Obiettivi
Test completo su hardware e ottimizzazione performance.

### Da Fare
- [ ] **Hardware Testing**
  - Test con gateway KNX reale
  - Stress test (molti messaggi)
  - Latency measurements
  - Reliability testing

- [ ] **Performance Optimization**
  - Memory profiling
  - Stack usage analysis
  - Ottimizzazione allocazioni
  - Reduce binary size

- [ ] **Documentation**
  - API documentation completa
  - Examples per ogni caso d'uso
  - Troubleshooting guide
  - Hardware setup guide

- [ ] **CI/CD**
  - GitHub Actions per test automatici
  - Build verification per RP2040
  - Coverage report
  - Release automation

### Tools
- `cargo-bloat` - Analisi dimensioni binary
- `cargo-call-stack` - Stack usage
- `defmt` - Logging embedded
- Logic analyzer - Debug protocollo

---

## 📊 Milestone

### M1: Protocol Complete (Fase 1-2) ✅ COMPLETATO
- ✅ Parsing completo KNXnet/IP
- ✅ DPT comuni implementati (1, 5, 7, 9, 13)
- ✅ 144 test passing
- **Completato:** Gennaio 2025

### M2: Client Functional (Fase 3-4) ✅ COMPLETATO
- ✅ Client funzionante (AsyncTunnelClient)
- ✅ Supporto Pico 2 W con WiFi
- ✅ Invio/ricezione comandi base
- ✅ Esempio completo funzionante
- **Completato:** Gennaio 2025

### M3: Production Ready (Fase 5-6) 🚧 IN CORSO
- [ ] API di alto livello (opzionale)
- [ ] Testing su hardware completo (necessario)
- [ ] Performance ottimizzate
- **Target:** Da definire

---

## 📝 Note Tecniche

### Constraints Embedded
- **RAM:** ~264 KB (RP2040)
- **Flash:** ~2 MB (Pico 2 W)
- **Stack:** ~16-32 KB per task
- **No heap allocations** (solo stack)

### Performance Targets
- **Latency:** < 50ms per comando
- **Throughput:** > 100 msg/sec
- **Memory:** < 64 KB totali

### KNX Specs Reference
- KNX Standard v2.1
- KNXnet/IP Core v1.0
- KNXnet/IP Tunneling v1.0
- DPT specs da KNX Association

---

## 🔗 Riferimenti

- [KNX Association](https://www.knx.org/)
- [Embassy Framework](https://embassy.dev/)
- [RP2040 Datasheet](https://datasheets.raspberrypi.com/rp2040/rp2040-datasheet.pdf)
- [Pico W Datasheet](https://datasheets.raspberrypi.com/picow/pico-w-datasheet.pdf)

---

## 📅 Changelog

### 2025-01-15
- ✅ Fase 4 completata (AsyncTunnelClient + Pico 2 W integration)
- ✅ Heartbeat/keep-alive support aggiunto
- ✅ Esempio completo `pico_knx_async.rs` con documentazione
- 📝 ROADMAP aggiornata con stato reale del progetto
- 🚀 **Pronto per testing su hardware**

### 2025-01-14
- ✅ Fase 1 completata (addressing, protocol, CEMI)
- ✅ Fase 2 completata (DPT 1, 5, 7, 9, 13)
- ✅ Fase 3 completata (TunnelClient con typestate pattern)
- 📝 Roadmap creata

---

**Ultimo aggiornamento:** 2025-01-15
**Versione:** 0.1.0-alpha
**Status:** Fasi 1-4 complete, pronto per hardware testing

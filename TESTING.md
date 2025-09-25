# Testing Guide for knx-rs

## Testing Without Physical KNX Hardware

This guide explains how to test `knx-rs` on macOS using a virtual KNX gateway.

### Prerequisites for macOS

1. **Install Docker Desktop**:
   ```bash
   brew install --cask docker
   # Or download from https://www.docker.com/products/docker-desktop
   ```

2. **Install Wireshark** (optional, for packet inspection):
   ```bash
   brew install --cask wireshark
   ```

### Quick Start with Virtual Gateway

We provide a simple Python-based KNXnet/IP gateway simulator (`knx_simulator.py`) that's easy to run on macOS without Docker.

#### 1. Start KNX Virtual Gateway

```bash
cd /path/to/knx-rs

# Start simulator (verbose mode to see traffic)
python3 knx_simulator.py --verbose

# Or run in background
python3 knx_simulator.py --verbose &
```

**Gateway Configuration**:
- IP Address: `127.0.0.1` (localhost)
- Port: `3671` (standard KNXnet/IP)
- Gateway Address: `1.1.250`
- Client Address Range: `1.1.128` - `1.1.135` (8 clients)

#### 2. Test the Simulator

```bash
# Quick test with provided script
python3 test_simulator.py

# Should output:
# ✅ SUCCESS! Simulator is working correctly!
```

#### 3. Simulator Commands

```bash
# Start in foreground (see all traffic)
python3 knx_simulator.py --verbose

# Start in background
python3 knx_simulator.py &

# Stop background simulator
pkill -f knx_simulator

# Check if running
pgrep -f knx_simulator
```

### What You CAN Test

✅ Protocol correctness (frame format, headers)
✅ Connection/disconnection flow
✅ Sequence counter management
✅ Heartbeat/keep-alive
✅ Error handling (timeout, invalid response)
✅ DPT encoding/decoding
✅ cEMI frame parsing
✅ Multiple concurrent clients

### What You CANNOT Test (without real hardware)

❌ Physical device responses (real lights turning on)
❌ Bus timing issues
❌ TP-UART communication
❌ Real-world interference/noise

---

## Come Funziona il Testing in `no_std`

### Configurazione Magica

In `src/lib.rs` abbiamo:

```rust
#![cfg_attr(not(test), no_std)]
```

**Spiegazione:**
- `cfg_attr`: attributo condizionale
- `not(test)`: "quando NON stiamo testando"
- `no_std`: usa no_std

**Traduzione:**
```
SE (stiamo testando):
    usa std (per il test framework)
ALTRIMENTI:
    usa no_std (per embedded)
```

### Workflow di Test

```
┌─────────────────────────────────────────┐
│  cargo test --lib                       │
├─────────────────────────────────────────┤
│  1. Compila per il tuo computer         │
│  2. Usa std (perché --test è attivo)   │
│  3. Esegue #[test] functions            │
│  4. Mostra risultati                    │
└─────────────────────────────────────────┘

┌─────────────────────────────────────────┐
│  cargo build --target thumbv8m...       │
├─────────────────────────────────────────┤
│  1. Compila per RP2040                  │
│  2. Usa no_std (--test NON attivo)     │
│  3. Binario per microcontrollore        │
└─────────────────────────────────────────┘
```

## Comandi per Testare

### ⚡ Comandi Rapidi (Alias Custom)

Abbiamo configurato degli alias in `.cargo/config.toml` per semplificare i comandi:

```bash
# Test su host (il più comune)
cargo test-host

# Test ottimizzati (release)
cargo test-host-release

# Build per RP2040
cargo build-rp2040

# Check per RP2040 (più veloce)
cargo check-rp2040

# Build ottimizzato per flash su RP2040
cargo flash-rp2040
```

### 1️⃣ Test Base (più comune)

```bash
# Nuovo modo (più semplice!)
cargo test --lib

# Oppure con alias
cargo test-host
```

**Output:**
```
running 28 tests
test addressing::group::tests::test_new_3level_valid ... ok
test addressing::individual::tests::test_from_str ... ok
test protocol::frame::tests::test_header_parse ... ok
...
test result: ok. 28 passed; 0 failed
```

**Cosa fa:**
- Compila per il tuo Mac/PC (aarch64-apple-darwin o x86_64)
- Usa `std` (grazie a `cfg_attr`)
- Esegue tutti i test con `#[test]`

### 2️⃣ Test in Release (ottimizzazioni)

```bash
cargo test --lib --release
```

**Perché:**
- Verifica che le ottimizzazioni non rompano nulla
- Testa gli `unsafe` block con ottimizzazioni attive
- Simula performance reali

### 3️⃣ Verifica Compilazione Embedded

```bash
cargo check --lib --target thumbv8m.main-none-eabihf
```

**Cosa fa:**
- NON esegue i test
- Compila in modalità `no_std`
- Verifica che il codice embedded sia valido

### 4️⃣ Test Specifico

```bash
# Test solo un modulo
cargo test --lib addressing::

# Test solo una funzione
cargo test --lib test_header_parse
```

## Struttura dei Test

### Dove Sono i Test?

I test sono **inline** nei moduli:

```rust
// src/addressing/individual.rs

pub struct IndividualAddress { ... }

impl IndividualAddress { ... }

#[cfg(test)]  // ← Compila solo durante i test
mod tests {
    use super::*;  // Importa IndividualAddress

    #[test]
    fn test_new_valid() {
        let addr = IndividualAddress::new(1, 2, 3).unwrap();
        assert_eq!(addr.area(), 1);
    }
}
```

**Vantaggi:**
- Test vicini al codice
- Accesso a funzioni private
- Organizzazione chiara

## Problemi Comuni

### ❌ Problema: "can't find crate for `test`"

```
error[E0463]: can't find crate for `test`
```

**Causa:** Stai compilando per embedded senza `#![cfg_attr(not(test), no_std)]`

**Soluzione:** Assicurati che `src/lib.rs` inizi con:
```rust
#![cfg_attr(not(test), no_std)]
```

### ❌ Problema: "can't find macro `format`"

```
error: cannot find macro `format` in this scope
```

**Causa:** Test usa `std::format!` ma il codice è `no_std`

**Soluzione già applicata:**
```rust
#![cfg_attr(not(test), no_std)]  // ← Abilita std durante test
```

### ❌ Problema: Test lenti su Mac ARM

Se il test di default fallisce, specifica esplicitamente il target:

```bash
# Per Mac M1/M2/M3
cargo test --lib --target aarch64-apple-darwin

# Per Mac Intel
cargo test --lib --target x86_64-apple-darwin
```

## Workflow Completo di Sviluppo

### Durante lo Sviluppo

```bash
# 1. Scrivi codice e test
vim src/addressing/individual.rs

# 2. Esegui test velocemente
cargo test --lib test_new_valid

# 3. Tutti i test
cargo test --lib

# 4. Verifica embedded
cargo check --lib --target thumbv8m.main-none-eabihf
```

### Prima di Commit

```bash
# Script completo
./test_commands.sh

# Oppure manualmente:
cargo test --lib --release                                    # Test ottimizzati
cargo check --lib --target thumbv8m.main-none-eabihf          # Embedded check
cargo check --bin knx-rs --features embassy-rp \
  --target thumbv8m.main-none-eabihf                          # Binary check
```

## Target Disponibili

```bash
# Lista tutti i target installati
rustup target list --installed

# Installa target se mancante
rustup target add thumbv8m.main-none-eabihf
rustup target add aarch64-apple-darwin
```

## CI/CD (GitHub Actions esempio)

```yaml
# .github/workflows/test.yml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: thumbv8m.main-none-eabihf

      - name: Run tests
        run: cargo test --lib

      - name: Check embedded build
        run: cargo check --lib --target thumbv8m.main-none-eabihf
```

## Domande Frequenti

### Q: Perché non posso fare `cargo test --target thumbv8m.main-none-eabihf`?

**A:** Il target embedded non ha `std`, e il test framework richiede `std`. Per questo usiamo `cfg_attr` per abilitare `std` solo durante i test.

### Q: Come testo il codice unsafe?

**A:** Usa Miri (interpreter per unsafe):
```bash
cargo +nightly miri test --lib
```

### Q: Posso testare su hardware reale?

**A:** Sì! Usa `defmt-test`:
```toml
[dev-dependencies]
defmt-test = "0.3"
```

Ma per ora, i test su host sono più che sufficienti.

### Q: Come vedo l'output dei test?

```bash
# Mostra println! nei test
cargo test --lib -- --nocapture

# Mostra test anche se passano
cargo test --lib -- --show-output
```

## Performance dei Test

```bash
# Parallelo (default)
cargo test --lib

# Seriale (per debug)
cargo test --lib -- --test-threads=1

# Quiet mode
cargo test --lib --quiet
```

## Summary

**Per sviluppo normale:**
```bash
cargo test --lib
```

**Per verificare embedded:**
```bash
cargo check --lib --target thumbv8m.main-none-eabihf
```

**Per tutto insieme:**
```bash
./test_commands.sh
```

✅ Il sistema di test funziona perfettamente: compila con `std` su host per i test, ma con `no_std` per embedded!

# Quick Start - knx-rs

## 🎯 Setup Rapido (5 minuti)

### 1. Test il Simulatore Virtuale

```bash
# Avvia il gateway virtuale KNX
python3 knx_simulator.py --verbose &

# Testa che funzioni
python3 test_simulator.py
# Output atteso: ✅ SUCCESS! Simulator is working correctly!
```

### 2. Test della Libreria

```bash
# Esegui tutti i test (144 tests)
cargo test --lib

# Verifica build embedded
cargo check-rp2040
```

**Fatto!** Ora puoi sviluppare senza hardware KNX reale 🚀

---

## 🚀 Comandi Essenziali

### Test e Sviluppo

```bash
# ✅ Test della libreria (più usato)
cargo test --lib

# ✅ Test ottimizzati
cargo test-host-release

# ✅ Test singolo
cargo test --lib test_header_parse

# ✅ Test con output
cargo test --lib -- --nocapture
```

### Build per RP2040

```bash
# ✅ Check veloce (consigliato durante sviluppo)
cargo check-rp2040

# ✅ Build completo
cargo build-rp2040

# ✅ Build ottimizzato per flash
cargo flash-rp2040

# ✅ Flash su Pico 2W (con picotool)
cargo flash-rp2040 && picotool reboot
```

### Verifica Compilazione

```bash
# ✅ Tutto insieme
cargo test --lib && cargo check-rp2040
```

## 📖 Come Funziona

### Doppio Target System

```
┌─────────────────────────────────────┐
│ cargo test --lib                    │  ← Compila per HOST
│   └─> usa std (test framework)     │     (Mac/PC)
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│ cargo build-rp2040                  │  ← Compila per EMBEDDED
│   └─> usa no_std (embedded)        │     (RP2040)
└─────────────────────────────────────┘
```

### Magic Config in `src/lib.rs`

```rust
#![cfg_attr(not(test), no_std)]
//        ↑
//    Se NON stiamo testando → usa no_std
//    Se stiamo testando → usa std
```

## 🔧 Struttura Progetto

```
knx-rs/
├── src/
│   ├── lib.rs              ← Libreria (no_std)
│   ├── main.rs             ← Binary per RP2040
│   ├── addressing/         ← Indirizzamento KNX
│   ├── protocol/           ← Parser KNXnet/IP
│   └── error.rs            ← Error types
├── .cargo/config.toml      ← Alias comandi
├── Cargo.toml              ← Features e profili
└── TESTING.md              ← Guida completa
```

## 🎯 Workflow Tipico

### Durante Sviluppo

```bash
# 1. Modifica codice
vim src/protocol/frame.rs

# 2. Test rapidi
cargo test --lib

# 3. Check embedded
cargo check-rp2040

# 4. Se OK, commit
git add . && git commit -m "feat: ..."
```

### Prima di Push

```bash
# Test completi
cargo test-host-release

# Verifica embedded
cargo check-rp2040

# Build release
cargo flash-rp2040
```

## 📝 Aggiungere Nuovi Test

```rust
// In fondo al tuo modulo (es. src/protocol/frame.rs)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_new_feature() {
        let result = my_function();
        assert_eq!(result, expected_value);
    }
}
```

Poi esegui:
```bash
cargo test --lib test_my_new_feature
```

## 🐛 Troubleshooting

### "can't find crate for `std`"

**Problema:** Stai compilando per embedded target ma serve std

**Soluzione:**
```bash
# Assicurati di NON avere target di default in .cargo/config.toml
# O usa esplicitamente:
cargo test --lib  # ← Auto-seleziona host target
```

### "can't find crate for `test`"

**Problema:** Stesso del precedente

**Soluzione:** Rimuovi `target = "thumbv8m..."` da `.cargo/config.toml`

### Test troppo lenti

```bash
# Usa release mode
cargo test-host-release

# O parallelo
cargo test --lib --release -- --test-threads=4
```

## 💡 Tips

### Vedere Output dei Test

```bash
cargo test --lib -- --nocapture
```

### Test Solo un Modulo

```bash
cargo test --lib addressing::
cargo test --lib protocol::
```

### Watch Mode (auto-recompile)

```bash
# Installa cargo-watch
cargo install cargo-watch

# Auto-test on changes
cargo watch -x "test --lib"
```

### Benchmarking (manuale)

```rust
use embassy_time::Instant;

let start = Instant::now();
let frame = KnxnetIpFrame::parse(&data)?;
let elapsed = start.elapsed();
defmt::info!("Parse: {} µs", elapsed.as_micros());
```

## 🎓 Prossimi Passi

1. **Leggi `TESTING.md`** per la guida completa
2. **Leggi `PERFORMANCE.md`** per ottimizzazioni
3. **Esplora `examples/`** per esempi pratici

## ❓ FAQ

**Q: Devo specificare sempre il target?**
A: No! Ora `cargo test --lib` funziona automaticamente.

**Q: Come flasho su Pico 2W?**
A: `cargo flash-rp2040`, poi tieni premuto BOOTSEL e resetta.

**Q: I test girano su hardware?**
A: No, i test girano sul tuo computer. Per testare su hardware, usa `defmt` nel binary.

**Q: Posso testare async code?**
A: Sì, usa `#[tokio::test]` o `embassy_executor` test support.

## 🔗 Collegamenti

- [TESTING.md](TESTING.md) - Guida completa test
- [PERFORMANCE.md](PERFORMANCE.md) - Performance guide
- [README.md](README.md) - Overview progetto

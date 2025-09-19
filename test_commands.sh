#!/bin/bash
# Script per eseguire i test di knx-rs

echo "=== Test della libreria knx-rs ==="
echo ""

# 1. Test su host (con std)
echo "1️⃣  Test standard su host..."
cargo test --lib

# 2. Test in release (per verificare ottimizzazioni)
echo ""
echo "2️⃣  Test in release mode..."
cargo test --lib --release

# 3. Check che compili per embedded (no_std)
echo ""
echo "3️⃣  Verifica compilazione embedded (no_std)..."
cargo check --lib --target thumbv8m.main-none-eabihf

# 4. Check binary completo
echo ""
echo "4️⃣  Verifica binary RP2040..."
cargo check --bin knx-rs --features embassy-rp --target thumbv8m.main-none-eabihf

echo ""
echo "✅ Tutti i controlli completati!"

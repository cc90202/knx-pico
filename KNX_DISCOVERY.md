# KNX Gateway Discovery

## Panoramica

È stata implementata la funzionalità di **discovery automatica** del gateway KNX utilizzando il protocollo `SEARCH_REQUEST` / `SEARCH_RESPONSE`. Questo elimina la necessità di configurare manualmente l'indirizzo IP del gateway in `configuration.rs`.

## Come Funziona

1. All'avvio, il sistema invia un `SEARCH_REQUEST` all'indirizzo multicast KNX (224.0.23.12:3671)
2. Il gateway KNX risponde con un `SEARCH_RESPONSE` contenente il suo indirizzo IP e porta
3. Il sistema si connette automaticamente al gateway scoperto
4. Se nessun gateway viene trovato entro 3 secondi, il sistema usa l'IP statico da `configuration.rs` come fallback

## Architettura

### File Creati

- **`src/knx_discovery.rs`** - Modulo dedicato alla discovery KNX
  - `discover_gateway()` - Funzione principale per scoprire il gateway
  - `GatewayInfo` - Struct con IP e porta del gateway scoperto
  - `build_search_request()` - Costruisce il pacchetto SEARCH_REQUEST (formato corretto con header_len 0x06)
  - `parse_search_response()` - Parser per le risposte dei gateway

### Modifiche ai File Esistenti

- **`src/lib.rs`** - Aggiunto modulo `pub mod knx_discovery;`
- **`src/main.rs`** - Aggiunta logica di discovery con flag di attivazione

## Formato Pacchetto SEARCH_REQUEST

Il bug critico risolto era l'assenza del byte `header_len` (0x06) all'inizio del pacchetto:

```
FORMATO CORRETTO (14 bytes):
┌────────────────────────────────────┐
│ Header (6 bytes)                   │
├────────────────────────────────────┤
│ 0x06  - header_len     ✓ CRITICAL │
│ 0x10  - protocol_version           │
│ 0x02  - service_type (high)        │
│ 0x01  - service_type (low)         │
│ 0x00  - total_length (high)        │
│ 0x0e  - total_length (low) = 14    │
├────────────────────────────────────┤
│ HPAI (8 bytes)                     │
├────────────────────────────────────┤
│ 0x08  - structure_length           │
│ 0x01  - protocol_code (UDP)        │
│ [4]   - local IP address           │
│ [2]   - local port (big-endian)    │
└────────────────────────────────────┘

FORMATO ERRATO (mancava 0x06):
0x10 0x02 0x01 0x00 0x0e ...  ❌
```

## Utilizzo

### Attivare la Discovery (Default)

Nel file `src/main.rs`, la costante `USE_AUTO_DISCOVERY` è impostata su `true`:

```rust
const USE_AUTO_DISCOVERY: bool = true;
```

Con questa impostazione:
- ✓ Il sistema cerca automaticamente il gateway
- ✓ Se trova un gateway, lo usa
- ✓ Se non trova nulla, usa l'IP statico da configuration.rs

### Disattivare la Discovery (Revert)

Per tornare al comportamento precedente (solo configurazione statica):

```rust
const USE_AUTO_DISCOVERY: bool = false;
```

Con questa impostazione:
- Il sistema usa SOLO l'IP configurato in `configuration.rs`
- Non viene effettuata alcuna discovery
- Comportamento identico alla versione precedente

## Log di Debug

Quando la discovery è attiva, vedrai questi messaggi:

```
Starting KNX gateway discovery (SEARCH)...
✓ KNX Gateway discovered automatically!
  IP: 192.168.1.250
  Port: 3671
```

Se la discovery fallisce:

```
Starting KNX gateway discovery (SEARCH)...
✗ No gateway found via discovery, using static configuration
  Fallback to: 192.168.1.29
```

Se la discovery è disattivata:

```
KNX Gateway (static config): 192.168.1.29
```

## Testing

### Test Unitari

Il modulo include test per:
- Costruzione corretta del pacchetto SEARCH_REQUEST
- Parsing delle risposte SEARCH_RESPONSE
- Calcolo dell'indirizzo di broadcast

Esegui i test con:

```bash
cargo test --lib knx_discovery
```

### Test con Simulatore

1. Avvia il simulatore KNX:
   ```bash
   python3 knx_simulator.py
   ```

2. Compila e carica il firmware:
   ```bash
   cargo build --release
   ./flash.sh
   ```

3. Osserva i log per verificare la discovery

## Vantaggi

✅ **Zero configurazione** - Non serve più conoscere l'IP del gateway  
✅ **Resiliente** - Fallback automatico alla configurazione statica  
✅ **Standard KNXnet/IP** - Implementazione conforme alle specifiche  
✅ **Facile revert** - Basta cambiare una costante boolean  
✅ **Testato** - Include test unitari e validazione formato pacchetto  

## Troubleshooting

### Il gateway non viene trovato

1. Verifica che il gateway sia acceso e collegato alla rete
2. Controlla che multicast sia abilitato sulla rete
3. Aumenta il timeout da 3 a 5 secondi in `main.rs`:
   ```rust
   knx_discovery::discover_gateway(&stack, Duration::from_secs(5)).await
   ```
4. Usa la configurazione statica come fallback (già implementato)

### Voglio vedere i pacchetti raw

Aggiungi logging in `knx_discovery.rs`:
```rust
info!("Sending SEARCH_REQUEST: {:02X?}", &request_buf[..request_len]);
```

## Riferimenti

- Specifiche KNXnet/IP Core v1.0 - Sezione 3.8.1 (SEARCH_REQUEST)
- Specifiche KNXnet/IP Core v1.0 - Sezione 3.8.2 (SEARCH_RESPONSE)
- Repository knx-search (Python reference implementation)


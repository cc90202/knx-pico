# KNX Gateway Discovery

## Overview

**Automatic discovery** of KNX gateways has been implemented using the `SEARCH_REQUEST` / `SEARCH_RESPONSE` protocol. This eliminates the need to manually configure the gateway IP address.

## How It Works

1. At startup, the system sends a `SEARCH_REQUEST` to the KNX multicast address (224.0.23.12:3671)
2. The KNX gateway responds with a `SEARCH_RESPONSE` containing its IP address and port
3. The system automatically connects to the discovered gateway
4. If no gateway is found within 3 seconds, the system halts with an error message

## Architecture

### Files Created

- **`src/knx_discovery.rs`** - Dedicated KNX discovery module
  - `discover_gateway()` - Main function to discover the gateway
  - `GatewayInfo` - Struct containing discovered gateway IP and port
  - `build_search_request()` - Constructs SEARCH_REQUEST packet (correct format with header_len 0x06)
  - `parse_search_response()` - Parser for gateway responses

### Modified Files

- **`src/lib.rs`** - Added module `pub mod knx_discovery;`
- **`examples/pico_knx_async.rs`** - Uses discovery with error on failure
- **`examples/knx_sniffer.rs`** - Uses discovery with error on failure

## SEARCH_REQUEST Packet Format

The critical bug fixed was the missing `header_len` byte (0x06) at the start of the packet:

```
CORRECT FORMAT (14 bytes):
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

INCORRECT FORMAT (missing 0x06):
0x10 0x02 0x01 0x00 0x0e ...  ❌
```

## Usage

Discovery is **always enabled** in both examples. If no gateway is found, the system halts with a clear error message telling the user to:
- Ensure the KNX gateway or simulator is running
- Ensure it's connected to the same network
- Reset the device to retry

## Debug Logs

When discovery is active, you'll see these messages:

```
Discovering KNX gateway via multicast...
✓ KNX Gateway discovered automatically!
  IP: 192.168.1.250
  Port: 3671
```

If discovery fails:

```
Discovering KNX gateway via multicast...
✗ No KNX gateway found on network!
  Ensure your KNX gateway or simulator is running
  and connected to the same network.
System halted. Reset device to retry.
```

## Testing

### Unit Tests

The module includes tests for:
- Correct SEARCH_REQUEST packet construction
- SEARCH_RESPONSE parsing
- Broadcast address calculation

Run tests with:

```bash
cargo test --lib knx_discovery
```

### Testing with Simulator

1. Start the KNX simulator:
   ```bash
   python3 knx_simulator.py
   ```

2. Build and flash the firmware:
   ```bash
   cargo flash-example-usb
   ```

3. Observe the logs to verify discovery

## Advantages

✅ **Zero configuration** - No need to know the gateway IP
✅ **Standard KNXnet/IP** - Implementation compliant with specifications
✅ **Clear error messages** - Users know exactly what to do if discovery fails
✅ **Well tested** - Includes unit tests and packet format validation

## Troubleshooting

### Gateway not found

1. Verify the gateway is powered on and connected to the network
2. Check that multicast is enabled on the network
3. Increase the timeout from 3 to 5 seconds in the example:
   ```rust
   knx_discovery::discover_gateway(&stack, Duration::from_secs(5)).await
   ```
4. Ensure your WiFi network allows multicast traffic

### View raw packets

Add logging in `knx_discovery.rs`:
```rust
info!("Sending SEARCH_REQUEST: {:02X?}", &request_buf[..request_len]);
```

## References

- KNXnet/IP Core v1.0 Specifications - Section 3.8.1 (SEARCH_REQUEST)
- KNXnet/IP Core v1.0 Specifications - Section 3.8.2 (SEARCH_RESPONSE)
- knx-search repository (Python reference implementation)

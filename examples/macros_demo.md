# KNX-RS Macro Examples

This document demonstrates the convenience macros provided by knx-rs.

## Available Macros

### 1. `ga!` - Group Address Creation

Create group addresses with clean, readable syntax:

```rust
use knx_rs::ga;

// Instead of:
let addr = GroupAddress::from(0x0A03);

// Use:
let addr = ga!(1/2/3);

// Examples:
let living_room_temp = ga!(1/2/10);
let kitchen_light = ga!(1/2/4);
let bedroom_dimmer = ga!(2/1/5);
```

### 2. `register_dpts!` - Bulk DPT Registration

Register multiple DPT type mappings at once:

```rust
use knx_rs::{register_dpts, DptType};

// Instead of:
client.register_dpt(ga!(1/2/3), DptType::Temperature)?;
client.register_dpt(ga!(1/2/4), DptType::Bool)?;
client.register_dpt(ga!(1/2/5), DptType::Humidity)?;

// Use:
register_dpts! {
    client,
    1/2/3 => Temperature,
    1/2/4 => Bool,
    1/2/5 => Humidity,
    1/2/6 => Lux,
    2/1/10 => Bool,
    2/1/11 => Percent,
}?;
```

### 3. `knx_write!` - Write with Inline Address

Write values with address notation:

```rust
use knx_rs::{knx_write, KnxValue};

// Instead of:
client.write(ga!(1/2/3), KnxValue::Bool(true)).await?;

// Use:
knx_write!(client, 1/2/3, KnxValue::Bool(true)).await?;

// Examples:
knx_write!(client, 1/2/4, KnxValue::Temperature(21.5)).await?;
knx_write!(client, 2/1/5, KnxValue::Percent(75)).await?;
```

### 4. `knx_read!` - Read with Inline Address

Request values with address notation:

```rust
use knx_rs::knx_read;

// Instead of:
client.read(ga!(1/2/10)).await?;

// Use:
knx_read!(client, 1/2/10).await?;

// Wait for response
match client.receive_event().await? {
    Some(KnxEvent::GroupResponse { address, value }) => {
        info!("Received response: {:?}", value);
    }
    _ => {}
}
```

### 5. `knx_respond!` - Respond with Inline Address

Respond to read requests:

```rust
use knx_rs::{knx_respond, KnxValue, KnxEvent};

match client.receive_event().await? {
    Some(KnxEvent::GroupRead { address }) => {
        // Instead of:
        client.respond(address, KnxValue::Temperature(21.5)).await?;

        // Use:
        knx_respond!(client, 1/2/10, KnxValue::Temperature(21.5)).await?;
    }
    _ => {}
}
```

## Complete Example

Here's a complete example using all the macros:

```rust
use knx_rs::{ga, register_dpts, knx_write, knx_read, KnxValue, DptType};

// Connect to KNX gateway
let mut client = KnxClient::builder()
    .gateway([192, 168, 1, 10], 3671)
    .device_address([1, 1, 1])
    .build_with_buffers(&stack, &mut buffers)?;

client.connect().await?;

// Register DPT types for known addresses
register_dpts! {
    client,
    1/2/3 => Temperature,    // Living room temperature
    1/2/4 => Bool,            // Kitchen light
    1/2/5 => Humidity,        // Bathroom humidity
    1/2/6 => Lux,             // Garden light sensor
    2/1/10 => Percent,        // Bedroom dimmer
}?;

// Turn on kitchen light
knx_write!(client, 1/2/4, KnxValue::Bool(true)).await?;

// Set bedroom dimmer to 75%
knx_write!(client, 2/1/10, KnxValue::Percent(75)).await?;

// Request current temperature
knx_read!(client, 1/2/3).await?;

// Listen for events
loop {
    match client.receive_event().await? {
        Some(KnxEvent::GroupWrite { address, value }) => {
            // Values are automatically typed based on DPT registry
            match value {
                KnxValue::Temperature(t) => {
                    info!("Temperature: {:.1}°C", t);
                }
                KnxValue::Bool(on) => {
                    info!("Switch: {}", if on { "ON" } else { "OFF" });
                }
                _ => {}
            }
        }
        _ => {}
    }

    Timer::after(Duration::from_millis(100)).await;
}
```

## Benefits

Using macros provides several benefits:

1. **Readability**: Address notation `1/2/3` is more intuitive than hex `0x0A03`
2. **Compile-time validation**: Invalid addresses are caught at compile time
3. **Less boilerplate**: Reduce repetitive code
4. **Type safety**: All type checking is preserved
5. **No runtime overhead**: Macros expand at compile time

## Address Validation

The `ga!` macro validates addresses at compile time:

```rust
// ✅ Valid
let addr = ga!(31/7/255);  // Maximum valid values

// ❌ Compile error
let addr = ga!(32/0/0);    // Main group > 31

// ❌ Compile error
let addr = ga!(1/8/0);     // Middle group > 7
```

## Integration with Existing Code

Macros are fully compatible with the existing API:

```rust
// Mix and match as needed
let addr1 = ga!(1/2/3);
let addr2 = GroupAddress::from(0x0A04);

client.write(addr1, KnxValue::Bool(true)).await?;
knx_write!(client, 1/2/4, KnxValue::Bool(false)).await?;
```

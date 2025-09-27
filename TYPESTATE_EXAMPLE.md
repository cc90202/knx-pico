# Typestate Pattern for TunnelClient

This document shows how to refactor `TunnelClient` using the typestate pattern
for compile-time state validation.

## Current Implementation (Runtime Checks)

```rust
pub enum ConnectionState {
    Idle,
    Connecting,
    Connected,
    Disconnecting,
}

pub struct TunnelClient {
    state: ConnectionState,  // Runtime check needed
    channel_id: u8,
    // ...
}

impl TunnelClient {
    pub fn send_data(&mut self, data: &[u8]) -> Result<()> {
        if self.state != ConnectionState::Connected {
            return Err(KnxError::NotConnected);  // Runtime error
        }
        // ...
    }
}
```

## Typestate Pattern (Compile-time Checks)

### 1. Define States as Types

```rust
/// Client is idle (not connected)
pub struct Idle;

/// Connection request sent, waiting for response
pub struct Connecting {
    sequence: u8,
}

/// Connected and ready to send/receive
pub struct Connected {
    channel_id: u8,
    send_sequence: u8,
    recv_sequence: u8,
}

/// Disconnect request sent
pub struct Disconnecting;
```

### 2. Generic Client with State Parameter

```rust
pub struct TunnelClient<State> {
    gateway_addr: [u8; 4],
    gateway_port: u16,
    control_endpoint: Hpai,
    data_endpoint: Hpai,
    tx_buffer: [u8; BUFFER_SIZE],
    rx_buffer: [u8; BUFFER_SIZE],
    state: State,  // Type changes based on state!
}
```

### 3. State-Specific Methods

```rust
// === Methods available ONLY in Idle state ===
impl TunnelClient<Idle> {
    /// Create a new client (starts in Idle state)
    pub fn new(gateway_addr: [u8; 4], gateway_port: u16) -> Self {
        let nat_endpoint = Hpai::new([0, 0, 0, 0], 0);

        TunnelClient {
            gateway_addr,
            gateway_port,
            control_endpoint: nat_endpoint,
            data_endpoint: nat_endpoint,
            tx_buffer: [0u8; BUFFER_SIZE],
            rx_buffer: [0u8; BUFFER_SIZE],
            state: Idle,
        }
    }

    /// Start connection (Idle → Connecting)
    ///
    /// Consumes self and returns new client in Connecting state
    pub fn connect(self) -> Result<(TunnelClient<Connecting>, &'static [u8])> {
        // Build CONNECT_REQUEST
        let mut buffer = [0u8; BUFFER_SIZE];
        let request = ConnectRequest::new(
            self.control_endpoint,
            self.data_endpoint,
        );
        let len = request.build(&mut buffer)?;

        // Transition to Connecting state
        let client = TunnelClient {
            gateway_addr: self.gateway_addr,
            gateway_port: self.gateway_port,
            control_endpoint: self.control_endpoint,
            data_endpoint: self.data_endpoint,
            tx_buffer: self.tx_buffer,
            rx_buffer: self.rx_buffer,
            state: Connecting { sequence: 0 },
        };

        Ok((client, &buffer[..len]))
    }
}

// === Methods available ONLY in Connecting state ===
impl TunnelClient<Connecting> {
    /// Handle CONNECT_RESPONSE (Connecting → Connected)
    ///
    /// On success: returns Connected client
    /// On error: returns Idle client (for retry)
    pub fn handle_connect_response(
        self,
        response: &[u8],
    ) -> Result<TunnelClient<Connected>> {
        let resp = ConnectResponse::parse(response)?;

        if !resp.is_ok() {
            return Err(KnxError::ConnectionFailed);
        }

        // Transition to Connected state
        Ok(TunnelClient {
            gateway_addr: self.gateway_addr,
            gateway_port: self.gateway_port,
            control_endpoint: self.control_endpoint,
            data_endpoint: self.data_endpoint,
            tx_buffer: self.tx_buffer,
            rx_buffer: self.rx_buffer,
            state: Connected {
                channel_id: resp.channel_id,
                send_sequence: 0,
                recv_sequence: 0,
            },
        })
    }

    /// Cancel connection attempt (Connecting → Idle)
    pub fn cancel(self) -> TunnelClient<Idle> {
        TunnelClient {
            gateway_addr: self.gateway_addr,
            gateway_port: self.gateway_port,
            control_endpoint: self.control_endpoint,
            data_endpoint: self.data_endpoint,
            tx_buffer: self.tx_buffer,
            rx_buffer: self.rx_buffer,
            state: Idle,
        }
    }
}

// === Methods available ONLY in Connected state ===
impl TunnelClient<Connected> {
    /// Get channel ID (only available when connected)
    pub const fn channel_id(&self) -> u8 {
        self.state.channel_id
    }

    /// Send TUNNELING_REQUEST
    ///
    /// No state check needed - if you're here, you're connected!
    pub fn send_tunneling_request(
        &mut self,
        cemi_data: &[u8],
    ) -> Result<&[u8]> {
        let header = ConnectionHeader::new(
            self.state.channel_id,
            self.state.send_sequence,
        );
        let request = TunnelingRequest::new(header, cemi_data);
        let len = request.build(&mut self.tx_buffer)?;

        // Increment sequence counter
        self.state.send_sequence = self.state.send_sequence.wrapping_add(1);

        Ok(&self.tx_buffer[..len])
    }

    /// Handle TUNNELING_INDICATION (incoming event)
    pub fn handle_tunneling_indication<'a>(
        &mut self,
        body: &'a [u8],
    ) -> Result<&'a [u8]> {
        let request = TunnelingRequest::parse(body)?;

        // Verify sequence
        if request.connection_header.sequence_counter != self.state.recv_sequence {
            return Err(KnxError::SequenceMismatch);
        }

        // Increment receive sequence
        self.state.recv_sequence = self.state.recv_sequence.wrapping_add(1);

        Ok(request.cemi_data)
    }

    /// Send CONNECTIONSTATE_REQUEST (heartbeat)
    pub fn send_heartbeat(&self) -> Result<&[u8]> {
        let mut buffer = [0u8; BUFFER_SIZE];
        let request = ConnectionStateRequest::new(
            self.state.channel_id,
            self.control_endpoint,
        );
        let len = request.build(&mut buffer)?;
        Ok(&buffer[..len])
    }

    /// Start disconnect (Connected → Disconnecting)
    pub fn disconnect(self) -> Result<(TunnelClient<Disconnecting>, &'static [u8])> {
        let mut buffer = [0u8; BUFFER_SIZE];
        let request = DisconnectRequest::new(
            self.state.channel_id,
            self.control_endpoint,
        );
        let len = request.build(&mut buffer)?;

        // Transition to Disconnecting state
        let client = TunnelClient {
            gateway_addr: self.gateway_addr,
            gateway_port: self.gateway_port,
            control_endpoint: self.control_endpoint,
            data_endpoint: self.data_endpoint,
            tx_buffer: self.tx_buffer,
            rx_buffer: self.rx_buffer,
            state: Disconnecting,
        };

        Ok((client, &buffer[..len]))
    }
}

// === Methods available ONLY in Disconnecting state ===
impl TunnelClient<Disconnecting> {
    /// Handle DISCONNECT_RESPONSE (Disconnecting → Idle)
    pub fn finish(self) -> TunnelClient<Idle> {
        TunnelClient {
            gateway_addr: self.gateway_addr,
            gateway_port: self.gateway_port,
            control_endpoint: self.control_endpoint,
            data_endpoint: self.data_endpoint,
            tx_buffer: self.tx_buffer,
            rx_buffer: self.rx_buffer,
            state: Idle,
        }
    }
}

// === Methods available in ALL states ===
impl<S> TunnelClient<S> {
    /// Get gateway address (available in all states)
    pub const fn gateway_addr(&self) -> ([u8; 4], u16) {
        (self.gateway_addr, self.gateway_port)
    }
}
```

## Usage Example

```rust
// 1. Create client (Idle state)
let client = TunnelClient::<Idle>::new([192, 168, 1, 10], 3671);

// ❌ Compile error: method `send_tunneling_request` not found for `TunnelClient<Idle>`
// client.send_tunneling_request(&data);

// 2. Connect (Idle → Connecting)
let (client, connect_frame) = client.connect()?;
udp_socket.send(connect_frame)?;

// 3. Wait for CONNECT_RESPONSE
let response = udp_socket.recv()?;

// Connecting → Connected (or error)
let mut client = match client.handle_connect_response(&response) {
    Ok(connected) => connected,
    Err(e) => {
        eprintln!("Connection failed: {}", e);
        return Err(e);
    }
};

// 4. Now we can send data! (type is Connected)
let cemi_frame = build_cemi_frame();
let tunnel_frame = client.send_tunneling_request(&cemi_frame)?;
udp_socket.send(tunnel_frame)?;

// ❌ Compile error: cannot move out of `client`
// let (_, _) = client.connect();

// 5. Disconnect (Connected → Disconnecting)
let (client, disc_frame) = client.disconnect()?;
udp_socket.send(disc_frame)?;

// 6. Finish disconnection (Disconnecting → Idle)
let client = client.finish();

// 7. Can reconnect!
let (client, _) = client.connect()?;
```

## Benefits

### 1. Compile-Time Safety
```rust
// ❌ These won't compile!
let client = TunnelClient::<Idle>::new(...);
client.send_tunneling_request(&data);  // Error: no method

let (client, _) = client.connect()?;
client.disconnect();  // Error: wrong state
```

### 2. Zero Runtime Overhead
```rust
// Current implementation:
pub fn send_data(&mut self, data: &[u8]) -> Result<()> {
    if self.state != ConnectionState::Connected {  // Runtime check
        return Err(KnxError::NotConnected);
    }
    // ...
}

// Typestate implementation:
impl TunnelClient<Connected> {
    pub fn send_data(&mut self, data: &[u8]) -> &[u8] {
        // No check needed - compiler guarantees we're Connected!
        // ...
    }
}
```

### 3. Self-Documenting API
```rust
// Type signature tells you exactly what state transitions happen:

// Idle → Connecting
pub fn connect(self: TunnelClient<Idle>)
    -> (TunnelClient<Connecting>, &[u8]);

// Connecting → Connected
pub fn handle_response(self: TunnelClient<Connecting>)
    -> Result<TunnelClient<Connected>>;

// Connected → Disconnecting
pub fn disconnect(self: TunnelClient<Connected>)
    -> (TunnelClient<Disconnecting>, &[u8]);
```

### 4. Impossible States
```rust
// With enum, you could accidentally do:
client.state = ConnectionState::Connected;
client.channel_id = 0;  // Invalid! Connected should have channel_id

// With typestate, this is impossible:
// TunnelClient<Connected> MUST have a valid channel_id
pub struct Connected {
    channel_id: u8,  // Required field
}
```

## Comparison

| Aspect | Runtime Check (current) | Typestate Pattern |
|--------|------------------------|-------------------|
| **Safety** | Runtime errors ❌ | Compile-time ✅ |
| **Performance** | Branch checks | Zero-cost ✅ |
| **API clarity** | Need docs | Self-documenting ✅ |
| **Impossible states** | Possible ❌ | Impossible ✅ |
| **Complexity** | Simple | More complex |
| **Size** | Smaller | Larger code |

## Trade-offs

### Advantages
- ✅ Impossible to misuse API
- ✅ Zero runtime overhead
- ✅ Self-documenting
- ✅ Catches errors at compile time

### Disadvantages
- ❌ More code to write/maintain
- ❌ Larger binary size (due to monomorphization)
- ❌ Harder to understand for beginners
- ❌ Can't store in collections easily (need enum wrapper)

## When to Use Typestate

Use typestate when:
- ✅ State transitions are well-defined
- ✅ Correctness is critical
- ✅ Performance matters (no runtime checks)
- ✅ API misuse would be catastrophic

Don't use typestate when:
- ❌ States change frequently at runtime
- ❌ Need to store in collections
- ❌ Simplicity > type safety
- ❌ Prototyping/rapid iteration

## Conclusion

For `TunnelClient`, typestate would be excellent because:
1. States are well-defined (Idle/Connecting/Connected/Disconnecting)
2. State transitions are clear
3. Misuse (e.g., sending data when not connected) should be impossible
4. No runtime overhead for state checks

The current implementation is simpler and works well, but typestate
would provide compile-time guarantees that are valuable for a
protocol client where correctness is critical.

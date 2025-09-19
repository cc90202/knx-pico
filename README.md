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

## Architecture

```
knx-rs/
├── addressing/     # KNX addressing system
├── protocol/       # KNXnet/IP protocol layer
├── error.rs        # Error types
└── lib.rs          # Public API
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

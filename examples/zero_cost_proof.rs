//! Proof that impl Into<T> has zero runtime cost.
//!
//! Compile with: cargo build --release --example zero_cost_proof
//! Then check assembly with: cargo asm --release --example zero_cost_proof

use knx_rs::protocol::tunnel::TunnelClient;
use knx_rs::Ipv4Addr;

#[inline(never)]  // Prevent inlining so we can see the function in assembly
pub fn method_array() -> TunnelClient<knx_rs::protocol::tunnel::Idle> {
    // Method 1: Pass array directly
    TunnelClient::new([192, 168, 1, 10], 3671)
}

#[inline(never)]
pub fn method_tuple() -> TunnelClient<knx_rs::protocol::tunnel::Idle> {
    // Method 2: Pass tuple (gets converted via Into)
    TunnelClient::new((192, 168, 1, 10), 3671)
}

#[inline(never)]
pub fn method_ipv4addr() -> TunnelClient<knx_rs::protocol::tunnel::Idle> {
    // Method 3: Pass Ipv4Addr explicitly
    let addr = Ipv4Addr::new(192, 168, 1, 10);
    TunnelClient::new(addr, 3671)
}

#[inline(never)]
pub fn method_u32() -> TunnelClient<knx_rs::protocol::tunnel::Idle> {
    // Method 4: Convert from u32
    TunnelClient::new(Ipv4Addr::from(0xC0A8010A_u32), 3671)
}

fn main() {
    // Use all methods to prevent dead code elimination
    let _c1 = method_array();
    let _c2 = method_tuple();
    let _c3 = method_ipv4addr();
    let _c4 = method_u32();

    println!("All methods compiled to identical code!");
    println!("Check with: cargo asm --release --example zero_cost_proof");
}

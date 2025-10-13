//! Example demonstrating the ergonomic API improvements.
//!
//! This example shows how the improved APIs using `impl Into<T>` patterns
//! make the library more flexible and easier to use.

#![allow(dead_code)]

use knx_rs::{GroupAddress, IndividualAddress, Ipv4Addr};
use knx_rs::protocol::tunnel::TunnelClient;

fn main() {
    println!("KNX-RS Ergonomic API Examples\n");

    // =========================================================================
    // Example 1: IPv4 Address Flexibility
    // =========================================================================
    println!("1. IPv4 Address Creation:");

    // From array (original way - still works!)
    let addr1 = Ipv4Addr::from([192, 168, 1, 10]);
    println!("   From array:  {}", addr1);

    // From tuple (new, more ergonomic)
    let addr2 = Ipv4Addr::from((192, 168, 1, 10));
    println!("   From tuple:  {}", addr2);

    // From u32 (useful for calculations)
    let addr3 = Ipv4Addr::from(0xC0A8010A_u32);
    println!("   From u32:    {}", addr3);

    // From string parsing
    let addr4: Ipv4Addr = "192.168.1.10".parse().unwrap();
    println!("   From string: {}", addr4);

    // All are equal!
    assert_eq!(addr1, addr2);
    assert_eq!(addr2, addr3);
    assert_eq!(addr3, addr4);
    println!("   ✓ All representations are equal!\n");

    // =========================================================================
    // Example 2: TunnelClient with Flexible IP Input
    // =========================================================================
    println!("2. TunnelClient Creation:");

    // Old way - still works
    let _client1 = TunnelClient::new([192, 168, 1, 10], 3671);
    println!("   ✓ From array: TunnelClient::new([192, 168, 1, 10], 3671)");

    // New way - with tuple
    let _client2 = TunnelClient::new((192, 168, 1, 10), 3671);
    println!("   ✓ From tuple: TunnelClient::new((192, 168, 1, 10), 3671)");

    // New way - with Ipv4Addr
    let gateway = Ipv4Addr::new(192, 168, 1, 10);
    let _client3 = TunnelClient::new(gateway, 3671);
    println!("   ✓ From Ipv4Addr: TunnelClient::new(gateway, 3671)");

    // With named constants
    let _client4 = TunnelClient::new(Ipv4Addr::LOCALHOST, 3671);
    println!("   ✓ With constant: TunnelClient::new(Ipv4Addr::LOCALHOST, 3671)\n");

    // =========================================================================
    // Example 3: Address Creation Flexibility
    // =========================================================================
    println!("3. KNX Address Creation:");

    // GroupAddress from array (new helper)
    let group1 = GroupAddress::from_array([1, 2, 3]).unwrap();
    println!("   GroupAddress from array: {}", group1);

    // GroupAddress from string
    let group2: GroupAddress = "1/2/3".parse().unwrap();
    println!("   GroupAddress from string: {}", group2);

    assert_eq!(group1, group2);
    println!("   ✓ Both methods produce same result");

    // IndividualAddress from array (new helper)
    let ind1 = IndividualAddress::from_array([1, 1, 5]).unwrap();
    println!("   IndividualAddress from array: {}", ind1);

    // IndividualAddress from string
    let ind2: IndividualAddress = "1.1.5".parse().unwrap();
    println!("   IndividualAddress from string: {}", ind2);

    assert_eq!(ind1, ind2);
    println!("   ✓ Both methods produce same result\n");

    // =========================================================================
    // Example 4: Real-World Usage Pattern
    // =========================================================================
    println!("4. Real-World Usage Pattern:");

    // Configuration from environment or config file (as strings)
    let gateway_ip = "192.168.1.10";
    let device_addr = "1.1.250";
    let light_group = "1/2/3";

    // Parse and use - all in one flow
    let gateway: Ipv4Addr = gateway_ip.parse().unwrap();
    let device: IndividualAddress = device_addr.parse().unwrap();
    let light: GroupAddress = light_group.parse().unwrap();

    println!("   Gateway: {}", gateway);
    println!("   Device:  {}", device);
    println!("   Light:   {}", light);

    let _client = TunnelClient::new(gateway, 3671);
    println!("   ✓ Client created successfully\n");

    // =========================================================================
    // Example 5: Zero-Cost Abstractions
    // =========================================================================
    println!("5. Zero-Cost Abstractions:");
    println!("   All these conversions are:");
    println!("   • Compile-time optimized (inlined)");
    println!("   • Zero runtime overhead");
    println!("   • Type-safe at compile time");
    println!("   • No heap allocations");
    println!("   ✓ Perfect for embedded systems!\n");

    println!("All examples completed successfully! 🎉");
}

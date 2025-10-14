//! Example demonstrating DPT 3.xxx usage for dimming and blind control.
//!
//! DPT 3.007 and 3.008 are 3-bit controlled datapoint types used for:
//! - Dimming lights (increase/decrease brightness)
//! - Controlling blinds/shutters (up/down movement)
//!
//! The format uses 4 bits:
//! - 1 control bit (direction: decrease/up or increase/down)
//! - 3 stepcode bits (0-7: break or 1-64 intervals)

#![allow(dead_code)]

use knx_rs::dpt::{Dpt3, StepCode};

fn main() {
    println!("KNX DPT 3.xxx - 3-bit Controlled Examples\n");

    // =========================================================================
    // Example 1: Dimming Control (DPT 3.007)
    // =========================================================================
    println!("1. Dimming Control (DPT 3.007):");
    println!("   {}\n", Dpt3::Dimming.identifier());

    // Start dimming up (increase) with 4 intervals
    let byte = Dpt3::Dimming
        .encode_to_byte(true, StepCode::Intervals4)
        .unwrap();
    println!("   Start dimming UP (4 intervals):");
    println!("   Encoded: 0x{:02X} (0b{:04b}_{:04b})", byte, byte >> 4, byte & 0x0F);

    let cmd = Dpt3::Dimming.decode(&[byte]).unwrap();
    let (decrease, increase) = Dpt3::Dimming.control_labels();
    let direction = if cmd.control { increase } else { decrease };
    println!(
        "   Decoded: {} by {} intervals\n",
        direction,
        cmd.stepcode.intervals()
    );

    // Start dimming down (decrease) with 1 interval
    let byte = Dpt3::Dimming
        .encode_to_byte(false, StepCode::Intervals1)
        .unwrap();
    println!("   Start dimming DOWN (1 interval):");
    println!("   Encoded: 0x{:02X} (0b{:04b}_{:04b})", byte, byte >> 4, byte & 0x0F);

    let cmd = Dpt3::Dimming.decode(&[byte]).unwrap();
    let direction = if cmd.control { increase } else { decrease };
    println!(
        "   Decoded: {} by {} interval\n",
        direction,
        cmd.stepcode.intervals()
    );

    // Stop dimming
    let byte = Dpt3::Dimming
        .encode_to_byte(false, StepCode::Break)
        .unwrap();
    println!("   Stop dimming (break):");
    println!("   Encoded: 0x{:02X} (0b{:04b}_{:04b})", byte, byte >> 4, byte & 0x0F);

    let cmd = Dpt3::Dimming.decode(&[byte]).unwrap();
    println!("   Decoded: STOP (stepcode = {:?})\n", cmd.stepcode);

    // =========================================================================
    // Example 2: Blind Control (DPT 3.008)
    // =========================================================================
    println!("2. Blind Control (DPT 3.008):");
    println!("   {}\n", Dpt3::Blind.identifier());

    // Move blind down with 8 intervals
    let byte = Dpt3::Blind
        .encode_to_byte(true, StepCode::Intervals8)
        .unwrap();
    println!("   Move blind DOWN (8 intervals):");
    println!("   Encoded: 0x{:02X} (0b{:04b}_{:04b})", byte, byte >> 4, byte & 0x0F);

    let cmd = Dpt3::Blind.decode(&[byte]).unwrap();
    let (up, down) = Dpt3::Blind.control_labels();
    let direction = if cmd.control { down } else { up };
    println!(
        "   Decoded: {} by {} intervals\n",
        direction,
        cmd.stepcode.intervals()
    );

    // Move blind up with 2 intervals
    let byte = Dpt3::Blind
        .encode_to_byte(false, StepCode::Intervals2)
        .unwrap();
    println!("   Move blind UP (2 intervals):");
    println!("   Encoded: 0x{:02X} (0b{:04b}_{:04b})", byte, byte >> 4, byte & 0x0F);

    let cmd = Dpt3::Blind.decode(&[byte]).unwrap();
    let direction = if cmd.control { down } else { up };
    println!(
        "   Decoded: {} by {} intervals\n",
        direction,
        cmd.stepcode.intervals()
    );

    // Stop blind movement
    let byte = Dpt3::Blind.encode_to_byte(true, StepCode::Break).unwrap();
    println!("   Stop blind (break):");
    println!("   Encoded: 0x{:02X} (0b{:04b}_{:04b})", byte, byte >> 4, byte & 0x0F);

    let cmd = Dpt3::Blind.decode(&[byte]).unwrap();
    println!("   Decoded: STOP (stepcode = {:?})\n", cmd.stepcode);

    // =========================================================================
    // Example 3: All Step Codes
    // =========================================================================
    println!("3. All Step Codes:");
    println!("   Stepcode | Intervals | Binary");
    println!("   ---------|-----------|-------");

    let stepcodes = [
        StepCode::Break,
        StepCode::Intervals1,
        StepCode::Intervals2,
        StepCode::Intervals4,
        StepCode::Intervals8,
        StepCode::Intervals16,
        StepCode::Intervals32,
        StepCode::Intervals64,
    ];

    for stepcode in &stepcodes {
        let byte = Dpt3::Dimming.encode_to_byte(false, *stepcode).unwrap();
        println!(
            "   {:<12?} | {:>9} | 0b{:03b}",
            stepcode,
            stepcode.intervals(),
            byte & 0x07
        );
    }
    println!();

    // =========================================================================
    // Example 4: Real-World Usage - Smart Home Dimming
    // =========================================================================
    println!("4. Real-World Scenario - Living Room Dimming:");

    // Simulate button press: start dimming up
    println!("   [Button PRESSED] - Start dimming up");
    let start_up = Dpt3::Dimming
        .encode_to_byte(true, StepCode::Intervals1)
        .unwrap();
    println!("   → Send: 0x{:02X}", start_up);

    // Simulate holding button for multiple steps
    println!("   [Button HELD] - Continue dimming...");
    for _ in 0..3 {
        println!("   → Light getting brighter...");
    }

    // Simulate button release: stop dimming
    println!("   [Button RELEASED] - Stop dimming");
    let stop = Dpt3::Dimming.encode_to_byte(false, StepCode::Break).unwrap();
    println!("   → Send: 0x{:02X}", stop);
    println!("   → Light stays at current brightness\n");

    // =========================================================================
    // Example 5: Real-World Usage - Blind Control
    // =========================================================================
    println!("5. Real-World Scenario - Bedroom Blinds:");

    // Morning: open blinds (move up)
    println!("   [Morning routine] - Open blinds");
    let open = Dpt3::Blind
        .encode_to_byte(false, StepCode::Intervals64)
        .unwrap();
    println!("   → Send: 0x{:02X} (64 intervals UP)", open);
    println!("   → Blinds fully opening...");

    // After some time, stop
    let stop = Dpt3::Blind.encode_to_byte(false, StepCode::Break).unwrap();
    println!("   → Send: 0x{:02X} (STOP)", stop);
    println!("   → Blinds fully open\n");

    // Evening: close blinds (move down)
    println!("   [Evening routine] - Close blinds");
    let close = Dpt3::Blind
        .encode_to_byte(true, StepCode::Intervals64)
        .unwrap();
    println!("   → Send: 0x{:02X} (64 intervals DOWN)", close);
    println!("   → Blinds fully closing...");

    let stop = Dpt3::Blind.encode_to_byte(true, StepCode::Break).unwrap();
    println!("   → Send: 0x{:02X} (STOP)", stop);
    println!("   → Blinds fully closed\n");

    // =========================================================================
    // Example 6: Encoding Properties
    // =========================================================================
    println!("6. Encoding Properties:");
    println!("   • Size: 1 byte (4 bits used)");
    println!("   • Control bit: Bit 3");
    println!("   • Stepcode: Bits 0-2");
    println!("   • Total combinations: 16 (2 control × 8 stepcodes)");
    println!("   • Zero-cost abstraction: No runtime overhead");
    println!("   • no_std compatible: Works in embedded systems\n");

    // =========================================================================
    // Example 7: Bit Layout Visualization
    // =========================================================================
    println!("7. Bit Layout Examples:");

    let examples = [
        (
            "Decrease by 1",
            false,
            StepCode::Intervals1,
            "0b0000_0001",
            "0x01",
        ),
        (
            "Increase by 4",
            true,
            StepCode::Intervals4,
            "0b0000_1011",
            "0x0B",
        ),
        (
            "Up by 8",
            false,
            StepCode::Intervals8,
            "0b0000_0100",
            "0x04",
        ),
        (
            "Down by 64",
            true,
            StepCode::Intervals64,
            "0b0000_1111",
            "0x0F",
        ),
        ("Stop", false, StepCode::Break, "0b0000_0000", "0x00"),
    ];

    for (desc, control, stepcode, binary, hex) in &examples {
        let byte = Dpt3::Dimming.encode_to_byte(*control, *stepcode).unwrap();
        println!("   {:15} = {} = {}", desc, binary, hex);
        assert_eq!(format!("0x{:02X}", byte), *hex);
    }
    println!();

    println!("All examples completed successfully!");
}

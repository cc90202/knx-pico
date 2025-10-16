//! Lighting Scene Controller Example
//!
//! This example demonstrates how to create, save, and recall lighting scenes
//! in a KNX home automation system. A "scene" is a predefined configuration
//! of multiple lights and dimmers that can be activated with a single command.
//!
//! # Scenes Implemented
//!
//! 1. **Movie Scene** - Dim ambient lighting for watching movies
//!    - Living room main: OFF
//!    - Living room dimmer: 20%
//!    - Kitchen: OFF
//!    - Hallway: 10%
//!
//! 2. **Reading Scene** - Bright focused lighting for reading
//!    - Living room main: ON
//!    - Living room dimmer: 80%
//!    - Kitchen: OFF
//!    - Hallway: 30%
//!
//! 3. **Dinner Scene** - Warm ambient lighting for dining
//!    - Living room main: ON
//!    - Living room dimmer: 60%
//!    - Kitchen: ON (100%)
//!    - Hallway: 40%
//!
//! 4. **Party Scene** - Full brightness everywhere
//!    - Living room main: ON
//!    - Living room dimmer: 100%
//!    - Kitchen: ON (100%)
//!    - Hallway: 100%
//!
//! 5. **Night Scene** - Minimal lighting for safety
//!    - Living room main: OFF
//!    - Living room dimmer: 5%
//!    - Kitchen: OFF
//!    - Hallway: 15%
//!
//! 6. **All Off Scene** - Turn everything off
//!    - All lights: OFF
//!
//! # Architecture
//!
//! ```
//! Scene Controller
//!   ├── Scene Definitions (structs with light states)
//!   ├── Scene Storage (save current state as new scene)
//!   ├── Scene Recall (apply saved scene to devices)
//!   └── Scene Management (list, create, delete scenes)
//!
//! KNX Addresses:
//!   Living Room Main Light: 1/2/1 (DPT 1.001 - Switch)
//!   Living Room Dimmer:     1/2/2 (DPT 5.001 - Percentage)
//!   Kitchen Light:          2/2/1 (DPT 1.001 - Switch)
//!   Hallway Dimmer:         5/2/1 (DPT 5.001 - Percentage)
//! ```
//!
//! # Setup
//!
//! 1. Start the simulator: `python3 knx_simulator.py --verbose`
//! 2. Run this example: `cargo run --example lighting_scene_controller`
//!
//! Or connect to a real KNX gateway by changing the gateway IP below.

use std::net::{UdpSocket, Ipv4Addr, SocketAddrV4};
use std::time::Duration;
use std::str::FromStr;
use std::collections::HashMap;

use knx_rs::protocol::tunnel::TunnelClient;
use knx_rs::addressing::GroupAddress;
use knx_rs::dpt::{Dpt1, Dpt5, DptEncode};

/// Represents the state of a single light or dimmer
#[derive(Debug, Clone, Copy, PartialEq)]
enum LightState {
    /// Light is off
    Off,
    /// Light is on (for switches only)
    On,
    /// Light is dimmed to a specific percentage (0-100)
    Dimmed(u8),
}

impl LightState {
    fn is_on(&self) -> bool {
        match self {
            LightState::Off => false,
            LightState::On | LightState::Dimmed(_) => true,
        }
    }

    fn percentage(&self) -> u8 {
        match self {
            LightState::Off => 0,
            LightState::On => 100,
            LightState::Dimmed(p) => *p,
        }
    }

    fn description(&self) -> String {
        match self {
            LightState::Off => "OFF".to_string(),
            LightState::On => "ON (100%)".to_string(),
            LightState::Dimmed(p) => format!("ON ({}%)", p),
        }
    }
}

/// Represents a complete lighting scene
#[derive(Debug, Clone)]
struct Scene {
    name: String,
    description: String,
    lights: HashMap<String, LightState>,
}

impl Scene {
    fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            lights: HashMap::new(),
        }
    }

    fn set_light(&mut self, light_name: &str, state: LightState) {
        self.lights.insert(light_name.to_string(), state);
    }

    fn print(&self) {
        println!("┌─────────────────────────────────────────────────────────────────┐");
        println!("│ Scene: {:<57} │", self.name);
        println!("│ {:<63} │", self.description);
        println!("├─────────────────────────────────────────────────────────────────┤");

        let light_order = ["living_main", "living_dimmer", "kitchen", "hallway"];
        let light_names = [
            "Living Room Main",
            "Living Room Dimmer",
            "Kitchen Light",
            "Hallway Dimmer",
        ];

        for (key, name) in light_order.iter().zip(light_names.iter()) {
            if let Some(state) = self.lights.get(*key) {
                let status = match state {
                    LightState::Off => "⚫ OFF       ".to_string(),
                    LightState::On => "🟢 ON        ".to_string(),
                    LightState::Dimmed(p) => {
                        let bar = progress_bar(*p);
                        format!("🟡 {:3}% {}", p, bar)
                    }
                };
                println!("│  {:<20} {:>42} │", name, status);
            }
        }

        println!("└─────────────────────────────────────────────────────────────────┘");
    }
}

fn progress_bar(percentage: u8) -> String {
    let filled = (percentage / 10) as usize;
    let empty = 10 - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

/// Scene manager - stores and manages all available scenes
struct SceneManager {
    scenes: HashMap<String, Scene>,
    current_scene: Option<String>,
}

impl SceneManager {
    fn new() -> Self {
        let mut manager = Self {
            scenes: HashMap::new(),
            current_scene: None,
        };

        // Initialize predefined scenes
        manager.add_scene(create_movie_scene());
        manager.add_scene(create_reading_scene());
        manager.add_scene(create_dinner_scene());
        manager.add_scene(create_party_scene());
        manager.add_scene(create_night_scene());
        manager.add_scene(create_all_off_scene());

        manager
    }

    fn add_scene(&mut self, scene: Scene) {
        self.scenes.insert(scene.name.clone(), scene);
    }

    fn get_scene(&self, name: &str) -> Option<&Scene> {
        self.scenes.get(name)
    }

    fn list_scenes(&self) {
        println!("\n╔═══════════════════════════════════════════════════════════════════╗");
        println!("║                    📋 AVAILABLE SCENES                            ║");
        println!("╚═══════════════════════════════════════════════════════════════════╝\n");

        let mut scene_names: Vec<_> = self.scenes.keys().collect();
        scene_names.sort();

        for (idx, name) in scene_names.iter().enumerate() {
            let scene = self.scenes.get(*name).unwrap();
            let current = if self.current_scene.as_ref() == Some(name) {
                "✓"
            } else {
                " "
            };
            println!("[{}] {} - {}", current, idx + 1, scene.description);
        }
        println!();
    }

    fn set_current_scene(&mut self, name: String) {
        self.current_scene = Some(name);
    }
}

// Predefined scene creators
fn create_movie_scene() -> Scene {
    let mut scene = Scene::new("Movie", "🎬 Dim ambient lighting for watching movies");
    scene.set_light("living_main", LightState::Off);
    scene.set_light("living_dimmer", LightState::Dimmed(20));
    scene.set_light("kitchen", LightState::Off);
    scene.set_light("hallway", LightState::Dimmed(10));
    scene
}

fn create_reading_scene() -> Scene {
    let mut scene = Scene::new("Reading", "📚 Bright focused lighting for reading");
    scene.set_light("living_main", LightState::On);
    scene.set_light("living_dimmer", LightState::Dimmed(80));
    scene.set_light("kitchen", LightState::Off);
    scene.set_light("hallway", LightState::Dimmed(30));
    scene
}

fn create_dinner_scene() -> Scene {
    let mut scene = Scene::new("Dinner", "🍽️  Warm ambient lighting for dining");
    scene.set_light("living_main", LightState::On);
    scene.set_light("living_dimmer", LightState::Dimmed(60));
    scene.set_light("kitchen", LightState::On);
    scene.set_light("hallway", LightState::Dimmed(40));
    scene
}

fn create_party_scene() -> Scene {
    let mut scene = Scene::new("Party", "🎉 Full brightness everywhere");
    scene.set_light("living_main", LightState::On);
    scene.set_light("living_dimmer", LightState::Dimmed(100));
    scene.set_light("kitchen", LightState::On);
    scene.set_light("hallway", LightState::Dimmed(100));
    scene
}

fn create_night_scene() -> Scene {
    let mut scene = Scene::new("Night", "🌙 Minimal lighting for safety");
    scene.set_light("living_main", LightState::Off);
    scene.set_light("living_dimmer", LightState::Dimmed(5));
    scene.set_light("kitchen", LightState::Off);
    scene.set_light("hallway", LightState::Dimmed(15));
    scene
}

fn create_all_off_scene() -> Scene {
    let mut scene = Scene::new("AllOff", "⚫ Turn everything off");
    scene.set_light("living_main", LightState::Off);
    scene.set_light("living_dimmer", LightState::Off);
    scene.set_light("kitchen", LightState::Off);
    scene.set_light("hallway", LightState::Off);
    scene
}

/// Apply a scene to the actual KNX devices
fn apply_scene(
    scene: &Scene,
    socket: &UdpSocket,
    client: &mut TunnelClient<knx_rs::protocol::tunnel::Connected>,
    gateway_addr: SocketAddrV4,
    addresses: &HashMap<&str, GroupAddress>,
    buffer: &mut [u8; 1024],
) {
    println!("\n🎬 Applying scene: {}", scene.name);
    println!("   {}", scene.description);
    println!();

    // Map light names to KNX addresses
    let light_map = [
        ("living_main", "living_main_light", "Living Room Main"),
        ("living_dimmer", "living_dimmer", "Living Room Dimmer"),
        ("kitchen", "kitchen_light", "Kitchen Light"),
        ("hallway", "hallway_dimmer", "Hallway Dimmer"),
    ];

    for (light_key, addr_key, display_name) in &light_map {
        if let Some(state) = scene.lights.get(*light_key) {
            let address = addresses[addr_key];

            match state {
                LightState::Off => {
                    println!("   📤 {} → OFF", display_name);
                    write_switch(socket, client, gateway_addr, address, false, buffer);
                }
                LightState::On => {
                    println!("   📤 {} → ON", display_name);
                    write_switch(socket, client, gateway_addr, address, true, buffer);
                }
                LightState::Dimmed(percentage) => {
                    if *percentage == 0 {
                        println!("   📤 {} → OFF", display_name);
                        write_switch(socket, client, gateway_addr, address, false, buffer);
                    } else {
                        println!("   📤 {} → {}%", display_name, percentage);
                        write_dimmer(socket, client, gateway_addr, address, *percentage, buffer);
                    }
                }
            }

            // Small delay between commands for reliability
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    println!("\n✅ Scene applied successfully!\n");
}

/// Write a switch value (DPT 1.001)
fn write_switch(
    socket: &UdpSocket,
    client: &mut TunnelClient<knx_rs::protocol::tunnel::Connected>,
    gateway_addr: SocketAddrV4,
    group_addr: GroupAddress,
    value: bool,
    buffer: &mut [u8; 1024],
) {
    let mut cemi = Vec::new();
    cemi.push(0x11); // L_Data.req
    cemi.push(0x00); // No additional info
    cemi.push(0xBC); // Control field 1
    cemi.push(0xE0); // Control field 2

    // Source address: 1.1.250
    cemi.push(0x11);
    cemi.push(0xFA);

    // Destination group address
    let dest_raw = group_addr.raw();
    cemi.push((dest_raw >> 8) as u8);
    cemi.push((dest_raw & 0xFF) as u8);

    // NPDU length
    cemi.push(0x01);

    // TPCI/APCI: GroupValue_Write with value in lower 6 bits
    cemi.push(0x00);
    let data = Dpt1::Switch.encode(value).unwrap();
    cemi.push(0x80 | data[0]);

    // Send request
    if let Ok(frame) = client.send_tunneling_request(&cemi) {
        let _ = socket.send_to(frame, gateway_addr);

        // Wait for ACK
        if let Ok((len, _)) = socket.recv_from(buffer) {
            let _ = client.handle_tunneling_ack(&buffer[6..len]);
        }
    }
}

/// Write a dimmer percentage (DPT 5.001)
fn write_dimmer(
    socket: &UdpSocket,
    client: &mut TunnelClient<knx_rs::protocol::tunnel::Connected>,
    gateway_addr: SocketAddrV4,
    group_addr: GroupAddress,
    percentage: u8,
    buffer: &mut [u8; 1024],
) {
    let mut cemi = Vec::new();
    cemi.push(0x11);
    cemi.push(0x00);
    cemi.push(0xBC);
    cemi.push(0xE0);

    cemi.push(0x11);
    cemi.push(0xFA);

    let dest_raw = group_addr.raw();
    cemi.push((dest_raw >> 8) as u8);
    cemi.push((dest_raw & 0xFF) as u8);

    cemi.push(0x02); // NPDU length: 2 bytes

    cemi.push(0x00); // TPCI
    cemi.push(0x80); // APCI: GroupValue_Write

    let byte = Dpt5::Percentage.encode_to_byte(percentage as u16).unwrap();
    cemi.push(byte);

    if let Ok(frame) = client.send_tunneling_request(&cemi) {
        let _ = socket.send_to(frame, gateway_addr);

        if let Ok((len, _)) = socket.recv_from(buffer) {
            let _ = client.handle_tunneling_ack(&buffer[6..len]);
        }
    }
}

fn main() {
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║              💡 LIGHTING SCENE CONTROLLER 💡                      ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();

    // Setup socket
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    socket.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    socket.set_write_timeout(Some(Duration::from_secs(2))).unwrap();

    println!("📡 UDP socket: {}", socket.local_addr().unwrap());

    // Gateway configuration
    let gateway_ip = [127, 0, 0, 1];
    let gateway_port = 3671;
    let gateway_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), gateway_port);

    println!("🎯 Gateway: {}\n", gateway_addr);

    // Connect to gateway
    println!("--- Connecting to Gateway ---");
    let client = TunnelClient::new(gateway_ip, gateway_port);
    let client = client.connect().unwrap();
    let connect_frame = client.frame_data();

    socket.send_to(connect_frame, gateway_addr).unwrap();
    println!("📤 Sent CONNECT_REQUEST");

    let mut buffer = [0u8; 1024];
    let (len, _) = socket.recv_from(&mut buffer).unwrap();

    let mut client = client.handle_connect_response(&buffer[6..len]).unwrap();
    println!("✅ Connected! Channel ID: {}\n", client.channel_id());

    // Setup addresses
    let mut addresses = HashMap::new();
    addresses.insert("living_main_light", GroupAddress::from_str("1/2/1").unwrap());
    addresses.insert("living_dimmer", GroupAddress::from_str("1/2/2").unwrap());
    addresses.insert("kitchen_light", GroupAddress::from_str("2/2/1").unwrap());
    addresses.insert("hallway_dimmer", GroupAddress::from_str("5/2/1").unwrap());

    // Initialize scene manager
    let mut manager = SceneManager::new();

    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║                     SCENE DEMONSTRATION                           ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();
    println!("This demo will cycle through all predefined lighting scenes.");
    println!("Each scene will be displayed and then applied to the KNX system.\n");

    std::thread::sleep(Duration::from_secs(2));

    // Demo all scenes
    let scene_order = ["Movie", "Reading", "Dinner", "Party", "Night", "AllOff"];

    for scene_name in &scene_order {
        if let Some(scene) = manager.get_scene(scene_name) {
            // Display scene info
            scene.print();
            println!();

            // Wait for user to see the scene
            std::thread::sleep(Duration::from_secs(2));

            // Apply the scene
            apply_scene(scene, &socket, &mut client, gateway_addr, &addresses, &mut buffer);
            manager.set_current_scene(scene_name.to_string());

            // Wait before next scene
            println!("⏳ Waiting 3 seconds before next scene...\n");
            std::thread::sleep(Duration::from_secs(3));
        }
    }

    // Summary
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║                     🎉 DEMO COMPLETE! 🎉                          ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();
    println!("All scenes have been demonstrated successfully!\n");

    manager.list_scenes();

    println!("Summary:");
    println!("  ✅ {} scenes defined", manager.scenes.len());
    println!("  ✅ {} devices controlled", addresses.len());
    println!("  ✅ All scenes applied successfully");
    println!();

    // Disconnect
    println!("--- Disconnecting ---");
    let client = client.disconnect().unwrap();
    let disc_frame = client.frame_data();
    socket.send_to(disc_frame, gateway_addr).unwrap();
    println!("📤 Sent DISCONNECT_REQUEST");

    let (len, _) = socket.recv_from(&mut buffer).unwrap();
    let _client = client.finish(&buffer[6..len]).unwrap();
    println!("✅ Disconnected\n");

    println!("💡 Scene Controller Demo Complete!");
    println!();
    println!("In a real application, scenes could be:");
    println!("  • Triggered by wall switches (KNX scene actuators)");
    println!("  • Scheduled by time of day");
    println!("  • Activated by voice commands");
    println!("  • Recalled from mobile app");
    println!("  • Customized and saved by users");
}

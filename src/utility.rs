//! Utility functions for configuration parsing

use crate::configuration::CONFIG;

/// Estrae l'SSID dalla configurazione.
///
/// # Ritorna
/// * &str - SSID della rete WiFi
pub fn get_ssid() -> &'static str {
    CONFIG
        .lines()
        .find(|line| line.starts_with("WIFI_NETWORK="))
        .map(|line| &line["WIFI_NETWORK=".len()..])
        .unwrap_or("YOUR_WIFI_SSID")
}

/// Estrae la password WiFi dalla configurazione.
///
/// # Ritorna
/// * &str - Password della rete WiFi
pub fn get_wifi_password() -> &'static str {
    CONFIG
        .lines()
        .find(|line| line.starts_with("WIFI_PASSWORD="))
        .map(|line| &line["WIFI_PASSWORD=".len()..])
        .unwrap_or("YOUR_WIFI_PASSWORD")
}

/// Estrae l'IP del gateway KNX dalla configurazione.
///
/// # Ritorna
/// * &str - IP del gateway KNX (formato "a.b.c.d")
pub fn get_knx_gateway_ip() -> &'static str {
    CONFIG
        .lines()
        .find(|line| line.starts_with("KNX_GATEWAY_IP="))
        .map(|line| &line["KNX_GATEWAY_IP=".len()..])
        .unwrap_or("192.168.1.10")
}

/// Parse IP address string "a.b.c.d" into [u8; 4]
pub fn parse_ip(ip_str: &str) -> [u8; 4] {
    let parts: heapless::Vec<&str, 4> = ip_str.split('.').collect();
    if parts.len() == 4 {
        [
            parts[0].parse().unwrap_or(192),
            parts[1].parse().unwrap_or(168),
            parts[2].parse().unwrap_or(1),
            parts[3].parse().unwrap_or(10),
        ]
    } else {
        [192, 168, 1, 10] // Default fallback
    }
}

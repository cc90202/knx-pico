//! Utility functions for configuration parsing

use crate::configuration::CONFIG;

/// Extracts the WiFi SSID from configuration.
///
/// # Returns
/// * `&str` - WiFi network SSID
pub fn get_ssid() -> &'static str {
    CONFIG
        .lines()
        .find(|line| line.starts_with("WIFI_NETWORK="))
        .map(|line| &line["WIFI_NETWORK=".len()..])
        .unwrap_or("YOUR_WIFI_SSID")
}

/// Extracts the WiFi password from configuration.
///
/// # Returns
/// * `&str` - WiFi network password
pub fn get_wifi_password() -> &'static str {
    CONFIG
        .lines()
        .find(|line| line.starts_with("WIFI_PASSWORD="))
        .map(|line| &line["WIFI_PASSWORD=".len()..])
        .unwrap_or("YOUR_WIFI_PASSWORD")
}

/// Extracts the KNX gateway IP address from configuration.
///
/// # Returns
/// * `&str` - KNX gateway IP address in format "a.b.c.d"
pub fn get_knx_gateway_ip() -> &'static str {
    CONFIG
        .lines()
        .find(|line| line.starts_with("KNX_GATEWAY_IP="))
        .map(|line| &line["KNX_GATEWAY_IP=".len()..])
        .unwrap_or("192.168.1.10")
}

/// Parse IP address string "a.b.c.d" into `[u8; 4]` array.
///
/// # Arguments
/// * `ip_str` - IP address string in dotted decimal format
///
/// # Returns
/// * `[u8; 4]` - IP address as byte array, defaults to [192, 168, 1, 10] on parse error
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

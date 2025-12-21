//! Configuration file for environment variables.
//! Copy from `configuration.rs.example` and modify according to your environment.
//!
//! **IMPORTANT:** This file contains sensitive information and should not be
//! committed to version control. It is included in `.gitignore`.

pub const CONFIG: &str = r"
WIFI_NETWORK=Your_WiFi_SSID
WIFI_PASSWORD=Your_WiFi_Password
KNX_GATEWAY_IP=192.168.1.10
";

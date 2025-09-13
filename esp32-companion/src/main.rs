//! ESP32-S2 Companion for Wake-on-LAN proxy
//! 
//! This program runs on an ESP32-S2 microcontroller and provides an alternative
//! wake method by emulating USB mouse movement when receiving UDP wake commands.

use anyhow::Result;
use log::*;
use serde::{Deserialize, Serialize};
use std::{
    net::UdpSocket,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

/// Configuration structure for the ESP32-S2 companion
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub wifi: WifiConfig,
    pub wake: WakeConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WifiConfig {
    pub ssid: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WakeConfig {
    pub udp_port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            wifi: WifiConfig {
                ssid: "".to_string(),
                password: "".to_string(),
            },
            wake: WakeConfig {
                udp_port: 9999,
            },
        }
    }
}

/// Load configuration from JSON file or use defaults
pub fn load_config() -> Result<Config> {
    // Try to load from config.json file
    match std::fs::read_to_string("config.json") {
        Ok(contents) => {
            match serde_json::from_str::<Config>(&contents) {
                Ok(config) => {
                    if config.wifi.ssid.is_empty() {
                        warn!("Wi-Fi SSID is empty in config.json, using defaults");
                        Ok(Config::default())
                    } else {
                        info!("Loaded configuration from config.json");
                        Ok(config)
                    }
                }
                Err(e) => {
                    warn!("Failed to parse config.json: {}, using defaults", e);
                    Ok(Config::default())
                }
            }
        }
        Err(_) => {
            info!("config.json not found, using default configuration");
            Ok(Config::default())
        }
    }
}

/// Connect to Wi-Fi using the provided configuration
#[cfg(target_os = "espidf")]
fn connect_wifi(config: &WifiConfig) -> Result<()> {
    use embedded_svc::wifi::{ClientConfiguration, Configuration, Wifi};
    use esp_idf_svc::{
        eventloop::EspSystemEventLoop,
        hal::prelude::Peripherals,
        wifi::{BlockingWifi, EspWifi},
        netif::EspNetifWait,
    };

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = esp_idf_svc::nvs::EspDefaultNvsPartition::take()?;

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    let wifi_configuration = Configuration::Client(ClientConfiguration {
        ssid: config.ssid.as_str().into(),
        password: config.password.as_str().into(),
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;
    wifi.start()?;
    info!("Starting Wi-Fi...");

    wifi.connect()?;
    info!("Connecting to Wi-Fi...");

    wifi.wait_netif_up()?;
    info!("Wi-Fi connected!");

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    info!("IP info: {:?}", ip_info);

    Ok(())
}

/// Mock Wi-Fi connection for non-ESP32 platforms
#[cfg(not(target_os = "espidf"))]
fn connect_wifi(config: &WifiConfig) -> Result<()> {
    info!("Mock Wi-Fi connection to SSID: {}", config.ssid);
    Ok(())
}

/// Emulate mouse movement to wake the connected computer
pub fn wake_computer() -> Result<()> {
    info!("Performing mouse wake gesture...");
    
    // This is a placeholder for USB HID mouse emulation
    // In a real ESP32-S2 implementation, this would use USB HID libraries
    // to move the mouse cursor slightly to wake the computer
    
    #[cfg(target_os = "espidf")]
    {
        // TODO: Implement actual USB HID mouse movement for ESP32-S2
        // This would require:
        // 1. Setting up USB HID device
        // 2. Sending mouse movement reports
        // 3. Small cursor movement (e.g., move by 1 pixel and back)
        info!("ESP32-S2 USB HID mouse movement (TODO: implement)");
    }
    
    #[cfg(not(target_os = "espidf"))]
    {
        info!("Mock mouse movement emulated");
    }
    
    Ok(())
}

/// UDP server that listens for wake commands
pub fn udp_server(port: u16, wake_requested: Arc<Mutex<bool>>) -> Result<()> {
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", port))?;
    socket.set_read_timeout(Some(Duration::from_secs(1)))?;
    
    info!("UDP server listening on port {}", port);
    
    let mut buf = [0; 64];
    
    loop {
        match socket.recv_from(&mut buf) {
            Ok((size, src)) => {
                info!("Received {} bytes from {}", size, src);
                
                // Simple wake command - any UDP packet triggers wake
                // In production, you might want to verify the packet content
                let mut wake_flag = wake_requested.lock().unwrap();
                *wake_flag = true;
                info!("Wake requested from {}", src);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Timeout, continue loop
                continue;
            }
            Err(e) => {
                warn!("UDP receive error: {}", e);
                thread::sleep(Duration::from_secs(1));
            }
        }
    }
}

fn main() -> Result<()> {
    // Initialize the logger
    #[cfg(target_os = "espidf")]
    {
        esp_idf_sys::link_patches();
        esp_idf_svc::log::EspLogger::initialize_default();
    }
    
    #[cfg(not(target_os = "espidf"))]
    {
        env_logger::init();
    }

    info!("ESP32-S2 Companion starting...");

    // Load configuration
    let config = load_config()?;
    
    // Check if Wi-Fi credentials are configured
    if config.wifi.ssid.is_empty() {
        error!("Wi-Fi SSID not configured!");
        error!("Please create config.json with Wi-Fi credentials (see config.json.example)");
        return Err(anyhow::anyhow!("Wi-Fi not configured"));
    }

    // Connect to Wi-Fi
    connect_wifi(&config.wifi)?;

    // Shared flag to signal wake requests
    let wake_requested = Arc::new(Mutex::new(false));
    let wake_requested_clone = wake_requested.clone();

    // Start UDP server in a separate thread
    let udp_port = config.wake.udp_port;
    thread::spawn(move || {
        if let Err(e) = udp_server(udp_port, wake_requested_clone) {
            error!("UDP server error: {}", e);
        }
    });

    info!("ESP32-S2 Companion ready!");
    info!("Listening for wake commands on UDP port {}", config.wake.udp_port);

    // Main loop - check for wake requests and execute them
    loop {
        {
            let mut wake_flag = wake_requested.lock().unwrap();
            if *wake_flag {
                if let Err(e) = wake_computer() {
                    error!("Failed to wake computer: {}", e);
                } else {
                    info!("Computer wake completed");
                }
                *wake_flag = false;
            }
        }
        
        thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.wake.udp_port, 9999);
        assert!(config.wifi.ssid.is_empty());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config {
            wifi: WifiConfig {
                ssid: "test_ssid".to_string(),
                password: "test_password".to_string(),
            },
            wake: WakeConfig {
                udp_port: 8888,
            },
        };
        
        let json = serde_json::to_string(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.wifi.ssid, "test_ssid");
        assert_eq!(parsed.wifi.password, "test_password");
        assert_eq!(parsed.wake.udp_port, 8888);
    }

    #[test]
    fn test_wake_computer_placeholder() {
        // Test that the wake function doesn't panic
        let result = wake_computer();
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_config_defaults() {
        // Test loading config when file doesn't exist
        let config = load_config().unwrap();
        assert_eq!(config.wake.udp_port, 9999);
    }
}
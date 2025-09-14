use alloc::string::{String, ToString};
use core::cell::RefCell;
use core::sync::atomic::{AtomicU16, Ordering};
use critical_section::Mutex;
use heapless::String as HString;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigCommand {
    SetWifi { ssid: String, pass: String },
    SetPort { port: u16 },
    Show,
}

#[derive(Deserialize)]
struct JsonCfg<'a> {
    #[serde(default)]
    ssid: Option<&'a str>,
    #[serde(default)]
    pass: Option<&'a str>,
    #[serde(default)]
    port: Option<u16>,
}

pub fn parse_json_config(s: &str) -> Result<ConfigCommand, ()> {
    let cfg: JsonCfg = serde_json::from_str(s).map_err(|_| ())?;
    if let (Some(ssid), Some(pass)) = (cfg.ssid, cfg.pass) {
        Ok(ConfigCommand::SetWifi {
            ssid: ssid.to_string(),
            pass: pass.to_string(),
        })
    } else if let Some(port) = cfg.port {
        Ok(ConfigCommand::SetPort { port })
    } else {
        Err(())
    }
}

pub fn parse_plain_config(s: &str) -> Option<ConfigCommand> {
    let s = s.trim();
    if s.eq_ignore_ascii_case("SHOW") {
        return Some(ConfigCommand::Show);
    }
    if s.len() >= 5 && s[..4].eq_ignore_ascii_case("PORT") {
        let rest = s[4..].trim();
        if let Ok(port) = rest.parse::<u16>() {
            return Some(ConfigCommand::SetPort { port });
        }
    }
    // WIFI <ssid> <pass>
    if s.len() >= 5 && s[..4].eq_ignore_ascii_case("WIFI") {
        let mut parts = s[4..].split_whitespace();
        if let (Some(ssid), Some(pass)) = (parts.next(), parts.next()) {
            return Some(ConfigCommand::SetWifi {
                ssid: ssid.to_string(),
                pass: pass.to_string(),
            });
        }
    }
    None
}

// Simple in-RAM persistence so runtime config works without NVS.
// Not retained across reset, but allows configuring once per boot via USB CDC.
static WIFI_MEM: Mutex<RefCell<(HString<64>, HString<64>)>> =
    Mutex::new(RefCell::new((HString::new(), HString::new())));
static PORT_MEM: AtomicU16 = AtomicU16::new(0);

pub fn save_wifi(ssid: &str, pass: &str) -> Result<(), ()> {
    critical_section::with(|cs| {
        let mut creds = WIFI_MEM.borrow_ref_mut(cs);
        creds.0.clear();
        creds.1.clear();
        let _ = creds.0.push_str(ssid);
        let _ = creds.1.push_str(pass);
    });
    Ok(())
}

pub fn load_wifi() -> Result<(String, String), ()> {
    let (ssid, pass) = critical_section::with(|cs| {
        let creds = WIFI_MEM.borrow_ref(cs);
        (creds.0.as_str().to_string(), creds.1.as_str().to_string())
    });
    Ok((ssid, pass))
}

pub fn save_port(port: u16) -> Result<(), ()> {
    PORT_MEM.store(port, Ordering::Relaxed);
    Ok(())
}

pub fn load_port() -> Result<Option<u16>, ()> {
    let p = PORT_MEM.load(Ordering::Relaxed);
    if p == 0 {
        Ok(None)
    } else {
        Ok(Some(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_wifi() {
        let c = parse_json_config("{\"ssid\":\"a\",\"pass\":\"b\"}").unwrap();
        assert!(matches!(c, ConfigCommand::SetWifi { .. }));
    }

    #[test]
    fn parse_json_port() {
        let c = parse_json_config("{\"port\":1234}").unwrap();
        assert!(matches!(c, ConfigCommand::SetPort{port} if port==1234));
    }

    #[test]
    fn parse_plain_wifi() {
        let c = parse_plain_config("WIFI myssid mypass").unwrap();
        assert!(matches!(c, ConfigCommand::SetWifi { .. }));
    }

    #[test]
    fn parse_plain_port() {
        let c = parse_plain_config("PORT 7777").unwrap();
        assert!(matches!(c, ConfigCommand::SetPort{port} if port==7777));
    }
}

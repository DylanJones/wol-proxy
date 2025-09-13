//! A simple program to intercept incoming TCP connections and send a
//! wake-on-lan packet to the real server, then transparently proxy once
//! the server has woken up.
use anyhow::{bail, Result};
use clap::Parser;
use ping_rs::PingOptions;
use std::{
    net::{IpAddr, SocketAddr, SocketAddrV4},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use tokio::net::{TcpListener, TcpStream};

#[derive(Parser)]
#[command(version, about = "Wake-on-LAN TCP proxy")]
struct Args {
    #[clap(short, long)]
    /// The MAC address of the server
    mac: String,

    #[clap(short, long)]
    /// The target address (ip:port) of the server
    target: String,

    #[clap(short, long)]
    /// The address to listen on
    bind: String,

    #[clap(long, default_value = "15")]
    /// Maximum time to wait for the server to wake up in seconds
    timeout: u64,
}

/// Wait for the target to come online, timing out after the given
/// timeout.
async fn ping(target: &IpAddr, timeout: Duration) -> bool {
    let ping_opts = PingOptions {
        ttl: 128,
        dont_fragment: true,
    };
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > timeout {
            return false;
        }
        if ping_rs::send_ping_async(
            target,
            Duration::from_secs(1),
            Arc::new(&[0u8; 0]),
            Some(&ping_opts),
        )
        .await
        .is_ok()
        {
            return true;
        }
    }
}

async fn handle_client(
    mut stream: TcpStream,
    target_addr: &SocketAddr,
    mac: &[u8; 6],
    timeout: u64,
) -> Result<()> {
    // Check if the server is already online, and skip WOL if it is:
    if !ping(&target_addr.ip(), Duration::from_secs(1)).await {
        // Send the wake-on-lan packet to the server
        let pkt = wake_on_lan::MagicPacket::new(mac);
        let sa_any = SocketAddr::from_str("[::]:0").unwrap();
        println!("Sending magic packet...");
        pkt.send_to(target_addr, &sa_any)?;

        // Wait for the server to wake up
        println!("Waiting for server to wake up...");
        if !ping(&target_addr.ip(), Duration::from_secs(timeout)).await {
            bail!("Server did not wake up in time");
        }
    }

    // Proxy the connection to the server
    println!("Proxying connection to server...");
    let mut server_conn = TcpStream::connect(target_addr).await?;
    tokio::io::copy_bidirectional(&mut server_conn, &mut stream).await?;

    // Done!
    Ok(())
}

/// Parse a MAC address into a [u8; 6]
fn parse_mac(mac: &str) -> Result<[u8; 6]> {
    // Check basic format: should be XX:XX:XX:XX:XX:XX (17 characters)
    if mac.len() != 17 {
        bail!("MAC address must be in format XX:XX:XX:XX:XX:XX");
    }

    // Split by colons and verify we have exactly 6 parts
    let parts: Vec<&str> = mac.split(':').collect();
    if parts.len() != 6 {
        bail!("MAC address must have exactly 6 hexadecimal parts separated by colons");
    }

    let mut out = [0u8; 6];
    for (i, part) in parts.iter().enumerate() {
        // Each part should be exactly 2 characters
        if part.len() != 2 {
            bail!("Each MAC address part must be exactly 2 hexadecimal characters");
        }

        // Parse as hexadecimal
        out[i] = u8::from_str_radix(part, 16).map_err(|_| {
            anyhow::anyhow!("Invalid hexadecimal characters in MAC address: {}", part)
        })?;
    }
    Ok(out)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();

    // parse mac address:
    let mac = parse_mac(&args.mac)?;

    // split target address into ip/port:
    let target_addr = SocketAddrV4::from_str(&args.target)?;

    let listener = TcpListener::bind(&args.bind).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            match handle_client(stream, &target_addr.into(), &mac, args.timeout).await {
                Ok(_) => {}
                Err(e) => eprintln!("client handling error: {}", e),
            };
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mac_valid() {
        // Test with valid MAC address
        let result = parse_mac("aa:bb:cc:dd:ee:ff");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
    }

    #[test]
    fn test_parse_mac_uppercase() {
        // Test with uppercase MAC address
        let result = parse_mac("AA:BB:CC:DD:EE:FF");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
    }

    #[test]
    fn test_parse_mac_mixed_case() {
        // Test with mixed case MAC address
        let result = parse_mac("aA:Bb:cC:Dd:eE:fF");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
    }

    #[test]
    fn test_parse_mac_zeros() {
        // Test with all zeros
        let result = parse_mac("00:00:00:00:00:00");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), [0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_parse_mac_max_values() {
        // Test with maximum hex values
        let result = parse_mac("ff:ff:ff:ff:ff:ff");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), [0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn test_parse_mac_invalid_too_short() {
        // Test with too short MAC address
        let result = parse_mac("aa:bb:cc:dd:ee");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_mac_invalid_too_long() {
        // Test with too long MAC address
        let result = parse_mac("aa:bb:cc:dd:ee:ff:gg");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_mac_invalid_chars() {
        // Test with invalid characters
        let result = parse_mac("zz:bb:cc:dd:ee:ff");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_mac_wrong_format() {
        // Test with wrong separator
        let result = parse_mac("aa-bb-cc-dd-ee-ff");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_mac_missing_separator() {
        // Test with missing separators
        let result = parse_mac("aabbccddeeff");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_mac_empty() {
        // Test with empty string
        let result = parse_mac("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_mac_single_digit() {
        // Test with single digit components
        let result = parse_mac("a:b:c:d:e:f");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_mac_with_extra_colons() {
        // Test with extra colons
        let result = parse_mac("aa:bb:cc:dd:ee:ff:");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_mac_three_digit_parts() {
        // Test with three digit parts
        let result = parse_mac("aaa:bb:cc:dd:ee:ff");
        assert!(result.is_err());
    }
}

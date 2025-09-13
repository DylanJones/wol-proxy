//! A simple program to intercept incoming TCP connections and send a
//! wake-on-lan packet to the real server, then transparently proxy once
//! the server has woken up.
#![allow(clippy::single_component_path_imports)]

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
use wake_on_lan;

#[derive(Parser)]
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
    let mut out = [0u8; 6];
    for i in 0..6 {
        out[i] = u8::from_str_radix(&mac[3 * i..(3 * i) + 2], 16)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    #[test]
    fn parse_mac_parses_bytes() {
        let mac = parse_mac("aa:bb:cc:dd:ee:ff").unwrap();
        assert_eq!(mac, [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
    }

    #[test]
    fn parse_mac_rejects_invalid() {
        assert!(parse_mac("not a mac").is_err());
    }

    #[tokio::test]
    async fn ping_localhost_succeeds() {
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(ping(&ip, Duration::from_secs(1)).await);
    }

    #[tokio::test]
    async fn handle_client_proxies_data() {
        // Start echo server that will act as the real target
        let echo = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let echo_addr = echo.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut stream, _) = echo.accept().await.unwrap();
            let mut buf = [0u8; 32];
            let n = stream.read(&mut buf).await.unwrap();
            stream.write_all(&buf[..n]).await.unwrap();
        });

        // Start listener representing the proxy and spawn handle_client
        let proxy = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_addr = proxy.local_addr().unwrap();
        tokio::spawn(async move {
            let (stream, _) = proxy.accept().await.unwrap();
            let mac = [0, 1, 2, 3, 4, 5];
            handle_client(stream, &echo_addr, &mac, 1).await.unwrap();
        });

        // Client connects to proxy and ensures data is echoed back
        let mut client = TcpStream::connect(proxy_addr).await.unwrap();
        client.write_all(b"ping").await.unwrap();
        let mut buf = [0u8; 4];
        client.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"ping");
    }
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

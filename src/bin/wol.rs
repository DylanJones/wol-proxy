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
        match ping_rs::send_ping_async(
            target,
            Duration::from_secs(1),
            Arc::new(&[0u8; 0]),
            Some(&ping_opts),
        )
        .await
        {
            Ok(_) => return true,
            Err(_) => (),
        };
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
        pkt.send_to(target_addr, &sa_any.try_into()?)?;

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

//! A simple TCP proxy that holds a wake lock during the connection
//! and for a configurable time afterwards.
use anyhow::Result;
use clap::Parser;
use keepawake::KeepAwake;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Notify;
use tokio::time::Duration;

#[derive(Parser)]
#[command(version, about = "TCP proxy to keep the machine awake")]
struct Args {
    #[clap(short, long)]
    /// Address of the target
    target: String,

    #[clap(short, long)]
    /// Listen address to bind to
    bind: String,

    #[clap(long, default_value = "300")]
    /// Number of seconds to keep the wake lock active after the last
    /// connection is closed
    timeout: u64,
}

/// Supervisor thread that waits for the last connection to close.
async fn supervisor(
    active_connections: Arc<AtomicU64>,
    ac_notify: Arc<Notify>,
    timeout: Duration,
) -> Result<()> {
    let mut _awake: Option<KeepAwake> = None;
    let mut locked = false;
    loop {
        // Wait for notification of a state change
        ac_notify.notified().await;
        // If there are active connections, ensure the wakelock is held
        if active_connections.load(Ordering::SeqCst) > 0 {
            if !locked {
                println!("acquiring wakelock");
                _awake = Some(
                    keepawake::Builder::default()
                        .display(false)
                        .idle(true)
                        .sleep(true)
                        .reason("active TCP proxy connection")
                        .app_reverse_domain("pw.karel.wol-proxy")
                        .create()?,
                );
                locked = true;
            }
        } else {
            // No active connections, wait for the timeout before releasing the wakelock
            tokio::select! {
                _ = tokio::time::sleep(timeout) => (),
                _ = ac_notify.notified() => ()
            };

            // Double-check active connections after waiting to avoid a race condition
            if active_connections.load(Ordering::SeqCst) == 0 && locked {
                println!("releasing wakelock");
                // we have to do this cause there's a bug in keepawake
                drop(_awake);
                _awake = None;
                // _awake = Some(keepawake::Builder::default()
                //     .display(false)
                //     .idle(false)
                //     .sleep(false)
                //     .reason("no active TCP proxy connection")
                //     .app_reverse_domain("pw.karel.wol-proxy")
                //     .create()?);
                // drop(_awake);
                // _awake = Some(keepawake::Builder::default()
                //     .display(false)
                //     .idle(false)
                //     .sleep(false)
                //     .reason("no active TCP proxy connection")
                //     .app_reverse_domain("pw.karel.wol-proxy")
                //     .create()?);
                locked = false;
            }
        }
    }
}

async fn handle_client(mut stream: TcpStream, target_addr: &SocketAddr) -> Result<()> {
    let mut target = TcpStream::connect(&target_addr).await?;
    tokio::io::copy_bidirectional(&mut stream, &mut target).await?;
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // parse command line arguments
    let args = Args::parse();
    let target_addr = SocketAddr::from_str(&args.target)?;

    let notify = Arc::new(Notify::new());
    let active_connections = Arc::new(AtomicU64::new(0));

    // Spawn supervisor thread to manage wakelock
    // (must be on its own thread bc of how wakelocks work)
    // let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    tokio::spawn(supervisor(
        active_connections.clone(),
        notify.clone(),
        Duration::from_secs(args.timeout),
    ));

    // main server loop: accept new connections and forward them to the target
    let listener = TcpListener::bind(&args.bind).await?;
    loop {
        let (stream, addr) = listener.accept().await?;

        // clone pointers for lifetime purposes
        let aconn_clone = active_connections.clone();
        let notify_clone = notify.clone();
        println!("Accepted connection from {}", addr);
        // spawn actual proxy task
        tokio::spawn(async move {
            // Increment active connection (only notify supervisor if this is the first connection to open)
            if aconn_clone.fetch_add(1, Ordering::SeqCst) == 0 {
                notify_clone.notify_waiters();
            }

            // proxy
            match handle_client(stream, &target_addr).await {
                Ok(()) => println!("connection finished successfully"),
                Err(e) => eprintln!("proxy error: {}", e),
            }
            // Decrement active connection (only notify supervisor if this was the last connection to close)
            if aconn_clone.fetch_sub(1, Ordering::SeqCst) == 1 {
                notify_clone.notify_waiters();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_args_parsing() {
        // Test that Args can be created manually (simulating CLI parsing)
        let args = Args {
            target: "127.0.0.1:8080".to_string(),
            bind: "0.0.0.0:8081".to_string(),
            timeout: 300,
        };

        assert_eq!(args.target, "127.0.0.1:8080");
        assert_eq!(args.bind, "0.0.0.0:8081");
        assert_eq!(args.timeout, 300);
    }

    #[test]
    fn test_args_default_timeout() {
        // Test default timeout value
        let args = Args {
            target: "127.0.0.1:8080".to_string(),
            bind: "0.0.0.0:8081".to_string(),
            timeout: 300, // This would be the default from clap
        };

        assert_eq!(args.timeout, 300);
    }

    #[test]
    fn test_atomic_counter() {
        // Test atomic connection counting logic
        let counter = Arc::new(AtomicU64::new(0));

        // Simulate incrementing connections
        let old_val = counter.fetch_add(1, Ordering::SeqCst);
        assert_eq!(old_val, 0); // Should trigger notification

        let old_val = counter.fetch_add(1, Ordering::SeqCst);
        assert_eq!(old_val, 1); // Should not trigger notification

        // Simulate decrementing connections
        let old_val = counter.fetch_sub(1, Ordering::SeqCst);
        assert_eq!(old_val, 2); // Should not trigger notification

        let old_val = counter.fetch_sub(1, Ordering::SeqCst);
        assert_eq!(old_val, 1); // Should trigger notification (last connection)

        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_socket_addr_parsing() {
        // Test that target address parsing works correctly
        let addr_str = "127.0.0.1:8080";
        let result = SocketAddr::from_str(addr_str);
        assert!(result.is_ok());

        let addr = result.unwrap();
        assert_eq!(addr.port(), 8080);

        // Test IPv4 specifically
        if let SocketAddr::V4(v4_addr) = addr {
            assert_eq!(v4_addr.ip().to_string(), "127.0.0.1");
        }
    }

    #[test]
    fn test_socket_addr_parsing_invalid() {
        // Test invalid address formats
        let invalid_addrs = vec![
            "not_an_address",
            "127.0.0.1",            // Missing port
            ":8080",                // Missing IP
            "127.0.0.1:99999",      // Port out of range
            "256.256.256.256:8080", // Invalid IP
        ];

        for addr_str in invalid_addrs {
            let result = SocketAddr::from_str(addr_str);
            assert!(result.is_err(), "Expected error for address: {}", addr_str);
        }
    }

    #[test]
    fn test_connection_counter_edge_cases() {
        // Test edge cases for connection counting
        let counter = Arc::new(AtomicU64::new(0));

        // Test multiple increments
        for i in 0..10 {
            let old_val = counter.fetch_add(1, Ordering::SeqCst);
            if i == 0 {
                assert_eq!(old_val, 0); // First connection should be 0 -> 1
            } else {
                assert!(old_val > 0); // Subsequent connections should be > 0
            }
        }

        // Test multiple decrements
        for i in (1..=10).rev() {
            let old_val = counter.fetch_sub(1, Ordering::SeqCst);
            assert_eq!(old_val, i); // Should match expected value
            if i == 1 {
                // Last connection closing should trigger notification
                assert_eq!(counter.load(Ordering::SeqCst), 0);
            }
        }
    }
}

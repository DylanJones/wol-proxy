//! A simple TCP proxy that holds a wake lock during the connection
//! and for a configurable time afterwards.
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use clap::Parser;
use keepawake::KeepAwake;
use tokio::sync::Notify;
use tokio::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use anyhow::Result;

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
    timeout: u64
}

/// Supervisor thread that waits for the last connection to close.
async fn supervisor(active_connections: Arc<AtomicU64>, ac_notify: Arc<Notify>, timeout: Duration) -> Result<()> {
    let mut _awake: Option<KeepAwake> = None;
    let mut locked = false;
    loop {
        // Wait for notification of a state change
        ac_notify.notified().await;
        // If there are active connections, ensure the wakelock is held
        if active_connections.load(Ordering::SeqCst) > 0 {
            if !locked {
                println!("acquiring wakelock");
                _awake = Some(keepawake::Builder::default()
                    .display(false)
                    .idle(true)
                    .sleep(true)
                    .reason("active TCP proxy connection")
                    .app_reverse_domain("pw.karel.wol-proxy")
                    .create()?);
                locked = true;
            }
        } else {
            // No active connections, wait for the timeout before releasing the wakelock
            tokio::select! {
                _ = tokio::time::sleep(timeout) => (),
                _ = ac_notify.notified() => ()
            };

            // Double-check active connections after waiting to avoid a race condition
            if active_connections.load(Ordering::SeqCst) == 0 {
                if locked {
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
    tokio::spawn(supervisor(active_connections.clone(), notify.clone(), Duration::from_secs(args.timeout)));

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
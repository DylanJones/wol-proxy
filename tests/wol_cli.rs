use std::net::TcpListener;
use std::process::{Command, Stdio};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn wol_cli_proxies_connections() {
    // Start echo server
    let echo_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let echo_port = echo_listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::from_std(echo_listener).unwrap();
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 32];
        let n = stream.read(&mut buf).await.unwrap();
        stream.write_all(&buf[..n]).await.unwrap();
    });

    // Reserve a port for the proxy to bind to
    let proxy_port = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        drop(l);
        port
    };

    // Spawn the wol binary
    let mut child = Command::new(env!("CARGO_BIN_EXE_wol"))
        .args([
            "--mac", "00:11:22:33:44:55",
            "--target", &format!("127.0.0.1:{}", echo_port),
            "--bind", &format!("127.0.0.1:{}", proxy_port),
            "--timeout", "1",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn wol");

    // Wait for the proxy to start accepting connections
    let mut client = loop {
        match TcpStream::connect(("127.0.0.1", proxy_port)).await {
            Ok(stream) => break stream,
            Err(_) => sleep(Duration::from_millis(50)).await,
        }
    };

    client.write_all(b"ping").await.unwrap();
    let mut buf = [0u8; 4];
    client.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"ping");

    let _ = child.kill();
    let _ = child.wait();
}

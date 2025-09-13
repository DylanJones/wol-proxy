# wol-proxy

A set of Rust programs to transparently wake up a high-power server on demand and manage system wake locks during active connections.

## Overview

This project consists of two complementary TCP proxy utilities:

- **`wol`** - A Wake-on-LAN TCP proxy that automatically wakes up sleeping servers when clients attempt to connect
- **`keepawake`** - A TCP proxy that prevents the local machine from sleeping while serving active connections

## Features

### wol (Wake-on-LAN Proxy)
- Intercepts incoming TCP connections to a target server
- Automatically sends Wake-on-LAN magic packets to sleeping servers
- Waits for the target server to wake up and become responsive
- Transparently proxies connections once the server is awake
- Configurable timeout for server wake-up process
- Supports IPv4 addresses and standard MAC address formats

### keepawake (System Wake Lock Proxy)
- Proxies TCP connections to a target server
- Maintains system wake locks to prevent sleep during active connections
- Configurable grace period after last connection closes
- Automatic wake lock management with connection counting
- Cross-platform sleep prevention

## Installation

### Prerequisites
- Rust 1.70+ (2021 edition)
- Network access for Wake-on-LAN functionality
- Administrator/root privileges may be required for wake lock functionality

### Building from Source

```bash
git clone https://github.com/DylanJones/wol-proxy.git
cd wol-proxy
cargo build --release
```

The compiled binaries will be available in `target/release/`:
- `target/release/wol`
- `target/release/keepawake`

### Installing with Cargo

```bash
cargo install --path .
```

## Usage

### wol - Wake-on-LAN Proxy

Wake up a server and proxy connections to it:

```bash
# Basic usage
wol --mac "00:11:22:33:44:55" --target "192.168.1.100:22" --bind "0.0.0.0:2222"

# With custom timeout (default: 15 seconds)
wol --mac "00:11:22:33:44:55" --target "192.168.1.100:3389" --bind "0.0.0.0:3389" --timeout 30
```

**Parameters:**
- `--mac, -m`: MAC address of the target server (format: "XX:XX:XX:XX:XX:XX")
- `--target, -t`: Target server address and port (format: "IP:PORT")
- `--bind, -b`: Local address to bind and listen on (format: "IP:PORT")
- `--timeout`: Maximum time to wait for server wake-up in seconds (default: 15)

**Example Use Cases:**
- Wake up a file server when accessing shares
- Wake up a game server when players try to connect
- Wake up a development server for remote work

### keepawake - System Wake Lock Proxy

Proxy connections while preventing system sleep:

```bash
# Basic usage
keepawake --target "192.168.1.100:22" --bind "0.0.0.0:2222"

# With custom timeout (default: 300 seconds / 5 minutes)
keepawake --target "localhost:3306" --bind "0.0.0.0:3306" --timeout 600
```

**Parameters:**
- `--target, -t`: Target server address and port (format: "IP:PORT")
- `--bind, -b`: Local address to bind and listen on (format: "IP:PORT")  
- `--timeout`: Seconds to keep wake lock after last connection closes (default: 300)

**Example Use Cases:**
- Keep laptop awake while serving database connections
- Prevent sleep during long-running SSH sessions
- Maintain system availability for remote access

## Configuration Examples

### Wake-on-LAN for SSH Access
```bash
# Wake up server and proxy SSH connections
wol --mac "aa:bb:cc:dd:ee:ff" --target "192.168.1.50:22" --bind "0.0.0.0:2222"

# Connect via: ssh user@localhost -p 2222
```

### Database Server with Wake Lock
```bash
# Proxy database connections and prevent sleep
keepawake --target "localhost:5432" --bind "0.0.0.0:5432" --timeout 1800
```

### Combined Setup
```bash
# Terminal 1: Wake up remote server
wol --mac "11:22:33:44:55:66" --target "192.168.1.200:3306" --bind "0.0.0.0:13306"

# Terminal 2: Keep local machine awake for serving
keepawake --target "localhost:8080" --bind "0.0.0.0:8080"
```

## Requirements

### Network Requirements
- Target servers must support Wake-on-LAN (WOL enabled in BIOS/UEFI)
- Network infrastructure must pass WOL magic packets
- ICMP ping must be allowed for server status checking

### System Requirements
- **Linux**: No additional requirements for wake locks
- **macOS**: May require Accessibility permissions for wake lock functionality
- **Windows**: May require running as Administrator for wake lock functionality

## Troubleshooting

### Wake-on-LAN Issues
- **Server doesn't wake up**: Verify WOL is enabled in BIOS and network card settings
- **Magic packet not sent**: Check network connectivity and MAC address format
- **Timeout errors**: Increase `--timeout` value for slower-starting servers

### Wake Lock Issues  
- **Permissions denied**: Run with elevated privileges (sudo/Administrator)
- **Wake lock not working**: Check system power management settings
- **High CPU usage**: Verify target server is accessible to avoid connection retry loops

### General Issues
- **Connection refused**: Verify target server address and port
- **Address already in use**: Choose a different bind port or stop conflicting services
- **Network unreachable**: Check network connectivity and firewall settings

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit issues, feature requests, or pull requests.

### Development

```bash
# Run tests
cargo test

# Check code formatting  
cargo fmt --check

# Run clippy for linting
cargo clippy

# Build documentation
cargo doc --open
```

# RemoteDesk

A lightweight, peer-to-peer remote desktop application built in Rust, designed as a streamlined alternative to AnyDesk with essential features and no central server dependency.

## Overview

RemoteDesk enables direct peer-to-peer remote desktop connections without requiring a central server for session management. It focuses on core functionality while maintaining security and performance.

**New to RemoteDesk?** Check out the [QUICKSTART.md](./QUICKSTART.md) guide for a beginner-friendly introduction.

## Core Features

### Implemented Features
- [ ] **9-Digit ID System**: Simple numeric ID generated on first launch
- [ ] **Remote Control**: Full keyboard and mouse control of remote desktop
- [ ] **Clipboard Sharing**: Bidirectional clipboard synchronization
- [ ] **Dual Authentication Modes**:
  - Manual Accept: Approve each connection explicitly (default)
  - Password Access: Auto-connect with password (optional)
- [ ] **Peer-to-Peer Architecture**: Direct connections without central server

### Key Characteristics

- **Lightweight**: Minimal resource footprint
- **Secure**: End-to-end encryption for all connections
- **Fast**: Direct P2P connections minimize latency
- **Cross-platform**: Works on Windows, Linux, and macOS
- **No Server Required**: Pure peer-to-peer architecture using NAT traversal

## Architecture

RemoteDesk uses a fully decentralized peer-to-peer architecture:

- **9-Digit IDs**: Each device gets a simple numeric ID (e.g., 123 456 789)
- **Direct Connections**: Peers connect directly to each other using their IDs
- **NAT Traversal**: Uses STUN/TURN for firewall/NAT penetration
- **Simple Authentication**: Choose between manual accept or password access
- **Session Management**: All session state managed locally by peers

See [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md) for detailed technical architecture.
See [docs/ID_AND_AUTH.md](./docs/ID_AND_AUTH.md) for ID and authentication details.

## Technology Stack

- **Language**: Rust
- **Networking**:
  - `tokio` - Async runtime
  - `webrtc` or `quinn` (QUIC) - P2P connections
  - `mdns` - Local network discovery
- **Screen Capture/Control**:
  - `scrap` - Cross-platform screen capture
  - `enigo` or `rdev` - Input simulation
- **Encryption**:
  - `rustls` - TLS implementation
  - `ring` - Cryptographic primitives
- **Compression**: `zstd` or `lz4` - Frame compression
- **Clipboard**: `arboard` - Cross-platform clipboard access

## Project Structure

```
remote-desk/
├── src/
│   ├── main.rs              # Application entry point
│   ├── network/             # P2P networking layer
│   │   ├── mod.rs
│   │   ├── connection.rs    # Connection management
│   │   ├── discovery.rs     # Peer discovery
│   │   ├── nat_traversal.rs # NAT/firewall traversal
│   │   └── protocol.rs      # Communication protocol
│   ├── desktop/             # Desktop capture and control
│   │   ├── mod.rs
│   │   ├── capture.rs       # Screen capture
│   │   ├── input.rs         # Keyboard/mouse input
│   │   └── display.rs       # Remote display rendering
│   ├── clipboard/           # Clipboard management
│   │   ├── mod.rs
│   │   └── sync.rs          # Clipboard synchronization
│   ├── security/            # Security and authentication
│   │   ├── mod.rs
│   │   ├── id.rs            # Device ID generation
│   │   ├── auth.rs          # Authentication
│   │   ├── encryption.rs    # Encryption handling
│   │   └── password.rs      # Password management
│   ├── ui/                  # User interface
│   │   ├── mod.rs
│   │   ├── tray.rs          # System tray
│   │   └── dialogs.rs       # Connection dialogs
│   └── config/              # Configuration management
│       ├── mod.rs
│       └── settings.rs      # User settings
├── docs/                    # Documentation
│   ├── ARCHITECTURE.md
│   ├── PROTOCOL.md
│   ├── SECURITY.md
│   ├── ROADMAP.md
│   ├── ID_AND_AUTH.md
│   └── CONTRIBUTING.md
├── tests/                   # Integration tests
└── Cargo.toml
```

## Getting Started

### Prerequisites

- Rust 1.70 or higher
- Platform-specific dependencies (see [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md))

### Building

```bash
# Clone the repository
git clone <repository-url>
cd remote-desk

# Build the project
cargo build --release

# Run
cargo run --release
```

### Usage

#### First Launch

On first launch, RemoteDesk will generate your unique 9-digit ID (e.g., **123 456 789**).
This ID is permanent and used by others to connect to your computer.

#### As Host (Receiving Connection)

**Option 1: Manual Accept (Default - Most Secure)**
1. Launch RemoteDesk
2. Share your 9-digit ID with the person who needs to connect
3. When they connect, you'll see a connection request
4. Review the details and click "Accept" or "Reject"

**Option 2: Password Access (Convenient for Regular Use)**
1. Launch RemoteDesk
2. Set a password in settings
3. Share your 9-digit ID and password
4. Connections with correct password will be accepted automatically

#### As Client (Connecting)

1. Launch RemoteDesk
2. Enter the host's 9-digit ID (e.g., 123 456 789)
3. If host uses password: Enter the password
4. Click "Connect"
5. If host uses manual accept: Wait for them to accept
6. Control the remote desktop

## Security

RemoteDesk implements multiple security layers:

- **9-Digit ID System**: Simple but secure identification (1 billion possible IDs)
- **Dual Authentication**: Choose between manual accept or password protection
- **End-to-End Encryption**: All data encrypted using TLS 1.3
- **Rate Limiting**: Protection against brute force attacks
- **Connection Logging**: Track all connection attempts

See [docs/SECURITY.md](./docs/SECURITY.md) for detailed security information.
See [docs/ID_AND_AUTH.md](./docs/ID_AND_AUTH.md) for authentication details.

## Development Roadmap

See [docs/ROADMAP.md](./docs/ROADMAP.md) for the detailed development plan and milestones.

## License

[Choose appropriate license - MIT, Apache 2.0, GPL, etc.]

## Contributing

Contributions are welcome! Please read [docs/CONTRIBUTING.md](./docs/CONTRIBUTING.md) before submitting pull requests.

## Disclaimer

This software is provided as-is for legitimate remote desktop access purposes. Users are responsible for ensuring compliance with applicable laws and regulations.

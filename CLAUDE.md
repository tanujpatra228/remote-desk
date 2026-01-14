# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

RemoteDesk is a peer-to-peer remote desktop application in Rust, designed as a lightweight alternative to AnyDesk. It uses direct P2P connections without a central server, featuring a 9-digit ID system for device identification.

## Build Commands

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo run --release      # Run the application
cargo test               # Run all tests
cargo test test_name     # Run single test
cargo check              # Type check without building
cargo fmt                # Format code
cargo clippy             # Lint
```

## Architecture

### Layer Structure (top to bottom)

1. **UI Layer** (`src/ui/`) - System tray, connection dialogs, overlays
2. **Application Layer** - Session manager, clipboard manager, config manager
3. **Desktop Layer** (`src/desktop/`) - Screen capture, input simulation, display rendering
4. **Security Layer** (`src/security/`) - Authentication, encryption (TLS 1.3), password management (Argon2id)
5. **Network Layer** (`src/network/`) - QUIC connections, mDNS discovery, NAT traversal (STUN/TURN)

### Key Design Decisions

- **Protocol**: Custom binary protocol over QUIC using `bincode` serialization
- **ID System**: 9-digit numeric IDs (100000000-999999999), stored in `~/.config/remotedesk/device_id`
- **Authentication Modes**: Manual accept (default) or password-based auto-accept
- **Frame Encoding**: Differential encoding with zstd compression, adaptive quality based on bandwidth
- **QUIC Streams**: Separate streams for control (0), video (1), input (2), clipboard (3), metadata (4)

### Message Types

Protocol messages are defined in `src/network/protocol.rs`:
- `0x00-0x0F`: Connection management
- `0x10-0x1F`: Authentication
- `0x20-0x3F`: Desktop control (frames, input events)
- `0x40-0x4F`: Clipboard
- `0x50-0x5F`: Metadata (quality, statistics)
- `0xF0-0xFF`: Errors

### Platform-Specific Implementations

- **Windows**: DXGI Desktop Duplication, SendInput API
- **Linux**: X11/XShm or Wayland/pipewire, XTest extension
- **macOS**: CoreGraphics, CGEvent API

## Configuration

Config files are stored at:
- Linux: `~/.config/remotedesk/`
- macOS: `~/Library/Application Support/RemoteDesk/`
- Windows: `%APPDATA%\RemoteDesk\`

## Key Dependencies

- `tokio` - Async runtime
- `quinn` - QUIC implementation
- `scrap` - Screen capture
- `rdev` - Input simulation
- `rustls`/`ring` - TLS and cryptography
- `arboard` - Clipboard
- `argon2` - Password hashing

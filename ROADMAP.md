# RemoteDesk Development Roadmap

## Overview

RemoteDesk is a lightweight peer-to-peer remote desktop application built in Rust. This document tracks the development progress and outlines future milestones.

---

## Phase 1: Foundation (Local Components)

### Milestone 1.1 - Project Setup & Core Foundation ✅
**Status: COMPLETED**

- [x] Project structure and Cargo.toml setup
- [x] Error handling framework (`src/error.rs`)
- [x] Configuration management (`src/config/`)
- [x] Logging infrastructure (`src/logging.rs`)
- [x] Device ID generation and validation (`src/security/device_id.rs`)

### Milestone 1.2 - Network Layer Implementation ✅
**Status: COMPLETED**

- [x] Protocol message definitions (`src/network/protocol.rs`)
- [x] Connection state management (`src/network/connection.rs`)
- [x] Connection manager (`src/network/manager.rs`)
- [x] Peer discovery structure (`src/network/discovery.rs`)
- [x] Interactive CLI with commands (connect, disconnect, status, etc.)
- [ ] Actual QUIC transport (deferred to Milestone 2.1)

### Milestone 1.3 - Security Layer ⚠️
**Status: PARTIAL**

Completed:
- [x] Password hashing with Argon2id (`src/security/password.rs`)
- [x] Device ID management
- [x] Basic authentication flow structure

Pending:
- [ ] TLS 1.3 certificate generation
- [ ] Encrypted stream setup
- [ ] Challenge-response authentication implementation

### Milestone 1.4 - Desktop Layer (Screen Capture) ✅
**Status: COMPLETED**

- [x] Screen capture using `scrap` crate (`src/desktop/capture.rs`)
- [x] Frame encoding with compression (`src/desktop/encoder.rs`)
- [x] Capture configuration (quality, FPS, region)
- [x] Cross-platform support (Linux X11/Wayland, Windows, macOS)

### Milestone 1.5 - Input Simulation ✅
**Status: COMPLETED**

- [x] Keyboard event simulation (`src/input/`)
- [x] Mouse event simulation (movement, clicks, scroll)
- [x] Input event types and mapping
- [x] Cross-platform support using `rdev`

---

## Milestone 1.6 - Session Management ✅
**Status: COMPLETED**

### Objective
Create a complete remote desktop session that integrates screen capture, input simulation, and network communication into a unified experience.

### Architecture

#### Transport Abstraction (Key Design Decision)
Communication is abstracted through async channels, enabling:
- **Loopback testing**: Host ↔ Client via in-memory channels
- **Future networking**: Replace channels with QUIC streams transparently

```
    HostSession                              ClientSession
    +-----------+     FrameChannel          +-----------+
    | Capture   |-------------------------->| Decode    |
    | Encode    |     (mpsc::Sender)        | Display   |
    | Simulate  |<--------------------------| Capture   |
    +-----------+     InputChannel          +-----------+
```

#### Session State Machine
```
Idle → Connecting → Authenticating → Active ⇄ Paused → Disconnecting → Disconnected
```

### Completed Implementation

#### Session Core (`src/session/`)
- [x] `SessionStateMachine` with validated state transitions (`state.rs`)
- [x] `SessionState` enum with all states
- [x] `HostSession` for screen sharing (`host.rs`)
- [x] `ClientSession` for viewing (`client.rs`)
- [x] `SessionManager` for coordinating sessions (`manager.rs`)
- [x] `SessionTransport` with channel-based abstraction (`transport.rs`)

#### Frame Pipeline
- [x] `FrameEncoder` - JPEG/PNG/Raw encoding (`desktop/encoder.rs`)
- [x] `FrameDecoder` - Decodes frames with statistics (`desktop/decoder.rs`)
- [x] `TransportFrame` - Serializable frame for transmission
- [x] Frame sequence numbering and ordering
- [x] Compression ratio tracking

#### Input Handling
- [x] `TransportInput` - Input events with coordinates
- [x] Input capture from viewer window (keyboard + mouse)
- [x] Coordinate translation (client window → remote screen)
- [x] Input simulation on host via `rdev`

#### Display Window
- [x] `ViewerWindow` - egui-based remote desktop viewer (`ui/viewer.rs`)
- [x] `StatusOverlay` - FPS, latency, bandwidth display (`ui/overlay.rs`)
- [x] Aspect ratio preservation
- [x] Mouse and keyboard input capture

#### Clipboard Integration
- [x] `ClipboardMonitor` - Detects clipboard changes (`clipboard/sync.rs`)
- [x] `ClipboardSync` - Bidirectional sync support
- [x] `ClipboardContent` - Text content with hash deduplication
- [x] Transport channel for clipboard data

### Files Created
```
src/session/
├── mod.rs           # Module exports (updated)
├── state.rs         # NEW: Session state machine
├── transport.rs     # NEW: Transport abstraction with channels
├── host.rs          # NEW: Host session implementation
├── client.rs        # NEW: Client session implementation
├── manager.rs       # NEW: Session manager
└── types.rs         # Existing session types

src/desktop/
├── decoder.rs       # NEW: Frame decoder with statistics

src/ui/
├── viewer.rs        # NEW: Remote desktop viewer window
├── overlay.rs       # NEW: Status overlay widget

src/clipboard/
├── sync.rs          # NEW: Clipboard synchronization

src/error.rs         # Updated: Added SessionError

examples/
├── loopback_demo.rs # NEW: Loopback demonstration

tests/
├── loopback_session.rs # NEW: Integration tests
```

### Success Criteria - All Met
- [x] Can capture local screen and display in a window (loopback test)
- [x] Can simulate input from viewer window to desktop
- [x] Frame rate configurable (default 30 FPS)
- [x] Input events processed in real-time
- [x] Session state machine with proper transitions

### Testing
```bash
# Run all tests
cargo test

# Run integration tests
cargo test --test loopback_session

# Run loopback demo (requires display)
cargo run --example loopback_demo
```

### Dependencies Added
- `arboard` - Clipboard (uncommented in Cargo.toml)

---

## Phase 2: Networking & P2P

## Milestone 2.1 - QUIC P2P Networking 🔜
**Status: PLANNED**

### Objective
Replace simulated connections with real peer-to-peer QUIC networking, enabling actual remote desktop connections between machines.

### Architecture

```
┌─────────────────┐         ┌─────────────────┐
│   Host Device   │         │  Client Device  │
│                 │         │                 │
│ ┌─────────────┐ │  QUIC   │ ┌─────────────┐ │
│ │   Session   │◄┼─────────┼─►   Session   │ │
│ └─────────────┘ │  Streams│ └─────────────┘ │
│                 │         │                 │
│ Device ID:      │         │ Device ID:      │
│ 621 301 222     │         │ 225 104 032     │
└─────────────────┘         └─────────────────┘
         │                           │
         └───────────┬───────────────┘
                     │
              ┌──────┴──────┐
              │ STUN Server │
              │ (NAT hole   │
              │  punching)  │
              └─────────────┘
```

### QUIC Stream Layout
| Stream ID | Purpose | Direction | Priority |
|-----------|---------|-----------|----------|
| 0 | Control (handshake, heartbeat) | Bidirectional | High |
| 1 | Video frames | Host → Client | Medium |
| 2 | Input events | Client → Host | High |
| 3 | Clipboard | Bidirectional | Low |
| 4 | Metadata (stats, quality) | Bidirectional | Low |

### Implementation Tasks

#### Task 2.1.1 - QUIC Transport (`src/network/transport.rs`)
- [ ] QUIC endpoint setup using `quinn`
- [ ] Self-signed certificate generation
- [ ] Connection establishment
- [ ] Stream multiplexing
- [ ] Connection keep-alive (heartbeat)
- [ ] Graceful disconnection

#### Task 2.1.2 - NAT Traversal (`src/network/nat.rs`)
- [ ] STUN client implementation
- [ ] Public IP/port discovery
- [ ] NAT hole punching
- [ ] Fallback to relay (TURN) - optional
- [ ] Connection candidates exchange

#### Task 2.1.3 - Peer Discovery Enhancement (`src/network/discovery.rs`)
- [ ] mDNS service advertisement
- [ ] mDNS service browsing
- [ ] Peer info caching
- [ ] Network change detection
- [ ] Manual IP:port connection option

#### Task 2.1.4 - Connection Handshake
```
Client                              Host
   │                                  │
   │──── ConnectionRequest ──────────►│
   │     (client_id, name, pwd_hash)  │
   │                                  │
   │◄─── AuthChallenge ──────────────│
   │     (nonce)                      │
   │                                  │
   │──── AuthResponse ───────────────►│
   │     (signed_nonce)               │
   │                                  │
   │◄─── ConnectionAccept ───────────│
   │     (session_id, capabilities)   │
   │                                  │
   │◄═══ Session Active ═════════════►│
```

#### Task 2.1.5 - Protocol Implementation
- [ ] Message serialization with `bincode`
- [ ] Message framing (length-prefixed)
- [ ] Message routing to handlers
- [ ] Error handling and recovery
- [ ] Protocol version negotiation

#### Task 2.1.6 - Connection Manager Updates
- [ ] Replace simulated connections with real QUIC
- [ ] Multiple simultaneous connections
- [ ] Connection quality monitoring
- [ ] Automatic reconnection
- [ ] Bandwidth estimation

### Files to Create/Modify
```
src/network/
├── mod.rs           # Update exports
├── transport.rs     # NEW: QUIC transport layer
├── nat.rs           # NEW: NAT traversal
├── discovery.rs     # Enhance: Real mDNS
├── connection.rs    # Enhance: Real connection handling
├── manager.rs       # Enhance: Real connection management
└── protocol.rs      # Enhance: Message handling
```

### Configuration
```toml
[network]
listen_port = 7070
enable_mdns = true
stun_servers = ["stun.l.google.com:19302"]
connection_timeout_secs = 30
heartbeat_interval_secs = 5
max_reconnect_attempts = 3
```

### Success Criteria
- [ ] Can discover peers on local network via mDNS
- [ ] Can connect to peer by Device ID
- [ ] Can connect through NAT (same network first)
- [ ] Connection survives brief network interruptions
- [ ] Latency under 50ms on local network
- [ ] Bandwidth adapts to network conditions

### Dependencies
- `quinn` - QUIC implementation
- `rustls` - TLS for QUIC
- `mdns-sd` - mDNS discovery
- `stun-client` or custom - STUN protocol

### Testing Plan
1. **Loopback Test**: Connect to self (localhost)
2. **LAN Test**: Two machines on same network
3. **NAT Test**: Two machines behind different NATs
4. **Stress Test**: Large screen, high FPS, extended duration

---

## Phase 3: Polish & Features (Future)

### Milestone 3.1 - UI Layer
- System tray integration
- Connection request dialogs
- Settings UI
- QR code scanning for easy pairing

### Milestone 3.2 - File Transfer
- Drag-and-drop file transfer
- Progress indicators
- Resume interrupted transfers

### Milestone 3.3 - Multi-Monitor Support
- Monitor selection
- Multi-monitor spanning
- Monitor switching during session

### Milestone 3.4 - Audio Streaming
- Audio capture
- Audio playback
- Audio codec (Opus)

### Milestone 3.5 - Performance Optimization
- Hardware encoding (NVENC, VAAPI)
- GPU-accelerated rendering
- Zero-copy frame handling

---

## Development Guidelines

### Build Commands
```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run
cargo run

# Run release
./target/release/remote-desk

# Test
cargo test

# Check (fast compile check)
cargo check
```

### Library Path Workaround
If `libxdo-dev` is not installed, the project includes a workaround:
```bash
# Symlink is at: target/lib/libxdo.so
# Config is at: .cargo/config.toml
```

### Code Style
- Follow Rust idioms and conventions
- Use `cargo fmt` before committing
- Use `cargo clippy` for linting
- Document public APIs

---

## Timeline Estimates

| Milestone | Estimated Effort | Dependencies |
|-----------|------------------|--------------|
| 1.6 Session Management | 2-3 weeks | 1.4, 1.5 |
| 2.1 QUIC Networking | 2-3 weeks | 1.6 |
| 1.3 Security Complete | 1 week | 2.1 |
| 3.x Features | Ongoing | 2.1 |

---

## References

- [QUIC RFC 9000](https://www.rfc-editor.org/rfc/rfc9000)
- [Quinn QUIC Library](https://github.com/quinn-rs/quinn)
- [mDNS-SD Crate](https://crates.io/crates/mdns-sd)
- [Scrap Screen Capture](https://crates.io/crates/scrap)
- [RDev Input](https://crates.io/crates/rdev)

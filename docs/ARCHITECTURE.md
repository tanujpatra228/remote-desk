# RemoteDesk Architecture

This document describes the technical architecture and design decisions for RemoteDesk.

## Overview

RemoteDesk is built as a peer-to-peer application with a decentralized architecture. The system is divided into several key modules that handle networking, desktop capture/control, security, and user interface.

## System Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        RemoteDesk                           │
├─────────────────────────────────────────────────────────────┤
│  UI Layer                                                   │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                │
│  │  Tray    │  │ Dialogs  │  │ Overlay  │                │
│  └──────────┘  └──────────┘  └──────────┘                │
├─────────────────────────────────────────────────────────────┤
│  Application Layer                                          │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐      │
│  │  Session    │  │  Clipboard   │  │   Config    │      │
│  │  Manager    │  │  Manager     │  │   Manager   │      │
│  └─────────────┘  └──────────────┘  └─────────────┘      │
├─────────────────────────────────────────────────────────────┤
│  Desktop Layer                                              │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐      │
│  │   Screen    │  │    Input     │  │   Display   │      │
│  │   Capture   │  │  Simulation  │  │   Render    │      │
│  └─────────────┘  └──────────────┘  └─────────────┘      │
├─────────────────────────────────────────────────────────────┤
│  Security Layer                                             │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐      │
│  │    Auth     │  │  Encryption  │  │  Password   │      │
│  └─────────────┘  └──────────────┘  └─────────────┘      │
├─────────────────────────────────────────────────────────────┤
│  Network Layer                                              │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐      │
│  │ Connection  │  │   Discovery  │  │     NAT     │      │
│  │   Manager   │  │              │  │  Traversal  │      │
│  └─────────────┘  └──────────────┘  └─────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

## Core Modules

### 1. Network Layer

#### Connection Management (`network/connection.rs`)

**Responsibilities:**
- Manage peer-to-peer connections
- Handle connection lifecycle (establish, maintain, close)
- Connection quality monitoring and adaptation
- Automatic reconnection on failure

**Key Components:**
- `Connection`: Represents a P2P connection
- `ConnectionPool`: Manages multiple connections
- `ConnectionState`: Tracks connection status

**Implementation Details:**
- Uses QUIC protocol for reliable, multiplexed connections
- Implements automatic bandwidth adaptation
- Supports multiple concurrent data streams per connection

#### Peer Discovery (`network/discovery.rs`)

**Responsibilities:**
- Discover peers on local network (mDNS)
- Generate and validate unique Peer IDs
- Maintain peer registry

**Key Components:**
- `PeerDiscovery`: Main discovery service
- `PeerId`: Unique peer identifier (based on public key fingerprint)
- `PeerInfo`: Peer metadata and connection info

**Discovery Methods:**
1. **Local Network**: mDNS/Bonjour for LAN discovery
2. **Direct Connection**: Manual IP:Port entry
3. **Peer ID**: Unique identifier for direct P2P connection

#### NAT Traversal (`network/nat_traversal.rs`)

**Responsibilities:**
- Traverse NAT/firewalls for P2P connections
- Implement STUN/TURN protocols
- ICE candidate gathering and negotiation

**Key Components:**
- `NatTraversal`: Main NAT traversal logic
- `StunClient`: STUN server communication
- `IceCandidate`: Network connectivity candidates

**Traversal Strategy:**
1. Try direct connection first
2. Use STUN for public endpoint discovery
3. Attempt UDP hole punching
4. Fall back to TURN relay if necessary

### 2. Desktop Layer

#### Screen Capture (`desktop/capture.rs`)

**Responsibilities:**
- Capture screen content efficiently
- Support multiple monitors
- Adaptive quality and frame rate

**Key Components:**
- `ScreenCapturer`: Main capture interface
- `CaptureConfig`: Capture settings (resolution, FPS, quality)
- `Frame`: Captured frame data

**Implementation Details:**
- Platform-specific capture APIs:
  - Windows: DXGI Desktop Duplication
  - Linux: X11/Wayland capture
  - macOS: CoreGraphics
- Differential encoding (only changed regions)
- Compression using zstd for efficient transmission

#### Input Simulation (`desktop/input.rs`)

**Responsibilities:**
- Simulate keyboard input
- Simulate mouse movement and clicks
- Handle special keys and combinations

**Key Components:**
- `InputSimulator`: Input injection interface
- `KeyEvent`: Keyboard event
- `MouseEvent`: Mouse event

**Security Considerations:**
- Validate all input events
- Respect system-level input restrictions
- Implement rate limiting to prevent abuse

#### Display Rendering (`desktop/display.rs`)

**Responsibilities:**
- Render remote desktop frames
- Handle scaling and aspect ratio
- Display cursor and annotations

**Key Components:**
- `DisplayRenderer`: Frame rendering
- `RenderConfig`: Display settings
- `Overlay`: UI overlay for connection info

### 3. Security Layer

#### Authentication (`security/auth.rs`)

**Responsibilities:**
- Authenticate peers before connection
- Manage authentication tokens
- Handle authentication challenges

**Key Components:**
- `AuthManager`: Authentication coordinator
- `AuthChallenge`: Challenge-response authentication
- `AuthToken`: Session authentication token

**Authentication Flow:**
1. Client requests connection with Peer ID
2. Host generates authentication challenge
3. Client responds with password-derived key
4. Host validates response
5. Establish encrypted session

#### Encryption (`security/encryption.rs`)

**Responsibilities:**
- End-to-end encryption for all data
- Key exchange and management
- Perfect forward secrecy

**Key Components:**
- `EncryptionManager`: Encryption coordinator
- `SessionKey`: Per-session encryption key
- `Cipher`: Encryption/decryption operations

**Encryption Details:**
- TLS 1.3 for transport encryption
- X25519 for key exchange
- ChaCha20-Poly1305 or AES-256-GCM for data encryption
- Unique session keys for each connection

#### Password Management (`security/password.rs`)

**Responsibilities:**
- Secure password storage
- Password hashing and verification
- Password strength validation

**Key Components:**
- `PasswordManager`: Password operations
- `HashedPassword`: Stored password hash

**Implementation:**
- Argon2id for password hashing
- Configurable complexity requirements
- Optional password rotation

### 4. Clipboard Layer

#### Clipboard Synchronization (`clipboard/sync.rs`)

**Responsibilities:**
- Monitor clipboard changes
- Synchronize clipboard across peers
- Handle different clipboard formats

**Key Components:**
- `ClipboardSync`: Synchronization manager
- `ClipboardContent`: Platform-agnostic clipboard data
- `ClipboardWatcher`: Monitor for changes

**Supported Formats:**
- Plain text
- Rich text (RTF/HTML)
- Images (PNG/JPEG)
- Files (metadata only, no automatic file transfer)

**Synchronization Strategy:**
- Event-based: Sync on clipboard change
- Debouncing to prevent rapid updates
- Size limits to prevent abuse (e.g., 10MB max)

### 5. UI Layer

#### System Tray (`ui/tray.rs`)

**Responsibilities:**
- System tray icon and menu
- Quick access to common actions
- Status indication

**Key Components:**
- `TrayIcon`: System tray integration
- `TrayMenu`: Context menu

**Features:**
- Connection status indicator
- Quick connect/disconnect
- Settings access

#### Dialogs (`ui/dialogs.rs`)

**Responsibilities:**
- Connection request dialogs
- Permission prompts
- Settings UI

**Key Components:**
- `ConnectionDialog`: Accept/reject connection requests
- `SettingsDialog`: Application settings
- `PasswordDialog`: Password entry

## Communication Protocol

### Protocol Overview

RemoteDesk uses a custom binary protocol over QUIC for efficient, reliable communication.

See [PROTOCOL.md](./PROTOCOL.md) for detailed protocol specification.

### Message Types

1. **Control Messages**: Connection management, authentication
2. **Desktop Messages**: Screen frames, input events
3. **Clipboard Messages**: Clipboard synchronization
4. **Metadata Messages**: Connection quality, statistics

### Data Flow

#### Screen Sharing Flow (Host → Client)

```
1. Capture screen frame
2. Detect changed regions (differential encoding)
3. Compress changed regions (zstd)
4. Encrypt frame data
5. Send over QUIC stream
6. Client receives and decrypts
7. Client decompresses and renders
```

#### Input Control Flow (Client → Host)

```
1. Client captures input event
2. Encrypt input event
3. Send over QUIC stream
4. Host receives and decrypts
5. Host validates input event
6. Host simulates input
```

## Performance Considerations

### Frame Rate Optimization

- Adaptive frame rate based on bandwidth (10-60 FPS)
- Motion detection for dynamic frame rate
- Reduced quality during high motion
- Full quality for static content

### Bandwidth Management

- Automatic quality adaptation
- Compression level adjustment
- Differential encoding to minimize data
- Bandwidth usage limits

### Latency Optimization

- QUIC for low-latency transport
- Direct P2P connections (no relay when possible)
- Input event prioritization
- Predictive cursor positioning

## Platform-Specific Considerations

### Windows

- DXGI Desktop Duplication for capture
- SendInput API for input simulation
- Windows registry for settings storage

### Linux

- X11 capture (XShm, XDamage)
- Wayland capture (pipewire)
- XTest extension for input simulation
- Multiple display server support

### macOS

- CoreGraphics for screen capture
- CGEvent API for input simulation
- Keychain for password storage
- System permissions handling

## Dependencies

### Core Dependencies

```toml
[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# Networking
quinn = "0.10"  # QUIC implementation
mdns-sd = "0.7"  # mDNS for local discovery

# Screen capture and input
scrap = "0.5"  # Screen capture
rdev = "0.5"   # Input simulation

# Clipboard
arboard = "3.2"  # Cross-platform clipboard

# Encryption
rustls = "0.21"
ring = "0.17"
x25519-dalek = "2"

# Compression
zstd = "0.13"

# Serialization
bincode = "1.3"
serde = { version = "1", features = ["derive"] }

# UI (system tray)
tray-icon = "0.9"

# Password hashing
argon2 = "0.5"

# Configuration
config = "0.13"
directories = "5"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"
```

## Configuration

### Configuration File Location

- Linux: `~/.config/remotedesk/config.toml`
- macOS: `~/Library/Application Support/RemoteDesk/config.toml`
- Windows: `%APPDATA%\RemoteDesk\config.toml`

### Configuration Options

```toml
[network]
listen_port = 0  # 0 for random port
enable_mdns = true
stun_servers = ["stun:stun.l.google.com:19302"]
max_connections = 5

[desktop]
default_quality = 80  # 0-100
default_fps = 30
compression_level = 3  # 0-22 for zstd

[security]
require_password = true
min_password_length = 8
session_timeout_minutes = 30

[clipboard]
enabled = true
max_size_mb = 10
sync_delay_ms = 500

[ui]
show_tray_icon = true
minimize_to_tray = true
```

## Error Handling

### Error Categories

1. **Network Errors**: Connection failures, timeouts
2. **Permission Errors**: Denied access to screen/input
3. **Authentication Errors**: Invalid password, rejected connection
4. **Resource Errors**: Insufficient memory, capture failure

### Error Recovery

- Automatic reconnection with exponential backoff
- Graceful degradation of quality
- User notification for critical errors
- Detailed logging for debugging

## Testing Strategy

### Unit Tests

- Test individual modules in isolation
- Mock external dependencies
- Focus on business logic

### Integration Tests

- Test module interactions
- Real network connections (localhost)
- End-to-end scenarios

### Performance Tests

- Benchmark frame encoding/decoding
- Measure latency under load
- Test bandwidth adaptation

### Security Tests

- Authentication bypass attempts
- Encryption validation
- Input validation testing

## Future Enhancements

- File transfer support
- Multi-monitor selection
- Audio streaming
- Mobile client support
- Connection quality indicators
- Chat functionality
- Session recording (with consent)
- Touch gesture support
- Annotation tools

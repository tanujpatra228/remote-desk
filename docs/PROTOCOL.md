# RemoteDesk Protocol Specification

This document defines the peer-to-peer communication protocol used by RemoteDesk.

## Overview

RemoteDesk uses a custom binary protocol layered over QUIC for secure, multiplexed, low-latency communication between peers.

## Protocol Stack

```
┌──────────────────────────────────┐
│   RemoteDesk Application Layer   │
├──────────────────────────────────┤
│    RemoteDesk Binary Protocol    │
├──────────────────────────────────┤
│            QUIC                  │
├──────────────────────────────────┤
│            UDP                   │
└──────────────────────────────────┘
```

## Connection Establishment

### 1. Peer Discovery

#### Local Network Discovery (mDNS)

```
Service Type: _remotedesk._udp.local
Instance Name: <9-digit-id>._remotedesk._udp.local

TXT Records:
- version=1
- id=<9-digit-id>         # e.g., 123456789
- name=<device-name>       # e.g., "John's Laptop"
- port=<udp-port>          # e.g., 7070
```

#### Direct Connection

Client manually enters:
- 9-digit ID (e.g., 123 456 789)
- Password (if required by host)
- IP address and port (optional, for direct connection)

### 2. Connection Handshake

**With Password Access:**
```
Client                          Host
  |                              |
  |-- ConnectionRequest -------->|
  |    (ID + Password Hash)      |
  |                              |
  |                          [Verify ID]
  |                          [Verify Password]
  |                              |
  |<---- ConnectionAccept -------|
  |      or ConnectionReject     |
  |                              |
  |<==== Encrypted Session =====>|
```

**With Manual Accept:**
```
Client                          Host
  |                              |
  |-- ConnectionRequest -------->|
  |    (ID only)                 |
  |                              |
  |                          [Verify ID]
  |                          [Show Dialog]
  |                          [User Accepts]
  |                              |
  |<---- ConnectionAccept -------|
  |      or ConnectionReject     |
  |                              |
  |<==== Encrypted Session =====>|
```

### 3. Session Establishment

After successful authentication, peers exchange:
- Supported features
- Desktop capabilities (resolution, capture method)
- Compression preferences
- Quality settings

## Message Format

### Base Message Structure

All messages use the following binary format:

```
┌────────────┬──────────────┬─────────────┬──────────────┐
│ Message ID │ Message Type │   Length    │   Payload    │
│  (4 bytes) │   (1 byte)   │  (4 bytes)  │  (variable)  │
└────────────┴──────────────┴─────────────┴──────────────┘
```

- **Message ID**: Unique identifier for request/response matching
- **Message Type**: Type of message (see below)
- **Length**: Payload length in bytes
- **Payload**: Message-specific data

### Message Types

```rust
pub enum MessageType {
    // Connection Management (0x00 - 0x0F)
    ConnectionRequest = 0x00,
    ConnectionAccept = 0x01,
    ConnectionReject = 0x02,
    Disconnect = 0x03,
    Heartbeat = 0x04,

    // Authentication (0x10 - 0x1F)
    AuthChallenge = 0x10,
    AuthResponse = 0x11,

    // Desktop Control (0x20 - 0x3F)
    ScreenFrame = 0x20,
    ScreenFrameDiff = 0x21,
    KeyboardEvent = 0x22,
    MouseEvent = 0x23,
    ResolutionChange = 0x24,

    // Clipboard (0x40 - 0x4F)
    ClipboardUpdate = 0x40,

    // Metadata (0x50 - 0x5F)
    QualityUpdate = 0x50,
    Statistics = 0x51,

    // Error (0xF0 - 0xFF)
    Error = 0xF0,
}
```

## Message Specifications

### Connection Management

#### ConnectionRequest (0x00)

Initiates connection to a peer.

**Payload:**
```rust
struct ConnectionRequest {
    protocol_version: u8,
    client_id: u32,             // Client's 9-digit ID
    client_name: String,         // Client device name
    host_id: u32,               // Host's 9-digit ID (to verify)
    password_hash: Option<[u8; 32]>,  // Password hash (if host requires password)
    requested_capabilities: Vec<Capability>,
}

enum Capability {
    RemoteControl,
    ClipboardSync,
    FileTransfer,
}
```

**Password Hashing:**
If the host requires password authentication:
```rust
// Client side
let password_hash = hash_password(&password, host_id);

fn hash_password(password: &str, host_id: u32) -> [u8; 32] {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(host_id.to_le_bytes());
    hasher.finalize().into()
}
```

#### ConnectionAccept (0x01)

Accepts an incoming connection.

**Payload:**
```rust
struct ConnectionAccept {
    session_id: [u8; 16],
    host_capabilities: Vec<Capability>,
    desktop_info: DesktopInfo,
}

struct DesktopInfo {
    screen_width: u16,
    screen_height: u16,
    screen_count: u8,
}
```

#### ConnectionReject (0x02)

Rejects an incoming connection.

**Payload:**
```rust
struct ConnectionReject {
    reason: RejectReason,
    message: Option<String>,
}

enum RejectReason {
    UserDenied,              // User clicked "Reject"
    InvalidPassword,         // Wrong password
    InvalidId,               // ID doesn't match
    AlreadyConnected,        // Already in a session
    AccountLocked,           // Too many failed attempts
    UnsupportedVersion,      // Protocol version mismatch
}
```

#### Disconnect (0x03)

Gracefully closes connection.

**Payload:**
```rust
struct Disconnect {
    reason: DisconnectReason,
}

enum DisconnectReason {
    UserInitiated,
    SessionTimeout,
    Error,
}
```

#### Heartbeat (0x04)

Keep-alive message.

**Payload:**
```rust
struct Heartbeat {
    timestamp: u64,  // Unix timestamp in milliseconds
}
```

### Authentication

Authentication is handled directly in the ConnectionRequest message.

**Manual Accept Mode:**
- Client sends ConnectionRequest with `password_hash = None`
- Host verifies the `host_id` matches its own ID
- Host displays connection dialog to user
- User clicks "Accept" or "Reject"
- Host sends ConnectionAccept or ConnectionReject

**Password Access Mode:**
- Client sends ConnectionRequest with `password_hash = Some(hash)`
- Host verifies the `host_id` matches its own ID
- Host verifies the password hash matches stored hash
- If valid, host sends ConnectionAccept automatically
- If invalid, host sends ConnectionReject with reason InvalidPassword
- Rate limiting prevents brute force attacks (see ID_AND_AUTH.md)

**Note:** The previous AuthChallenge/AuthResponse messages (0x10, 0x11) are reserved but not currently used. They may be implemented in the future for enhanced security.

### Desktop Control

#### ScreenFrame (0x20)

Full screen frame.

**Payload:**
```rust
struct ScreenFrame {
    frame_id: u32,
    timestamp: u64,
    width: u16,
    height: u16,
    format: ImageFormat,
    compression: CompressionType,
    data: Vec<u8>,
}

enum ImageFormat {
    RGB24,
    RGBA32,
    YUV420,
}

enum CompressionType {
    None,
    Zstd,
    Lz4,
}
```

#### ScreenFrameDiff (0x21)

Differential frame (only changed regions).

**Payload:**
```rust
struct ScreenFrameDiff {
    frame_id: u32,
    base_frame_id: u32,
    timestamp: u64,
    regions: Vec<ChangedRegion>,
}

struct ChangedRegion {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    format: ImageFormat,
    compression: CompressionType,
    data: Vec<u8>,
}
```

#### KeyboardEvent (0x22)

Keyboard input event.

**Payload:**
```rust
struct KeyboardEvent {
    timestamp: u64,
    event_type: KeyEventType,
    key_code: u32,
    modifiers: KeyModifiers,
    character: Option<char>,
}

enum KeyEventType {
    KeyDown,
    KeyUp,
}

bitflags! {
    struct KeyModifiers: u8 {
        const SHIFT = 0b00000001;
        const CTRL  = 0b00000010;
        const ALT   = 0b00000100;
        const META  = 0b00001000;
    }
}
```

#### MouseEvent (0x23)

Mouse input event.

**Payload:**
```rust
struct MouseEvent {
    timestamp: u64,
    event_type: MouseEventType,
    x: u16,
    y: u16,
    button: Option<MouseButton>,
    modifiers: KeyModifiers,
}

enum MouseEventType {
    Move,
    ButtonDown,
    ButtonUp,
    Scroll,
}

enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}
```

#### ResolutionChange (0x24)

Notifies of desktop resolution change.

**Payload:**
```rust
struct ResolutionChange {
    width: u16,
    height: u16,
    screen_count: u8,
}
```

### Clipboard

#### ClipboardUpdate (0x40)

Clipboard content synchronization.

**Payload:**
```rust
struct ClipboardUpdate {
    timestamp: u64,
    content_type: ClipboardContentType,
    data: Vec<u8>,
}

enum ClipboardContentType {
    Text,           // Plain text (UTF-8)
    RichText,       // HTML/RTF
    Image,          // PNG format
    FileList,       // File paths (not actual files)
}
```

### Metadata

#### QualityUpdate (0x50)

Updates quality settings during session.

**Payload:**
```rust
struct QualityUpdate {
    quality: u8,        // 0-100
    frame_rate: u8,     // Target FPS
    compression: u8,    // Compression level
}
```

#### Statistics (0x51)

Connection statistics for monitoring.

**Payload:**
```rust
struct Statistics {
    timestamp: u64,
    bytes_sent: u64,
    bytes_received: u64,
    frames_sent: u32,
    frames_received: u32,
    average_latency_ms: u32,
    packet_loss_rate: f32,
}
```

### Error

#### Error (0xF0)

Error notification.

**Payload:**
```rust
struct Error {
    error_code: ErrorCode,
    message: String,
}

enum ErrorCode {
    UnknownError = 0,
    ProtocolViolation = 1,
    UnsupportedMessage = 2,
    InvalidPayload = 3,
    PermissionDenied = 4,
    ResourceExhausted = 5,
}
```

## QUIC Streams

RemoteDesk uses multiple QUIC streams for different purposes:

### Stream Types

1. **Control Stream (Stream ID: 0)**
   - Connection management
   - Authentication
   - Configuration
   - Bidirectional, reliable

2. **Video Stream (Stream ID: 1)**
   - Screen frames
   - Unidirectional (host → client)
   - Semi-reliable (allow frame drops)

3. **Input Stream (Stream ID: 2)**
   - Keyboard/mouse events
   - Unidirectional (client → host)
   - Reliable, ordered

4. **Clipboard Stream (Stream ID: 3)**
   - Clipboard updates
   - Bidirectional
   - Reliable

5. **Metadata Stream (Stream ID: 4)**
   - Quality updates
   - Statistics
   - Bidirectional
   - Unreliable (datagram-style)

## Security Considerations

### Transport Security

- QUIC provides TLS 1.3 encryption
- Certificate-based peer authentication
- Perfect forward secrecy

### Application Security

- Password-based access control
- HMAC-based authentication
- Rate limiting on input events
- Input validation on all messages

### DoS Protection

- Connection rate limiting
- Maximum message size limits
- Heartbeat timeout for dead connections
- Maximum concurrent connections per peer

## Performance Optimizations

### Adaptive Frame Rate

```
Network Condition    Frame Rate    Quality
─────────────────────────────────────────────
Excellent (>10Mbps)  60 FPS        100%
Good (5-10Mbps)      30 FPS        80%
Fair (2-5Mbps)       20 FPS        60%
Poor (1-2Mbps)       10 FPS        40%
Very Poor (<1Mbps)   5 FPS         30%
```

### Frame Encoding Strategy

1. **Static Content**: High quality, low frame rate
2. **Motion Detected**: Lower quality, higher frame rate
3. **Text/UI**: Lossless compression for crisp text
4. **Video/Animation**: Lossy compression acceptable

### Differential Encoding

- Track previous frame
- Send only changed regions
- Minimum change threshold (avoid noise)
- Region merging for efficiency

## Protocol Versioning

Protocol version is negotiated during connection handshake.

**Current Version**: 1

**Version Compatibility:**
- Major version change: Breaking changes, not compatible
- Minor version change: Backward compatible additions

Version format: `MAJOR.MINOR`

## Implementation Notes

### Serialization

Use `bincode` for efficient binary serialization of Rust structures.

```rust
use serde::{Serialize, Deserialize};
use bincode;

let message = ScreenFrame { /* ... */ };
let encoded: Vec<u8> = bincode::serialize(&message)?;
let decoded: ScreenFrame = bincode::deserialize(&encoded)?;
```

### Error Handling

- All messages must be validated before processing
- Invalid messages should result in Error response
- Repeated protocol violations should terminate connection
- Log all protocol errors for debugging

### Flow Control

- Respect QUIC stream flow control
- Implement application-level backpressure
- Drop frames if client can't keep up (video stream)
- Never drop input events (use buffering)

### Timeout Values

```rust
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(30);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(15);
```

## Future Protocol Extensions

Potential additions to the protocol:

1. **File Transfer**: Dedicated stream for file transfers
2. **Audio Streaming**: Audio capture and playback
3. **Multi-Monitor**: Separate streams per monitor
4. **Annotations**: Drawing/pointer tools
5. **Chat**: Text chat alongside remote session
6. **Session Recording**: Recording capability
7. **Bandwidth Probing**: Active bandwidth measurement

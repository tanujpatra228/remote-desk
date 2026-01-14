# RemoteDesk System Overview

This document provides a high-level overview of how RemoteDesk works.

## How It Works

### The Big Picture

```
┌─────────────────┐                           ┌─────────────────┐
│   Your Device   │                           │  Remote Device  │
│                 │                           │                 │
│  ID: 123456789  │◄─────── Internet ───────►│  ID: 987654321  │
│                 │      (Encrypted P2P)      │                 │
│  You Control ───┼──────────────────────────►│  Their Screen  │
│  Their Screen   │                           │  Responds       │
└─────────────────┘                           └─────────────────┘
```

### Step-by-Step Connection Flow

```
1. First Launch (One Time)
   ┌──────────────┐
   │ RemoteDesk   │
   └──────┬───────┘
          │
          ├─► Generate 9-digit ID: 123456789
          │
          └─► Save ID permanently

2. When Someone Connects to You
   ┌──────────────┐                    ┌──────────────┐
   │   Client     │                    │     You      │
   │              │                    │ ID: 123456789│
   └──────┬───────┘                    └──────┬───────┘
          │                                   │
          │  "Connect to 123456789"           │
          ├──────────────────────────────────►│
          │                                   │
          │           ┌───────────────────────┤
          │           │ Password? ────────────┤─► Yes: Verify password
          │           │                       │   No:  Show dialog
          │           └───────────────────────┤
          │                                   │
          │◄──────── Accept/Reject ───────────┤
          │                                   │
          │◄═══════ Encrypted Session ═══════►│
          │                                   │

3. During Session
   ┌──────────────┐                    ┌──────────────┐
   │   Client     │                    │     Host     │
   │ (Sees Screen)│                    │(Being Viewed)│
   └──────┬───────┘                    └──────┬───────┘
          │                                   │
          │  Mouse Movement ──────────────────►│─► Simulate Input
          │  Keyboard Events ─────────────────►│─► Simulate Input
          │◄───────── Screen Frames ───────────┤─► Capture Screen
          │◄───────── Clipboard Data ──────────┤─► Sync Clipboard
          │                                   │
```

## Core Components

### 1. ID System

```
Device ID Generation
├── Random 9-digit number (100,000,000 - 999,999,999)
├── Stored locally: ~/.config/remotedesk/device_id
├── Never changes (unless manually reset)
└── Used by others to connect to you

Format: 123 456 789 (with spaces for readability)
Stored: 123456789 (without spaces)
```

### 2. Authentication System

```
Two Modes:

┌─────────────────────────────────────────────────────┐
│ Mode 1: Manual Accept (Default)                    │
├─────────────────────────────────────────────────────┤
│                                                     │
│  1. Client enters your ID                          │
│  2. You see notification                           │
│  3. You click "Accept" or "Reject"                 │
│  4. Connection established (if accepted)           │
│                                                     │
│  Security: Maximum (you control every connection)  │
│  Use case: Occasional help from different people   │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│ Mode 2: Password Access (Optional)                 │
├─────────────────────────────────────────────────────┤
│                                                     │
│  1. You set a password                             │
│  2. Client enters your ID + password               │
│  3. Connection established automatically           │
│  4. No manual accept needed                        │
│                                                     │
│  Security: High (password required)                │
│  Use case: Regular access from trusted users       │
└─────────────────────────────────────────────────────┘
```

### 3. Network Architecture

```
Peer-to-Peer Connection

┌──────────────┐                           ┌──────────────┐
│   Device A   │                           │   Device B   │
│  (Client)    │                           │    (Host)    │
└──────┬───────┘                           └──────┬───────┘
       │                                          │
       │  1. Discover via mDNS (same network)    │
       │     or ID lookup (internet)             │
       │                                          │
       │  2. NAT Traversal (STUN/TURN)           │
       │     - Find public IP                    │
       │     - Punch through firewall            │
       │                                          │
       │◄═══════════════════════════════════════►│
       │        Direct P2P Connection            │
       │          (TLS 1.3 Encrypted)            │
       │                                          │

No Central Server Required!
(except for NAT traversal assistance)
```

### 4. Data Flow

```
Remote Control Data Flow

┌─────────────────────────────────────────────────────┐
│                    Host Computer                    │
├─────────────────────────────────────────────────────┤
│                                                     │
│  Screen Capture ───► Compress ───► Encrypt ───┐   │
│      (30-60 FPS)      (Zstd)      (TLS 1.3)   │   │
│                                                 │   │
│  Input Simulation ◄── Validate ◄── Decrypt ◄───┤   │
│  (Mouse/Keyboard)                               │   │
│                                                 │   │
│  Clipboard Monitor ──► Sync ──────────────────►│   │
│                                                 │   │
└─────────────────────────────────────────────────┼───┘
                                                  │
                           Network (Internet)     │
                                                  │
┌─────────────────────────────────────────────────┼───┐
│                  Client Computer                │   │
├─────────────────────────────────────────────────┤   │
│                                                 │   │
│  Display Render ◄── Decompress ◄── Decrypt ◄───┘   │
│  (Show screen)       (Zstd)       (TLS 1.3)        │
│                                                     │
│  Input Capture ───► Encrypt ──────────────────────►│
│  (Mouse/Keyboard)   (TLS 1.3)                      │
│                                                     │
│  Clipboard Sync ◄──────────────────────────────────┤
│                                                     │
└─────────────────────────────────────────────────────┘
```

## Security Layers

```
Security Architecture (Defense in Depth)

┌────────────────────────────────────────────────────┐
│ Layer 5: User Control                             │
│ - Manual accept mode                              │
│ - User must approve each connection               │
├────────────────────────────────────────────────────┤
│ Layer 4: Application Security                     │
│ - Password authentication                         │
│ - Rate limiting (prevent brute force)             │
│ - Input validation                                │
├────────────────────────────────────────────────────┤
│ Layer 3: Session Security                         │
│ - Session tokens                                  │
│ - Timeout mechanisms                              │
│ - Connection logging                              │
├────────────────────────────────────────────────────┤
│ Layer 2: Encryption                               │
│ - TLS 1.3 (ChaCha20-Poly1305)                    │
│ - End-to-end encryption                           │
│ - Perfect forward secrecy                         │
├────────────────────────────────────────────────────┤
│ Layer 1: Transport Security                       │
│ - QUIC protocol                                   │
│ - Connection integrity                            │
│ - NAT traversal                                   │
└────────────────────────────────────────────────────┘
```

## Use Cases

### 1. Tech Support

```
Support Agent                  User Needing Help
     │                               │
     │  "What's your RemoteDesk ID?" │
     │◄──────────────────────────────┤
     │                               │
     │  "123 456 789"                │
     ├──────────────────────────────►│
     │                               │
     │  [Connects]                   │
     ├──────────────────────────────►│
     │                               │
     │         [User accepts]         │
     │◄──────────────────────────────┤
     │                               │
     │◄═══════ Connected ═══════════►│
     │                               │
     │  [Fixes problem]              │
     │                               │
     │  [Disconnects]                │
     │                               │
```

### 2. Remote Work

```
You at Home                   Your Office Computer
     │                               │
     │                          [ID: 987654321]
     │                          [Password: set]
     │                               │
     │  Connect to 987654321         │
     │  Password: ********           │
     ├──────────────────────────────►│
     │                               │
     │      [Auto-accepts]           │
     │◄═══════ Connected ═══════════►│
     │                               │
     │  [Work normally]              │
     │                               │
```

### 3. Help Family/Friends

```
You                           Mom's Computer
     │                               │
     │  "I need help with..."        │
     │◄──────────────────────────────┤
     │                               │
     │  "Open RemoteDesk"            │
     ├──────────────────────────────►│
     │                               │
     │  "What's your ID?"            │
     │◄──────────────────────────────┤
     │                               │
     │  "111 222 333"                │
     ├──────────────────────────────►│
     │                               │
     │  [Connects to 111222333]      │
     │◄═══════ Connected ═══════════►│
     │                               │
     │  [Shows her how to do X]      │
     │                               │
```

## Technology Stack Summary

```
┌────────────────────────────────────────────────────┐
│ Language: Rust                                     │
│ - Memory safe                                      │
│ - High performance                                 │
│ - Cross-platform                                   │
└────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────┐
│ Networking                                         │
│ - Protocol: QUIC (UDP-based)                      │
│ - Encryption: TLS 1.3                             │
│ - Discovery: mDNS                                 │
│ - NAT Traversal: STUN/TURN                        │
└────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────┐
│ Desktop Control                                    │
│ - Screen Capture: Platform-specific APIs          │
│   • Windows: DXGI                                 │
│   • Linux: X11/Wayland                            │
│   • macOS: CoreGraphics                           │
│ - Input Simulation: Cross-platform libraries      │
│ - Clipboard: Cross-platform clipboard access      │
└────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────┐
│ Performance                                        │
│ - Compression: Zstd                               │
│ - Differential Encoding: Only send changes        │
│ - Adaptive Quality: Adjust to bandwidth           │
│ - Frame Rate: 10-60 FPS (adaptive)                │
└────────────────────────────────────────────────────┘
```

## Performance Characteristics

### Bandwidth Usage

```
Typical Bandwidth (1920x1080 screen):

Static Content (desktop, documents):
├── High Quality:  2-5 Mbps
├── Medium Quality: 1-3 Mbps
└── Low Quality:   0.5-1 Mbps

Dynamic Content (video, animations):
├── High Quality:  10-20 Mbps
├── Medium Quality: 5-10 Mbps
└── Low Quality:   2-5 Mbps

Factors affecting bandwidth:
- Screen resolution
- Amount of motion
- Compression level
- Quality setting
```

### Latency

```
Expected Latency:

Local Network (LAN):
└── 10-50 ms (excellent)

Same City:
└── 20-80 ms (great)

Cross-Country:
└── 50-150 ms (good)

International:
└── 100-300 ms (acceptable)

Factors affecting latency:
- Network distance
- Internet speed
- Processing overhead
- Encryption overhead (minimal)
```

### Resource Usage

```
Typical Resource Usage:

CPU:
├── Host (capturing):  5-15%
└── Client (viewing):  3-10%

Memory:
├── Host: ~100-150 MB
└── Client: ~80-120 MB

Network:
├── Upload (host): 1-20 Mbps
└── Download (client): 1-20 Mbps
```

## Comparison with Alternatives

```
┌─────────────┬───────────┬──────────┬──────────────┐
│ Feature     │ RemoteDesk│ AnyDesk  │ TeamViewer   │
├─────────────┼───────────┼──────────┼──────────────┤
│ ID System   │ 9-digit   │ 10-digit │ 10-digit     │
│ Server      │ None (P2P)│ Central  │ Central      │
│ Privacy     │ High      │ Medium   │ Medium       │
│ Setup       │ Simple    │ Simple   │ Simple       │
│ Features    │ Core only │ Many     │ Many         │
│ Open Source │ Yes       │ No       │ No           │
│ Cost        │ Free      │ Paid     │ Paid         │
└─────────────┴───────────┴──────────┴──────────────┘
```

## System Requirements

```
Minimum Requirements:
├── OS: Windows 10, macOS 10.15, Linux (any modern distro)
├── RAM: 4 GB
├── CPU: Dual-core 1.5 GHz
├── Network: 1 Mbps
└── Screen: Any resolution

Recommended:
├── OS: Windows 11, macOS 12+, Linux (Ubuntu 22.04+)
├── RAM: 8 GB+
├── CPU: Quad-core 2.0 GHz+
├── Network: 10 Mbps+
└── Screen: 1920x1080 or higher
```

## Future Enhancements

See [ROADMAP.md](./ROADMAP.md) for detailed development plans.

**Planned Features:**
- File transfer
- Audio streaming
- Multi-monitor support
- Mobile apps (Android/iOS)
- Session recording
- Chat functionality
- Custom IDs (choose your own ID)
- QR code sharing
- Address book

## Conclusion

RemoteDesk provides a simple, secure, and private way to remotely control computers using:

1. **Simple 9-digit IDs** - Easy to remember and share
2. **Flexible authentication** - Manual accept or password access
3. **True P2P architecture** - No central server required
4. **Strong encryption** - TLS 1.3 end-to-end
5. **Cross-platform** - Works on all major operating systems

The focus is on essential features done well, with privacy and simplicity as core principles.

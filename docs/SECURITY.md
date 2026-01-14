# RemoteDesk Security Documentation

This document outlines the security considerations, implementations, and best practices for RemoteDesk.

## Security Philosophy

RemoteDesk is designed with security as a foundational principle:

1. **Defense in Depth**: Multiple layers of security
2. **Principle of Least Privilege**: Minimal permissions required
3. **Secure by Default**: Safe default settings
4. **Transparency**: Clear security indicators for users
5. **Privacy First**: No data collection, no telemetry to external servers

## Threat Model

### Assets to Protect

1. **Desktop Access**: Unauthorized remote control
2. **Clipboard Data**: Sensitive information in clipboard
3. **Network Traffic**: Screen content, input events, passwords
4. **Authentication Credentials**: Passwords, session keys
5. **User Privacy**: Activity, usage patterns

### Potential Threats

1. **Unauthorized Access**: Attacker gains remote control
2. **Man-in-the-Middle (MITM)**: Traffic interception
3. **Password Guessing**: Brute force attacks
4. **Denial of Service**: Resource exhaustion
5. **Input Injection**: Malicious input commands
6. **Eavesdropping**: Network traffic analysis
7. **Session Hijacking**: Taking over active session

### Adversary Capabilities

- **Network Attacker**: Can observe and modify network traffic
- **Local Attacker**: Has access to same LAN
- **Malicious Peer**: Compromised or malicious client/host

## Security Architecture

### Layered Security Model

```
┌─────────────────────────────────────────────┐
│  Application Security Layer                 │
│  - Input validation                         │
│  - Rate limiting                            │
│  - Session management                       │
├─────────────────────────────────────────────┤
│  Authentication & Authorization Layer       │
│  - Password-based auth                      │
│  - Manual accept                            │
│  - Session tokens                           │
├─────────────────────────────────────────────┤
│  Encryption Layer                           │
│  - TLS 1.3 (via QUIC)                      │
│  - End-to-end encryption                    │
│  - Perfect forward secrecy                  │
├─────────────────────────────────────────────┤
│  Transport Security Layer                   │
│  - QUIC protocol                            │
│  - Certificate verification                 │
│  - Connection integrity                     │
└─────────────────────────────────────────────┘
```

## Authentication

### Password-Based Authentication

#### Password Requirements

**Default Requirements:**
- Minimum length: 8 characters
- Recommended: 12+ characters
- No complexity requirements (length is more important)
- Maximum length: 128 characters

**Configurable Options:**
```toml
[security]
min_password_length = 8
require_password = true
max_login_attempts = 3
lockout_duration_minutes = 15
```

#### Password Storage

Passwords are NEVER stored in plain text.

**Host Side:**
```rust
// Password hashing using Argon2id
let config = argon2::Config {
    variant: argon2::Variant::Argon2id,
    version: argon2::Version::Version13,
    mem_cost: 65536,      // 64 MB
    time_cost: 3,         // 3 iterations
    lanes: 4,             // 4 parallel threads
    thread_mode: argon2::ThreadMode::Parallel,
    secret: &[],
    ad: &[],
    hash_length: 32
};

let password_hash = argon2::hash_encoded(
    password.as_bytes(),
    &salt,
    &config
)?;
```

**Storage Location:**
- Linux: `~/.config/remotedesk/password.hash`
- macOS: Keychain
- Windows: Credential Manager

#### Authentication Protocol

**Challenge-Response Flow:**

```
1. Client → Host: ConnectionRequest(peer_id, client_name)
2. Host generates: nonce (32 bytes), salt (16 bytes)
3. Host → Client: AuthChallenge(nonce, salt)
4. Client derives: password_hash = Argon2id(password, salt)
5. Client computes: response = HMAC-SHA256(password_hash, nonce)
6. Client → Host: AuthResponse(response)
7. Host verifies: expected_response = HMAC-SHA256(stored_hash, nonce)
8. Host → Client: ConnectionAccept or ConnectionReject
```

**Security Properties:**
- Password never transmitted over network
- Nonce prevents replay attacks
- HMAC prevents forgery
- Salt prevents rainbow table attacks

### Manual Connection Accept

Every connection requires explicit user approval:

1. **Connection Request Notification**: Visual and audible alert
2. **Connection Details**: Show peer ID, client name, IP address
3. **Accept/Reject**: User must click to accept
4. **Timeout**: Auto-reject after 60 seconds
5. **Remember Decision**: Optional (not recommended for security)

## Encryption

### Transport Encryption (QUIC + TLS 1.3)

**Encryption Suite:**
- Key Exchange: X25519 (ECDH)
- Signature: Ed25519
- Cipher: ChaCha20-Poly1305 or AES-256-GCM
- Hash: SHA-256

**TLS 1.3 Benefits:**
- 0-RTT connection establishment (after first connection)
- Perfect forward secrecy
- No vulnerable legacy cipher suites
- Encrypted handshake

### Certificate Management

**Self-Signed Certificates:**
RemoteDesk uses self-signed certificates for peer identity.

```rust
// Certificate generation
let cert = rcgen::Certificate::from_params({
    let mut params = rcgen::CertificateParams::new(vec!["remotedesk".to_string()]);
    params.distinguished_name = rcgen::DistinguishedName::new();
    params.key_pair = Some(rcgen::KeyPair::generate(&rcgen::PKCS_ED25519)?);
    params.alg = &rcgen::PKCS_ED25519;
    params
})?;
```

**Certificate Pinning:**
- First connection: User must accept certificate fingerprint
- Subsequent connections: Verify against pinned certificate
- Changed certificate: Alert user, require re-acceptance

**Peer ID:**
Peer ID is derived from certificate public key:
```rust
peer_id = SHA256(public_key)[0..32]
```

### Data Encryption

**Screen Frames:**
- Encrypted via QUIC stream encryption
- Additional application-level encryption optional

**Input Events:**
- Encrypted via QUIC stream encryption
- Signed to prevent tampering

**Clipboard Data:**
- Encrypted in transit
- Not stored persistently

## Authorization

### Connection Authorization

**Two-Factor Authorization:**
1. Password authentication (something you know)
2. Manual accept (something you do)

**Session Tokens:**
After successful authentication:
```rust
struct SessionToken {
    session_id: [u8; 16],
    peer_id: [u8; 32],
    issued_at: u64,
    expires_at: u64,
}
```

**Token Validation:**
- Verify session_id matches current session
- Check expiration time
- Validate peer_id matches connected peer

### Permission Model

**Host Permissions:**
- Accept/reject connections
- Disconnect active sessions
- View connected peers
- Change password

**Client Permissions (after authorization):**
- Control keyboard/mouse
- View screen
- Sync clipboard
- Disconnect

## Input Validation

### Input Event Validation

All received input events are validated:

```rust
fn validate_mouse_event(event: &MouseEvent, screen: &Screen) -> Result<()> {
    // Validate coordinates within screen bounds
    if event.x > screen.width || event.y > screen.height {
        return Err(SecurityError::InvalidInput);
    }

    // Validate button is valid
    match event.button {
        Some(MouseButton::Left | MouseButton::Right | MouseButton::Middle) => Ok(()),
        _ => Err(SecurityError::InvalidInput)
    }
}

fn validate_keyboard_event(event: &KeyboardEvent) -> Result<()> {
    // Validate key code is within valid range
    if event.key_code > MAX_KEY_CODE {
        return Err(SecurityError::InvalidInput);
    }

    // Rate limit to prevent input flooding
    if !rate_limiter.check() {
        return Err(SecurityError::RateLimitExceeded);
    }

    Ok(())
}
```

### Message Validation

All protocol messages are validated:

```rust
fn validate_message(msg: &Message) -> Result<()> {
    // Check message size limits
    if msg.payload.len() > MAX_MESSAGE_SIZE {
        return Err(SecurityError::MessageTooLarge);
    }

    // Validate message type is known
    if !is_valid_message_type(msg.message_type) {
        return Err(SecurityError::UnknownMessageType);
    }

    // Deserialize and validate payload
    validate_payload(msg.message_type, &msg.payload)?;

    Ok(())
}
```

## Rate Limiting

### Connection Rate Limiting

Prevent connection flooding:

```rust
const MAX_CONNECTION_ATTEMPTS: u32 = 5;
const CONNECTION_WINDOW: Duration = Duration::from_secs(60);

// Per IP address rate limiting
struct ConnectionLimiter {
    attempts: HashMap<IpAddr, VecDeque<Instant>>,
}

impl ConnectionLimiter {
    fn check(&mut self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let attempts = self.attempts.entry(ip).or_default();

        // Remove old attempts
        attempts.retain(|&time| now.duration_since(time) < CONNECTION_WINDOW);

        // Check limit
        if attempts.len() >= MAX_CONNECTION_ATTEMPTS {
            return false;
        }

        attempts.push_back(now);
        true
    }
}
```

### Input Rate Limiting

Prevent input event flooding:

```rust
const MAX_INPUT_EVENTS_PER_SECOND: u32 = 100;

struct InputRateLimiter {
    token_bucket: TokenBucket,
}

impl InputRateLimiter {
    fn check(&mut self) -> bool {
        self.token_bucket.take(1)
    }
}
```

### Authentication Rate Limiting

Prevent brute force attacks:

```rust
const MAX_AUTH_ATTEMPTS: u32 = 3;
const LOCKOUT_DURATION: Duration = Duration::from_secs(900); // 15 minutes

struct AuthLimiter {
    failed_attempts: HashMap<PeerId, (u32, Instant)>,
}

impl AuthLimiter {
    fn check_and_record(&mut self, peer_id: PeerId, success: bool) -> Result<()> {
        if let Some((attempts, locked_until)) = self.failed_attempts.get(&peer_id) {
            if Instant::now() < *locked_until {
                return Err(SecurityError::AccountLocked);
            }
        }

        if !success {
            let entry = self.failed_attempts.entry(peer_id).or_insert((0, Instant::now()));
            entry.0 += 1;

            if entry.0 >= MAX_AUTH_ATTEMPTS {
                entry.1 = Instant::now() + LOCKOUT_DURATION;
                return Err(SecurityError::AccountLocked);
            }
        } else {
            self.failed_attempts.remove(&peer_id);
        }

        Ok(())
    }
}
```

## Session Security

### Session Management

**Session Lifecycle:**
1. **Creation**: After successful authentication
2. **Active**: During remote control session
3. **Idle Timeout**: No activity for configured duration
4. **Termination**: Manual disconnect or timeout

**Session Timeout:**
```toml
[security]
session_timeout_minutes = 30
idle_timeout_minutes = 10
```

**Session Monitoring:**
```rust
struct Session {
    id: SessionId,
    peer_id: PeerId,
    created_at: Instant,
    last_activity: Instant,

    fn check_timeout(&self) -> Result<()> {
        let now = Instant::now();

        if now.duration_since(self.created_at) > SESSION_TIMEOUT {
            return Err(SessionError::SessionExpired);
        }

        if now.duration_since(self.last_activity) > IDLE_TIMEOUT {
            return Err(SessionError::IdleTimeout);
        }

        Ok(())
    }
}
```

### Secure Session Termination

On disconnect:
1. Close all QUIC streams
2. Destroy session keys
3. Clear frame buffers
4. Clear clipboard cache
5. Reset input state
6. Log disconnection event

## Privacy

### Data Minimization

RemoteDesk collects minimal data:

**NOT Collected:**
- User activity logs
- Screen content (except during active session)
- Clipboard history
- Usage statistics
- Telemetry data
- Crash reports (unless explicitly sent by user)

**Collected (Local Only):**
- Connection history (peer ID, timestamp, duration)
- Failed authentication attempts
- Configuration settings

### Data Retention

**Connection History:**
- Retained for 30 days by default
- User can clear at any time
- Configurable retention period

**Logs:**
- Debug logs disabled by default
- If enabled, automatically rotated and purged
- No sensitive data in logs (passwords, screen content)

## Secure Defaults

Default configuration prioritizes security:

```toml
[security]
# Authentication
require_password = true
min_password_length = 8
max_login_attempts = 3
lockout_duration_minutes = 15

# Session
session_timeout_minutes = 30
idle_timeout_minutes = 10
require_manual_accept = true

# Network
allow_connections_from_internet = false  # LAN only by default
max_concurrent_connections = 1

# Clipboard
clipboard_enabled = true
clipboard_max_size_mb = 10

# Logging
debug_logging = false
log_retention_days = 7
```

## Platform-Specific Security

### Windows

**Required Permissions:**
- Screen capture: None (DXGI)
- Input simulation: Running as user (no admin required)

**Security Features:**
- UAC prompt if elevation needed
- Windows Defender exclusion may be needed
- Firewall exception required for network access

### Linux

**Required Permissions:**
- Screen capture: X11 access or Wayland portal
- Input simulation: XTest extension or libei

**Security Features:**
- Wayland permission dialogs
- SELinux/AppArmor policies
- Firewall configuration (ufw/firewalld)

### macOS

**Required Permissions:**
- Screen Recording: System Preferences → Security & Privacy
- Accessibility: For input simulation
- Network: Incoming connections

**Security Features:**
- Gatekeeper approval
- Notarization required for distribution
- Keychain for credential storage

## Security Monitoring

### Logging

**Security Events Logged:**
- Connection attempts (success/failure)
- Authentication failures
- Session creation/termination
- Permission denials
- Rate limit violations
- Protocol errors

**Log Format:**
```
[TIMESTAMP] [LEVEL] [EVENT] [DETAILS]
2024-01-15 10:30:45 WARN AUTH_FAILED peer=abc123 ip=192.168.1.100 attempts=2
2024-01-15 10:31:00 INFO CONNECTION_ACCEPTED peer=abc123 ip=192.168.1.100
```

### Intrusion Detection

**Anomaly Detection:**
- Multiple failed authentication attempts
- Rapid connection attempts
- Excessive input rate
- Invalid protocol messages
- Unexpected disconnections

**Automatic Response:**
- Temporary IP blocking
- Account lockout
- Connection termination
- User notification

## Incident Response

### Security Incident Handling

1. **Detection**: Identify suspicious activity
2. **Containment**: Disconnect affected sessions
3. **Investigation**: Review logs and connection history
4. **Remediation**: Block attacker, reset credentials
5. **Recovery**: Resume normal operation
6. **Lessons Learned**: Update security measures

### User Actions

**If Unauthorized Access Suspected:**
1. Immediately disconnect all sessions
2. Change password
3. Review connection history
4. Check for suspicious file changes
5. Consider network-level blocking

## Security Best Practices for Users

### For Hosts (Receiving Connections)

1. **Use Strong Passwords**: 12+ characters, unique
2. **Accept Only Known Connections**: Verify peer identity
3. **Enable Session Timeout**: Don't leave sessions open
4. **Monitor Active Connections**: Check who's connected
5. **Update Regularly**: Install security updates
6. **Limit Network Exposure**: LAN only when possible

### For Clients (Connecting)

1. **Verify Host Identity**: Confirm peer ID out-of-band
2. **Use Secure Networks**: Avoid public Wi-Fi
3. **Disconnect When Done**: Don't leave sessions open
4. **Protect Password**: Don't share or write down
5. **Be Cautious**: Only connect to trusted hosts

## Vulnerability Disclosure

If you discover a security vulnerability:

1. **Do NOT** disclose publicly immediately
2. Report to: security@remotedesk.example (replace with actual contact)
3. Provide detailed description and steps to reproduce
4. Allow reasonable time for fix (e.g., 90 days)
5. Coordinate disclosure timing

## Security Audits

Planned security measures:

- [ ] Third-party security audit before 1.0 release
- [ ] Penetration testing
- [ ] Fuzzing of protocol implementation
- [ ] Static analysis (clippy, cargo-audit)
- [ ] Dependency vulnerability scanning

## Compliance

RemoteDesk aims to comply with:

- General security best practices
- OWASP guidelines for secure coding
- Rust security guidelines
- Platform-specific security requirements

## Security Updates

Security updates will be:

- Released promptly for critical vulnerabilities
- Clearly marked as security updates
- Documented in changelog with CVE numbers if applicable
- Announced to users

## Limitations

RemoteDesk provides security for remote desktop access, but:

- Cannot protect against compromised endpoints
- Cannot prevent social engineering attacks
- Cannot protect if attacker has physical access
- Relies on platform security (OS, hardware)
- Security is only as strong as the password chosen

## Conclusion

Security is a continuous process. This document will be updated as new threats emerge and new security features are implemented. User feedback and security research are welcome to improve RemoteDesk's security posture.

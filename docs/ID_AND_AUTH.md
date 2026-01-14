# RemoteDesk ID and Authentication System

This document describes the ID generation and authentication system for RemoteDesk.

## Overview

RemoteDesk uses a simple, user-friendly authentication system based on 9-digit numeric IDs with optional password protection.

## 9-Digit ID System

### ID Generation

On first launch, RemoteDesk generates a unique 9-digit numeric ID for the device.

**Format:**
- 9 decimal digits (000000000 - 999999999)
- Example: `123 456 789` (displayed with spaces for readability)
- Stored format: `123456789` (no spaces)

**Generation Algorithm:**

```rust
use rand::Rng;
use std::fs;
use std::path::PathBuf;

const ID_LENGTH: usize = 9;
const ID_MIN: u32 = 100_000_000;
const ID_MAX: u32 = 999_999_999;

/// Generates a random 9-digit ID
fn generate_id() -> u32 {
    let mut rng = rand::thread_rng();
    rng.gen_range(ID_MIN..=ID_MAX)
}

/// Gets or creates the device ID
fn get_or_create_device_id() -> Result<u32> {
    let config_dir = get_config_dir()?;
    let id_file = config_dir.join("device_id");

    if id_file.exists() {
        // Load existing ID
        let id_str = fs::read_to_string(&id_file)?;
        let id: u32 = id_str.trim().parse()?;
        Ok(id)
    } else {
        // Generate new ID
        let id = generate_id();
        fs::create_dir_all(&config_dir)?;
        fs::write(&id_file, id.to_string())?;
        Ok(id)
    }
}

/// Formats ID for display (with spaces)
fn format_id(id: u32) -> String {
    let id_str = format!("{:09}", id);
    format!("{} {} {}",
        &id_str[0..3],
        &id_str[3..6],
        &id_str[6..9]
    )
}
```

**Storage Location:**
- Linux: `~/.config/remotedesk/device_id`
- macOS: `~/Library/Application Support/RemoteDesk/device_id`
- Windows: `%APPDATA%\RemoteDesk\device_id`

**ID Properties:**
- Permanent: Never changes after generation
- Unique: Statistically very low collision probability (1 billion possible IDs)
- Simple: Easy to read and communicate
- No personal information: Just a random number

### ID Display

The ID is prominently displayed in the UI:

```
┌─────────────────────────────┐
│     RemoteDesk              │
├─────────────────────────────┤
│  Your ID:  123 456 789      │
│  [Copy]                     │
├─────────────────────────────┤
│  Password: [Set Password]   │
│  Manual Accept: [✓]         │
├─────────────────────────────┤
│  Status: Waiting...         │
└─────────────────────────────┘
```

### ID Collision Handling

With 1 billion possible IDs and no central server, collisions are theoretically possible but statistically rare.

**Collision Probability:**
- With 1 million active users: ~0.05% chance of any collision
- With 10 million active users: ~5% chance of any collision

**Collision Resolution:**
If two devices have the same ID on the same network:
1. Detection: mDNS discovery will detect duplicate ID
2. Resolution: Newer device regenerates its ID
3. Notification: User is informed of ID change

```rust
fn handle_id_collision() -> Result<u32> {
    log::warn!("ID collision detected, generating new ID");

    // Generate new ID
    let new_id = generate_id();

    // Save new ID
    let config_dir = get_config_dir()?;
    let id_file = config_dir.join("device_id");
    fs::write(&id_file, new_id.to_string())?;

    // Notify user
    show_notification(
        "ID Changed",
        &format!("Your new ID is: {}", format_id(new_id))
    )?;

    Ok(new_id)
}
```

## Authentication Modes

RemoteDesk supports two authentication modes:

### Mode 1: Manual Accept (No Password)

**Default mode** - Secure and requires explicit user action.

**Flow:**
```
1. Client enters host's 9-digit ID
2. Client clicks "Connect"
3. Host receives connection request notification
4. Host reviews connection details:
   - Client ID: 987 654 321
   - Client Name: John's Laptop
   - IP Address: 192.168.1.100
5. Host clicks "Accept" or "Reject"
6. If accepted, connection established
```

**Security:**
- User must explicitly accept each connection
- Cannot be automated or bypassed
- Prevents unauthorized access even with ID known

**Use Cases:**
- Temporary assistance
- Infrequent connections
- Maximum security
- When different people might connect

### Mode 2: Password Access (Automatic)

**Optional mode** - Convenient for trusted users.

**Setup:**
```
1. Host sets a password in RemoteDesk settings
2. Password is hashed and stored locally
3. Manual accept is automatically disabled
4. Host shares ID + password with trusted users
```

**Flow:**
```
1. Client enters host's 9-digit ID
2. Client enters password
3. Client clicks "Connect"
4. Password is verified
5. If correct, connection established automatically
6. No manual accept required
```

**Security:**
- Password never transmitted in plain text
- Password hashed using Argon2id
- Rate limiting on password attempts
- Account lockout after failed attempts

**Use Cases:**
- Regular connections from trusted users
- Personal device to personal device
- IT support with pre-shared credentials
- Family members accessing each other's devices

### Mode Comparison

| Feature | Manual Accept | Password Access |
|---------|---------------|-----------------|
| Password Required | No | Yes |
| User Action Required | Yes (every time) | No (if password correct) |
| Security Level | Highest | High |
| Convenience | Lower | Higher |
| Best For | Occasional access | Regular access |
| Default | ✓ | |

## Password Management

### Setting a Password

```rust
use argon2::{self, Config, Variant, Version};

const PASSWORD_MIN_LENGTH: usize = 6;
const PASSWORD_MAX_LENGTH: usize = 128;

/// Sets or updates the password
fn set_password(password: &str) -> Result<()> {
    // Validate password length
    if password.len() < PASSWORD_MIN_LENGTH {
        return Err(AuthError::PasswordTooShort);
    }

    if password.len() > PASSWORD_MAX_LENGTH {
        return Err(AuthError::PasswordTooLong);
    }

    // Generate salt
    let salt: [u8; 16] = rand::random();

    // Hash password using Argon2id
    let config = Config {
        variant: Variant::Argon2id,
        version: Version::Version13,
        mem_cost: 65536,      // 64 MB
        time_cost: 3,         // 3 iterations
        lanes: 4,             // 4 parallel threads
        thread_mode: argon2::ThreadMode::Parallel,
        secret: &[],
        ad: &[],
        hash_length: 32
    };

    let hash = argon2::hash_encoded(
        password.as_bytes(),
        &salt,
        &config
    )?;

    // Store hash
    let config_dir = get_config_dir()?;
    let password_file = config_dir.join("password.hash");
    fs::write(&password_file, hash)?;

    Ok(())
}
```

### Password Verification

```rust
/// Verifies a password against the stored hash
fn verify_password(password: &str) -> Result<bool> {
    let config_dir = get_config_dir()?;
    let password_file = config_dir.join("password.hash");

    if !password_file.exists() {
        return Ok(false);
    }

    let stored_hash = fs::read_to_string(&password_file)?;

    Ok(argon2::verify_encoded(&stored_hash, password.as_bytes())?)
}

/// Checks if password is set
fn is_password_set() -> bool {
    let config_dir = get_config_dir().ok()?;
    let password_file = config_dir.join("password.hash");
    password_file.exists()
}
```

### Removing Password

```rust
/// Removes the password (reverts to manual accept mode)
fn remove_password() -> Result<()> {
    let config_dir = get_config_dir()?;
    let password_file = config_dir.join("password.hash");

    if password_file.exists() {
        fs::remove_file(&password_file)?;
    }

    Ok(())
}
```

## Connection Flow

### Detailed Connection Sequence

#### With Password Access

```
Client                          Host
  |                              |
  |-- Connect(ID: 123456789) --->|
  |    Password: "secret123"     |
  |                              |
  |                          [Verify ID]
  |                          [Verify Password]
  |                              |
  |<-- ConnectionAccept ---------|
  |                              |
  |<===== Encrypted Session ====>|
```

#### With Manual Accept

```
Client                          Host
  |                              |
  |-- Connect(ID: 123456789) --->|
  |                              |
  |                          [Verify ID]
  |                          [Show Dialog]
  |                          [User Clicks Accept]
  |                              |
  |<-- ConnectionAccept ---------|
  |                              |
  |<===== Encrypted Session ====>|
```

## Protocol Messages

### Connection Request

```rust
#[derive(Serialize, Deserialize)]
struct ConnectionRequest {
    /// Client's 9-digit ID
    client_id: u32,

    /// Client device name
    client_name: String,

    /// Host's 9-digit ID (to verify)
    host_id: u32,

    /// Optional password (if host requires password)
    password: Option<String>,

    /// Protocol version
    version: u8,
}
```

### Connection Response

```rust
#[derive(Serialize, Deserialize)]
enum ConnectionResponse {
    /// Connection accepted
    Accept {
        session_id: [u8; 16],
        host_name: String,
        desktop_info: DesktopInfo,
    },

    /// Connection rejected
    Reject {
        reason: RejectReason,
    },
}

#[derive(Serialize, Deserialize)]
enum RejectReason {
    /// User manually rejected
    UserDenied,

    /// Wrong password
    InvalidPassword,

    /// ID not found
    InvalidId,

    /// Already connected
    AlreadyConnected,

    /// Account locked (too many failed attempts)
    AccountLocked,
}
```

## Security Considerations

### Rate Limiting

To prevent brute force password attacks:

```rust
const MAX_PASSWORD_ATTEMPTS: u32 = 5;
const LOCKOUT_DURATION: Duration = Duration::from_secs(900); // 15 minutes

struct PasswordLimiter {
    attempts: HashMap<u32, AttemptRecord>,
}

struct AttemptRecord {
    failed_attempts: u32,
    locked_until: Option<Instant>,
}

impl PasswordLimiter {
    fn check_and_record(&mut self, client_id: u32, success: bool) -> Result<()> {
        let record = self.attempts.entry(client_id).or_default();

        // Check if locked
        if let Some(locked_until) = record.locked_until {
            if Instant::now() < locked_until {
                return Err(AuthError::AccountLocked);
            } else {
                // Lockout expired, reset
                record.failed_attempts = 0;
                record.locked_until = None;
            }
        }

        if success {
            // Reset on success
            self.attempts.remove(&client_id);
            Ok(())
        } else {
            // Increment failures
            record.failed_attempts += 1;

            if record.failed_attempts >= MAX_PASSWORD_ATTEMPTS {
                // Lock account
                record.locked_until = Some(Instant::now() + LOCKOUT_DURATION);

                // Notify host
                show_notification(
                    "Security Alert",
                    &format!("Account locked due to {} failed password attempts from ID: {}",
                        MAX_PASSWORD_ATTEMPTS, client_id)
                )?;

                Err(AuthError::AccountLocked)
            } else {
                Err(AuthError::InvalidPassword)
            }
        }
    }
}
```

### Password Hashing

**Why Argon2id:**
- Winner of Password Hashing Competition
- Resistant to GPU/ASIC attacks
- Memory-hard (prevents parallel attacks)
- Configurable cost parameters

**Parameters:**
- Memory cost: 64 MB (prevents massive parallel attacks)
- Time cost: 3 iterations (balances security and UX)
- Parallelism: 4 threads (utilizes modern CPUs)

### Connection Logging

All connection attempts are logged:

```rust
#[derive(Serialize, Deserialize)]
struct ConnectionLog {
    timestamp: SystemTime,
    client_id: u32,
    client_name: String,
    client_ip: IpAddr,
    success: bool,
    reason: Option<String>,
}

fn log_connection_attempt(
    client_id: u32,
    client_name: String,
    client_ip: IpAddr,
    success: bool,
    reason: Option<String>,
) -> Result<()> {
    let log = ConnectionLog {
        timestamp: SystemTime::now(),
        client_id,
        client_name,
        client_ip,
        success,
        reason,
    };

    // Append to log file
    let config_dir = get_config_dir()?;
    let log_file = config_dir.join("connections.log");

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;

    writeln!(file, "{}", serde_json::to_string(&log)?)?;

    Ok(())
}
```

## UI/UX Considerations

### First Launch Experience

```
┌──────────────────────────────────────────┐
│  Welcome to RemoteDesk!                  │
│                                          │
│  Your unique ID has been generated:      │
│                                          │
│          123 456 789                     │
│                                          │
│  Share this ID with anyone who needs     │
│  to connect to your computer.            │
│                                          │
│  [Copy ID]  [Continue]                   │
└──────────────────────────────────────────┘
```

### Main Window

```
┌─────────────────────────────────────┐
│  RemoteDesk                    [≡]  │
├─────────────────────────────────────┤
│                                     │
│  Your ID:  123 456 789   [Copy]    │
│                                     │
│  ○ Manual Accept (recommended)      │
│     Accept connections manually     │
│                                     │
│  ○ Password Access                  │
│     [Set Password]                  │
│     Auto-accept with password       │
│                                     │
├─────────────────────────────────────┤
│  Connect to Another Computer        │
│                                     │
│  Remote ID:  [___] [___] [___]     │
│  Password:   [_________________]    │
│                                     │
│              [Connect]              │
│                                     │
└─────────────────────────────────────┘
```

### Connection Request Dialog

```
┌──────────────────────────────────────┐
│  Incoming Connection Request         │
├──────────────────────────────────────┤
│                                      │
│  From ID:    987 654 321             │
│  Name:       John's Laptop           │
│  IP:         192.168.1.100           │
│  Time:       14:32:15                │
│                                      │
│  ⚠ This will allow full control      │
│     of your computer                 │
│                                      │
│  [Reject]              [Accept]      │
│                                      │
│  Auto-reject in 60 seconds           │
└──────────────────────────────────────┘
```

## Configuration

### Configuration File

```toml
# remotedesk.toml

[identity]
# 9-digit device ID (auto-generated)
device_id = 123456789

[authentication]
# Authentication mode: "manual" or "password"
mode = "manual"

# Password file location (when password mode enabled)
password_file = "password.hash"

# Rate limiting
max_password_attempts = 5
lockout_duration_minutes = 15

[connection]
# Manual accept timeout
accept_timeout_seconds = 60

# Connection history retention
history_retention_days = 30

# Maximum concurrent connections
max_connections = 1
```

## Best Practices

### For Hosts

**Manual Accept Mode:**
- ✓ Most secure for occasional access
- ✓ Review each connection request carefully
- ✓ Verify the connecting ID matches who you expect
- ✓ Never accept connections from unknown IDs

**Password Access Mode:**
- ✓ Use strong passwords (8+ characters)
- ✓ Only share password with trusted users
- ✓ Change password periodically
- ✓ Monitor connection logs regularly
- ✗ Don't use the same password for other services

### For Clients

- ✓ Verify the ID belongs to the correct person
- ✓ Enter password carefully (limited attempts)
- ✓ Disconnect when finished
- ✗ Don't share others' IDs and passwords

## Migration and Backup

### Backing Up Your ID

```bash
# Linux/macOS
cp ~/.config/remotedesk/device_id ~/remotedesk_backup.txt

# Windows
copy %APPDATA%\RemoteDesk\device_id remotedesk_backup.txt
```

### Restoring Your ID

```bash
# Linux/macOS
cp ~/remotedesk_backup.txt ~/.config/remotedesk/device_id

# Windows
copy remotedesk_backup.txt %APPDATA%\RemoteDesk\device_id
```

### Moving to a New Device

Option 1: Keep Same ID
- Backup device_id file from old device
- Restore on new device
- Old and new device will have same ID

Option 2: Generate New ID
- Just install on new device
- New ID will be generated automatically
- Share new ID with your contacts

## Future Enhancements

Potential improvements to the ID system:

1. **Custom IDs**: Allow users to choose memorable IDs
2. **QR Codes**: Generate QR code for easy ID sharing
3. **Address Book**: Save frequently used IDs with nicknames
4. **ID Verification**: Optional cryptographic verification
5. **Temporary IDs**: Generate single-use IDs for one-time access
6. **ID Federation**: Optional central registry for ID-to-IP resolution (preserves P2P for actual connection)

## Conclusion

The 9-digit ID system provides a good balance between:
- **Simplicity**: Easy to communicate and remember
- **Security**: Combined with password or manual accept
- **Privacy**: No personal information exposed
- **Usability**: Quick connection setup

The dual authentication modes (manual accept vs. password) give users flexibility to choose the right security level for their use case.

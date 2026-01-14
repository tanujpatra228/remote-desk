# RemoteDesk CLI Guide

## Current Status

**Milestone 1.1 Complete** ✓ - Basic CLI interface with commands
**Milestone 1.2 Pending** ⏳ - Network functionality (coming next)

The CLI interface is functional, but actual network connections will be implemented in Milestone 1.2.

## Running RemoteDesk

```bash
cargo run
```

## Available Commands

### View Your Device ID

```bash
id
```

**Output:**
```
Your Device ID: 621 301 222
```

### Connect to Another Device

**Format 1: With spaces**
```bash
connect 123 456 789
```

**Format 2: Without spaces**
```bash
connect 123456789
```

**Format 3: With password**
```bash
connect 123 456 789 mypassword
```

or

```bash
connect 123456789 mypassword
```

**Current Behavior:**
The command validates the ID format and shows what would happen, but doesn't actually connect (network layer pending).

**Example Output:**
```
Connecting to device: 123 456 789
No password provided (manual accept required)

⚠️  Network functionality not implemented yet (Milestone 1.2)
   This will be available in the next phase.
```

### Set a Password

Enable **Password Access Mode** so connections with the correct password are accepted automatically:

```bash
password MySecurePassword123
```

**Output:**
```
✓ Password set successfully!
  Password Access Mode is now ENABLED
  Connections with this password will be accepted automatically.
```

**Requirements:**
- Minimum 6 characters
- Maximum 128 characters
- Securely hashed with Argon2id

### Remove Password

Revert to **Manual Accept Mode**:

```bash
remove-password
```

**Output:**
```
✓ Password removed successfully!
  Manual Accept Mode is now ENABLED
  You will need to accept each connection manually.
```

### Check Status

```bash
status
```

**Output:**
```
Status:
  Device ID: 621 301 222
  Mode: 🔓 Manual Accept
  Network: Not connected (Milestone 1.2 pending)
```

or with password:

```
Status:
  Device ID: 621 301 222
  Mode: 🔐 Password Access
  Network: Not connected (Milestone 1.2 pending)
```

### Show Help

```bash
help
```

**Output:**
```
Available commands:
  connect <ID> [password]  - Connect to another device
                             Example: connect 123 456 789
                             Example: connect 123456789 mypassword
  password <new_password>  - Set a password for this device
  remove-password          - Remove the password (use manual accept)
  id                       - Show your device ID
  status                   - Show current status
  help                     - Show this help message
  quit / exit              - Exit the application
```

### Exit

```bash
quit
```

or

```bash
exit
```

or press `Ctrl+C`

## Usage Examples

### Example 1: First Time Setup

```bash
$ cargo run

╔═══════════════════════════════════════════════════╗
║           RemoteDesk - Ready to Connect          ║
╚═══════════════════════════════════════════════════╝

  Your Device ID: 621 301 222

  Share this ID with others to allow connections.

  🔓 Manual Accept Mode: ENABLED
     You will need to accept each connection manually.

  Commands:
    Type 'connect <ID>' to connect to another device
    Type 'password <new_password>' to set a password
    Type 'remove-password' to remove password
    Type 'help' for all commands
    Type 'quit' or press Ctrl+C to exit

> help
[Shows help message]

> id
Your Device ID: 621 301 222

> quit
Exiting...
```

### Example 2: Setting Up Password Access

```bash
> status
Status:
  Device ID: 621 301 222
  Mode: 🔓 Manual Accept
  Network: Not connected (Milestone 1.2 pending)

> password MySecurePassword123
✓ Password set successfully!
  Password Access Mode is now ENABLED
  Connections with this password will be accepted automatically.

> status
Status:
  Device ID: 621 301 222
  Mode: 🔐 Password Access
  Network: Not connected (Milestone 1.2 pending)
```

### Example 3: Attempting to Connect (Preview)

```bash
> connect 987 654 321
Connecting to device: 987 654 321
No password provided (manual accept required)

⚠️  Network functionality not implemented yet (Milestone 1.2)
   This will be available in the next phase.

> connect 987654321 theirpassword
Connecting to device: 987 654 321
Using password: **************

⚠️  Network functionality not implemented yet (Milestone 1.2)
   This will be available in the next phase.
```

### Example 4: Invalid ID

```bash
> connect 12345
[ERROR] Invalid device ID: Device ID must be 9 digits, got: 12345
Device ID must be 9 digits (e.g., 123456789 or 123 456 789)
```

## Authentication Modes

### 🔓 Manual Accept Mode (Default)

**How it works:**
1. Someone enters your ID to connect
2. You see a connection request notification
3. You click "Accept" or "Reject"
4. Connection established (if accepted)

**Security:** Maximum - you control every connection
**Use case:** Occasional help from different people

**To enable:**
```bash
remove-password
```

### 🔐 Password Access Mode (Optional)

**How it works:**
1. You set a password
2. Someone enters your ID + password
3. Connection automatically accepted if password is correct
4. No manual action needed

**Security:** High - password required
**Use case:** Regular access from trusted users

**To enable:**
```bash
password YourSecurePassword
```

## Configuration Files

RemoteDesk stores configuration in:
- **Linux/macOS:** `~/.config/remotedesk/`
- **Windows:** `%APPDATA%\RemoteDesk\`

**Files created:**
- `device_id` - Your permanent 9-digit ID
- `config.toml` - Application configuration
- `password.hash` - Password hash (if set)
- `connections.log` - Connection history (future)

### View Your Device ID

```bash
cat ~/.config/remotedesk/device_id
# Output: 621301222
```

### View Configuration

```bash
cat ~/.config/remotedesk/config.toml
```

## Tips

1. **Share Your ID Securely**
   - You can share your ID openly
   - Only share your password with trusted users
   - Use password mode only for regular access

2. **Format Flexibility**
   - Both `123456789` and `123 456 789` work
   - Spaces are automatically removed
   - ID is validated before use

3. **Password Best Practices**
   - Use at least 8 characters (6 minimum)
   - Mix letters, numbers, and symbols
   - Don't use the same password as other services
   - Change periodically

4. **Testing Without Network**
   - You can test all commands now
   - Commands validate input and show expected behavior
   - Actual connections coming in Milestone 1.2

## What's Coming in Milestone 1.2

The next phase will implement:

- ✓ QUIC network connections
- ✓ Peer discovery (mDNS for local network)
- ✓ Actual P2P connectivity
- ✓ Connection handshake
- ✓ Heartbeat mechanism
- ✓ Connection lifecycle management

**Timeline:** 2-3 weeks

Then the `connect` command will actually establish connections!

## Troubleshooting

### Command Not Working

Make sure you're typing commands correctly:
```bash
# Correct
connect 123456789

# Incorrect (missing space after command)
connect123456789
```

### Invalid ID Error

Device IDs must be exactly 9 digits:
```bash
# Valid
connect 123456789
connect 123 456 789

# Invalid
connect 12345       # Too short
connect 1234567890  # Too long
connect abc123456   # Not numeric
```

### Password Too Short

Passwords must be at least 6 characters:
```bash
# Valid
password secure123

# Invalid
password short  # Only 5 characters
```

## Interactive Mode

The CLI runs in **interactive mode** where you can type multiple commands:

```bash
$ cargo run
[Welcome screen]
> id
[Shows ID]
> status
[Shows status]
> connect 123456789
[Attempts connection]
> quit
[Exits]
```

## Batch Mode

You can also pipe commands:

```bash
echo "id\nstatus\nquit" | cargo run
```

or from a file:

```bash
cat commands.txt | cargo run
```

## Development Mode

Run with debug logging:

```bash
RUST_LOG=debug cargo run
```

This shows detailed logs for debugging.

## Questions?

- Check the main README: `README.md`
- Read the quick start: `QUICKSTART.md`
- View development log: `DEVELOPMENT_LOG.md`
- See full roadmap: `docs/ROADMAP.md`

---

**Remember:** Network functionality is coming in Milestone 1.2. For now, you can practice with the commands and see how the interface will work!

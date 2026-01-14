# RemoteDesk Quick Start Guide

This guide will help you get started with RemoteDesk in minutes.

## What is RemoteDesk?

RemoteDesk is a lightweight, peer-to-peer remote desktop application that lets you control another computer over the network. Think of it as a simpler alternative to AnyDesk or TeamViewer, built with a focus on essential features and privacy.

## Key Concepts

### Your 9-Digit ID

When you first launch RemoteDesk, you'll get a unique **9-digit ID** (like **123 456 789**). This ID:
- Is generated randomly on first launch
- Stays the same forever (unless you manually change it)
- Is used by others to connect to your computer
- Contains no personal information

**Example:** Your ID might be `456 789 123`

### Two Ways to Connect

RemoteDesk gives you two authentication options:

#### 1. Manual Accept (Default)
- **Most Secure**
- Someone enters your ID to connect
- You see a popup and click "Accept" or "Reject"
- Best for: Occasional help from different people

#### 2. Password Access (Optional)
- **More Convenient**
- You set a password
- Anyone with your ID + password connects automatically
- No popup needed
- Best for: Regular access from trusted users

## First Time Setup

### Step 1: Install and Launch

```bash
# Download and run RemoteDesk
./remotedesk
```

On first launch, you'll see:
```
┌──────────────────────────────────────┐
│  Welcome to RemoteDesk!              │
│                                      │
│  Your ID: 123 456 789                │
│                                      │
│  [Copy ID]  [Continue]               │
└──────────────────────────────────────┘
```

**That's it!** You now have a permanent ID.

### Step 2: Choose Your Security Mode

**Option A: Stay with Manual Accept (Recommended)**
- No additional setup needed
- Just share your ID when someone needs to connect
- You'll approve each connection

**Option B: Set Up Password Access**
1. Open Settings
2. Click "Set Password"
3. Enter a password (6+ characters)
4. Share your ID + password with trusted users

## How to Connect

### Scenario 1: Someone Wants to Connect to YOU

**If using Manual Accept:**
1. Share your 9-digit ID
2. Wait for connection notification
3. Review connection details
4. Click "Accept"

**If using Password Access:**
1. Share your 9-digit ID + password
2. They connect automatically
3. No action needed from you

### Scenario 2: You Want to Connect to SOMEONE ELSE

1. Get their 9-digit ID (and password if they use password mode)
2. Open RemoteDesk
3. Enter their ID in the "Remote ID" field
4. Enter password (if required)
5. Click "Connect"
6. Wait for them to accept (if using manual mode)
7. You're connected!

## Example Walkthrough

### Example: Helping Your Friend

**Your friend needs help with their computer:**

1. Your friend launches RemoteDesk
2. They tell you their ID: `987 654 321`
3. You enter `987 654 321` in RemoteDesk
4. You click "Connect"
5. Your friend sees: "Connection request from [Your Name]"
6. Your friend clicks "Accept"
7. You can now see and control their desktop!

### Example: Regular Access to Your Work Computer

**You want to access your work computer from home:**

1. At work, set up RemoteDesk with password access
2. Set password to something secure: `MyWorkPC2024!`
3. Note your work computer's ID: `111 222 333`
4. At home, open RemoteDesk
5. Enter ID: `111 222 333`
6. Enter password: `MyWorkPC2024!`
7. Click "Connect"
8. Instant access! (no need to accept at work computer)

## Main Interface Overview

```
┌─────────────────────────────────────────┐
│  RemoteDesk                        [≡]  │
├─────────────────────────────────────────┤
│  YOUR INFORMATION                       │
│                                         │
│  Your ID:  123 456 789      [Copy]     │
│                                         │
│  Security Mode:                         │
│  ○ Manual Accept ✓                     │
│  ○ Password Access  [Set Password]     │
│                                         │
├─────────────────────────────────────────┤
│  CONNECT TO ANOTHER COMPUTER            │
│                                         │
│  Remote ID:  [___] [___] [___]         │
│  Password:   [___________________]      │
│                                         │
│              [Connect]                  │
│                                         │
├─────────────────────────────────────────┤
│  Status: Ready                          │
└─────────────────────────────────────────┘
```

## During a Remote Session

### As the Controller (Client)
- You see the remote computer's screen
- You can use your keyboard and mouse normally
- The remote computer responds to your actions
- Click "Disconnect" when done

### As the Controlled (Host)
- You see a notification: "Remote session active"
- You can still use your computer normally
- You can see the remote cursor moving
- Click "Disconnect" to end the session

## Common Questions

### How do I find my ID?
Open RemoteDesk - it's displayed at the top of the window.

### Can I change my ID?
Not recommended, but yes - delete the configuration file and restart RemoteDesk. You'll get a new ID.

**Location:**
- Linux: `~/.config/remotedesk/device_id`
- macOS: `~/Library/Application Support/RemoteDesk/device_id`
- Windows: `%APPDATA%\RemoteDesk\device_id`

### Is it secure?
Yes! All connections are encrypted. With manual accept, you control who connects. With password access, only those with the password can connect.

### What if someone guesses my ID?
- 1 billion possible IDs = very unlikely to guess
- With manual accept: You must click "Accept" anyway
- With password access: They also need your password

### What if I forget my password?
Delete the password file and set a new one:
- Linux: `~/.config/remotedesk/password.hash`
- macOS: `~/Library/Application Support/RemoteDesk/password.hash`
- Windows: `%APPDATA%\RemoteDesk\password.hash`

### Can multiple people connect to me?
By default, no. You can change this in settings (max_connections).

### Does it work over the internet?
Yes! RemoteDesk uses NAT traversal to connect across different networks. Just share your ID - no port forwarding needed.

### Do I need to keep RemoteDesk open to receive connections?
Yes. RemoteDesk must be running (can be minimized to system tray).

## Tips and Best Practices

### For Security

✓ **DO:**
- Use manual accept for maximum security
- Use strong passwords if using password mode
- Review connection logs periodically
- Disconnect when you're done

✗ **DON'T:**
- Share your ID and password publicly
- Leave sessions running unattended
- Use simple passwords like "123456"
- Accept connections from unknown IDs

### For Performance

✓ **DO:**
- Use wired connections when possible
- Close unnecessary applications during remote sessions
- Use lower quality settings on slow connections

### For Convenience

✓ **DO:**
- Add frequently used IDs to favorites (future feature)
- Use password mode for your own devices
- Set up system tray auto-start (future feature)

## Troubleshooting

### Connection Failed

**Problem:** Can't connect to remote computer

**Solutions:**
1. Verify you entered the correct ID
2. Check if the remote computer is online and running RemoteDesk
3. Try on the same local network first
4. Check firewall settings

### Connection Rejected

**Problem:** "Connection rejected" message

**Possible reasons:**
- Host clicked "Reject"
- Wrong password
- Host ID incorrect
- Too many failed password attempts (account locked)

**Solution:** Contact the host and verify ID/password

### Slow Performance

**Problem:** Laggy or slow remote desktop

**Solutions:**
1. Reduce quality settings
2. Check your internet connection speed
3. Close bandwidth-intensive applications
4. Use wired connection instead of Wi-Fi

### Password Locked

**Problem:** "Account locked" after failed password attempts

**Solution:** Wait 15 minutes, or ask the host to delete the lock file and restart RemoteDesk

## Next Steps

- Read [docs/SECURITY.md](./docs/SECURITY.md) for security details
- Read [docs/ID_AND_AUTH.md](./docs/ID_AND_AUTH.md) for authentication details
- Check [docs/ROADMAP.md](./docs/ROADMAP.md) for upcoming features

## Getting Help

- Check documentation in `docs/` folder
- Report issues on GitHub
- Read the FAQ in README.md

---

**Remember:** Your 9-digit ID is your identity in RemoteDesk. Keep it handy and share it only with people you trust!

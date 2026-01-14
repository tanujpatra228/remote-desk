# Build Dependencies

This document lists the system dependencies required to build RemoteDesk on different platforms.

## Linux

RemoteDesk requires X11/XCB libraries for screen capture on Linux.

### Ubuntu / Debian

```bash
sudo apt-get install libxcb1-dev libxcb-shm0-dev libxcb-randr0-dev
```

### Fedora / RHEL

```bash
sudo dnf install libxcb-devel libxcb-shm-devel libxcb-randr-devel
```

### Arch Linux

```bash
sudo pacman -S libxcb
```

## macOS

No additional system dependencies are required on macOS. The scrap crate uses native macOS APIs.

## Windows

No additional system dependencies are required on Windows. The scrap crate uses Windows APIs.

## Rust Dependencies

All Rust dependencies are managed through Cargo and will be downloaded automatically:

- **scrap** - Cross-platform screen capture
- **image** - Image encoding/decoding (JPEG, PNG)
- **bytes** - Efficient byte buffer handling
- **tokio** - Async runtime
- **serde** - Serialization
- **tracing** - Logging
- And more (see Cargo.toml)

## Building

After installing system dependencies:

```bash
# Build the project
cargo build

# Run tests
cargo test

# Build release version
cargo build --release
```

## Troubleshooting

### Linux: "unable to find library -lxcb-randr"

This error means the XCB development libraries are not installed. Install them using the commands above for your distribution.

### macOS: Permission errors when capturing screen

On macOS 10.15+, you may need to grant screen recording permissions:
1. System Preferences → Security & Privacy → Privacy → Screen Recording
2. Add Terminal (or your IDE) to the list

### Windows: Build fails with MSVC errors

Make sure you have the Visual Studio build tools installed:
- Download from: https://visualstudio.microsoft.com/downloads/
- Select "Desktop development with C++"

## Development Mode

For development with debug logging:

```bash
RUST_LOG=debug cargo run
```

For release mode with optimizations:

```bash
cargo run --release
```

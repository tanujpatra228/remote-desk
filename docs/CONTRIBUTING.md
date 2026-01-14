# Contributing to RemoteDesk

Thank you for your interest in contributing to RemoteDesk! This document provides guidelines and information for contributors.

## Code of Conduct

Be respectful, constructive, and professional in all interactions.

## Getting Started

### Prerequisites

- Rust 1.70 or higher
- Git
- Platform-specific build tools (see ARCHITECTURE.md)

### Development Setup

```bash
# Clone the repository
git clone <repository-url>
cd remote-desk

# Build the project
cargo build

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run
```

## Development Workflow

### Branching Strategy

- `master`: Stable, production-ready code
- `develop`: Integration branch for features
- `feature/*`: Feature development branches
- `bugfix/*`: Bug fix branches
- `hotfix/*`: Critical production fixes

### Making Changes

1. **Create a Branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make Your Changes**
   - Write clean, documented code
   - Follow the coding standards below
   - Add tests for new functionality

3. **Test Your Changes**
   ```bash
   cargo test
   cargo clippy
   cargo fmt --check
   ```

4. **Commit Your Changes**
   ```bash
   git add .
   git commit -m "Brief description of changes"
   ```

5. **Push and Create Pull Request**
   ```bash
   git push origin feature/your-feature-name
   ```

## Coding Standards

### Rust Style Guide

Follow the official [Rust Style Guide](https://doc.rust-lang.org/1.0.0/style/).

**Key Points:**
- Use `cargo fmt` for consistent formatting
- Use `cargo clippy` for linting
- Maximum line length: 100 characters
- Use meaningful variable and function names
- Add documentation comments for public APIs

### Code Organization

```rust
// Module structure
mod network {
    mod connection;
    mod discovery;
    mod protocol;
}

// Imports order
use std::collections::HashMap;  // std library
use tokio::sync::mpsc;          // external crates
use crate::network::connection; // internal modules

// Constants
const MAX_CONNECTIONS: usize = 5;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

// Types
pub struct Connection { /* ... */ }
pub enum ConnectionState { /* ... */ }

// Implementation
impl Connection {
    pub fn new() -> Self { /* ... */ }
    fn internal_method(&self) { /* ... */ }
}
```

### Error Handling

Use Result types and custom error enums:

```rust
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Timeout after {0:?}")]
    Timeout(Duration),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, NetworkError>;

fn connect() -> Result<Connection> {
    // Use ? operator for error propagation
    let socket = TcpStream::connect("127.0.0.1:8080")?;
    Ok(Connection { socket })
}
```

### Documentation

Document public APIs with doc comments:

```rust
/// Establishes a connection to the specified peer.
///
/// # Arguments
///
/// * `peer_id` - The unique identifier of the peer to connect to
/// * `password` - The authentication password
///
/// # Returns
///
/// Returns a `Connection` object on success, or an error if connection fails.
///
/// # Examples
///
/// ```no_run
/// let conn = connect_to_peer(peer_id, "password123").await?;
/// ```
///
/// # Errors
///
/// Returns `NetworkError::ConnectionFailed` if unable to establish connection.
/// Returns `NetworkError::AuthenticationFailed` if password is incorrect.
pub async fn connect_to_peer(peer_id: PeerId, password: &str) -> Result<Connection> {
    // Implementation
}
```

### Testing

Write tests for all new functionality:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_creation() {
        let conn = Connection::new();
        assert!(conn.is_valid());
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = async_function().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_error_handling() {
        let result = function_that_fails();
        assert!(matches!(result, Err(NetworkError::ConnectionFailed(_))));
    }
}
```

## Commit Message Guidelines

Use conventional commit format:

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks
- `perf`: Performance improvements
- `security`: Security improvements

**Examples:**
```
feat(network): add STUN client implementation

Implemented STUN protocol client for NAT traversal.
Supports binding requests and response parsing.

Closes #123
```

```
fix(desktop): resolve screen capture memory leak

Fixed memory leak in screen capture module by properly
releasing frame buffers after use.

Fixes #456
```

## Pull Request Process

### Before Submitting

- [ ] Code compiles without errors
- [ ] All tests pass (`cargo test`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] Documentation is updated
- [ ] CHANGELOG.md is updated (if applicable)

### PR Description Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
Describe how you tested your changes

## Checklist
- [ ] My code follows the style guidelines
- [ ] I have performed a self-review
- [ ] I have commented my code where needed
- [ ] I have updated the documentation
- [ ] My changes generate no new warnings
- [ ] I have added tests
- [ ] All tests pass

## Related Issues
Closes #<issue-number>
```

### Review Process

1. Automated checks must pass (CI/CD)
2. Code review by at least one maintainer
3. Address review comments
4. Maintainer approves and merges

## Areas to Contribute

### High Priority

- NAT traversal improvements
- Performance optimizations
- Cross-platform compatibility
- Security enhancements
- Bug fixes

### Feature Development

- File transfer
- Audio streaming
- Mobile clients
- UI improvements

### Documentation

- User guides
- API documentation
- Architecture documentation
- Troubleshooting guides

### Testing

- Unit tests
- Integration tests
- Platform-specific testing
- Performance benchmarks

## Platform-Specific Contributions

### Windows Development

- Visual Studio Build Tools
- Windows SDK
- Testing on various Windows versions

### Linux Development

- Test on multiple distributions
- X11 and Wayland compatibility
- Package creation (deb, rpm)

### macOS Development

- Xcode Command Line Tools
- Testing on various macOS versions
- App signing and notarization

## Issue Guidelines

### Reporting Bugs

**Use the bug report template:**

```markdown
**Describe the bug**
A clear description of the bug

**To Reproduce**
Steps to reproduce:
1. Do this
2. Then this
3. See error

**Expected behavior**
What you expected to happen

**Actual behavior**
What actually happened

**Environment:**
- OS: [e.g., Windows 11, Ubuntu 22.04]
- Version: [e.g., 0.1.0]
- Build: [e.g., debug/release]

**Logs**
Relevant log output (if available)

**Additional context**
Any other relevant information
```

### Feature Requests

```markdown
**Feature Description**
Clear description of the proposed feature

**Use Case**
Why this feature would be useful

**Proposed Implementation**
(Optional) How you think this could be implemented

**Alternatives Considered**
Other solutions you've considered

**Additional Context**
Any other relevant information
```

## Security Issues

**DO NOT** create public issues for security vulnerabilities.

Instead:
1. Email security@remotedesk.example (replace with actual contact)
2. Include detailed description
3. Allow time for fix before disclosure

See SECURITY.md for full security policy.

## Build and Release Process

### Building for Release

```bash
# Release build
cargo build --release

# Platform-specific optimizations
# Windows
cargo build --release --target x86_64-pc-windows-msvc

# Linux
cargo build --release --target x86_64-unknown-linux-gnu

# macOS
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
```

### Version Numbering

Follow [Semantic Versioning](https://semver.org/):

- MAJOR.MINOR.PATCH
- Example: 1.2.3

### Release Checklist

- [ ] Update version in Cargo.toml
- [ ] Update CHANGELOG.md
- [ ] Run full test suite
- [ ] Build for all platforms
- [ ] Test on all platforms
- [ ] Update documentation
- [ ] Create git tag
- [ ] Create release notes
- [ ] Publish binaries

## Getting Help

- GitHub Issues: For bugs and feature requests
- Discussions: For questions and ideas
- Documentation: Check docs/ directory
- Code Comments: Read inline documentation

## Recognition

Contributors will be recognized in:
- CONTRIBUTORS.md file
- Release notes
- Project website (when available)

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (see LICENSE file).

## Questions?

If you have questions about contributing, feel free to ask in GitHub Discussions or create an issue with the "question" label.

Thank you for contributing to RemoteDesk!

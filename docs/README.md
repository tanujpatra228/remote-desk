# RemoteDesk Documentation Index

This directory contains all technical and planning documentation for RemoteDesk.

## Documentation Overview

### For Users

- **[../QUICKSTART.md](../QUICKSTART.md)** - Start here! Beginner-friendly guide to using RemoteDesk
- **[SYSTEM_OVERVIEW.md](./SYSTEM_OVERVIEW.md)** - Visual overview of how RemoteDesk works
- **[ID_AND_AUTH.md](./ID_AND_AUTH.md)** - Detailed explanation of 9-digit IDs and authentication

### For Developers

- **[ARCHITECTURE.md](./ARCHITECTURE.md)** - Complete technical architecture and design
- **[PROTOCOL.md](./PROTOCOL.md)** - P2P communication protocol specification
- **[ROADMAP.md](./ROADMAP.md)** - Development roadmap and milestones
- **[CONTRIBUTING.md](./CONTRIBUTING.md)** - How to contribute to the project

### For Security

- **[SECURITY.md](./SECURITY.md)** - Security architecture and best practices

## Quick Navigation

### By Topic

#### Getting Started
1. [QUICKSTART.md](../QUICKSTART.md) - How to use RemoteDesk
2. [SYSTEM_OVERVIEW.md](./SYSTEM_OVERVIEW.md) - How it works (visual)

#### Understanding the System
1. [ID_AND_AUTH.md](./ID_AND_AUTH.md) - ID system and authentication modes
2. [ARCHITECTURE.md](./ARCHITECTURE.md) - Technical architecture
3. [PROTOCOL.md](./PROTOCOL.md) - Network protocol details

#### Development
1. [ROADMAP.md](./ROADMAP.md) - What's being built and when
2. [CONTRIBUTING.md](./CONTRIBUTING.md) - How to contribute
3. [SECURITY.md](./SECURITY.md) - Security considerations

## Document Summaries

### QUICKSTART.md
**For:** End users
**Purpose:** Learn how to use RemoteDesk in 5 minutes
**Contents:**
- What is RemoteDesk
- First time setup
- How to connect
- Example scenarios
- Troubleshooting

### SYSTEM_OVERVIEW.md
**For:** Everyone
**Purpose:** Understand how RemoteDesk works (with diagrams)
**Contents:**
- Visual architecture diagrams
- Connection flow
- Component overview
- Use cases
- Performance characteristics

### ID_AND_AUTH.md
**For:** Users and developers
**Purpose:** Deep dive into ID system and authentication
**Contents:**
- 9-digit ID generation
- Manual accept mode
- Password access mode
- Security considerations
- Implementation details

### ARCHITECTURE.md
**For:** Developers
**Purpose:** Complete technical architecture
**Contents:**
- System architecture (5 layers)
- Module descriptions
- Platform-specific details
- Dependencies
- Testing strategy

### PROTOCOL.md
**For:** Developers
**Purpose:** P2P communication protocol specification
**Contents:**
- Protocol stack
- Message format
- All message types
- QUIC streams
- Security details

### SECURITY.md
**For:** Security researchers and developers
**Purpose:** Security architecture and best practices
**Contents:**
- Threat model
- Security layers
- Authentication details
- Encryption implementation
- Privacy considerations
- Best practices

### ROADMAP.md
**For:** Developers and contributors
**Purpose:** Development plan and timeline
**Contents:**
- 4 development phases
- Detailed milestones
- Timeline estimates
- Priorities
- Success metrics

### CONTRIBUTING.md
**For:** Contributors
**Purpose:** How to contribute to RemoteDesk
**Contents:**
- Development setup
- Coding standards
- Testing guidelines
- Pull request process
- Issue guidelines

## Reading Paths

### Path 1: "I want to use RemoteDesk"
1. Start with [QUICKSTART.md](../QUICKSTART.md)
2. Read [ID_AND_AUTH.md](./ID_AND_AUTH.md) for authentication details
3. Check [SECURITY.md](./SECURITY.md) for security tips

### Path 2: "I want to understand how it works"
1. Start with [SYSTEM_OVERVIEW.md](./SYSTEM_OVERVIEW.md)
2. Read [ID_AND_AUTH.md](./ID_AND_AUTH.md) for ID system
3. Dive into [ARCHITECTURE.md](./ARCHITECTURE.md) for details
4. Check [PROTOCOL.md](./PROTOCOL.md) for protocol specs

### Path 3: "I want to contribute code"
1. Read [CONTRIBUTING.md](./CONTRIBUTING.md)
2. Study [ARCHITECTURE.md](./ARCHITECTURE.md)
3. Review [PROTOCOL.md](./PROTOCOL.md)
4. Check [ROADMAP.md](./ROADMAP.md) for current priorities

### Path 4: "I want to review security"
1. Start with [SECURITY.md](./SECURITY.md)
2. Check [ID_AND_AUTH.md](./ID_AND_AUTH.md) for auth details
3. Review [PROTOCOL.md](./PROTOCOL.md) for protocol security
4. Study [ARCHITECTURE.md](./ARCHITECTURE.md) for implementation

## Key Design Decisions

### 9-Digit Numeric IDs
- **Why:** Simple to communicate, easy to remember
- **Trade-off:** 1 billion possible IDs vs. cryptographic security
- **Mitigation:** Combined with authentication (password or manual accept)

### Dual Authentication Modes
- **Why:** Flexibility for different use cases
- **Manual Accept:** Maximum security, user control
- **Password Access:** Convenience for trusted connections

### Pure P2P Architecture
- **Why:** Privacy, no central server dependency
- **Trade-off:** NAT traversal complexity
- **Benefit:** No data passes through third parties

### QUIC Protocol
- **Why:** Low latency, built-in encryption, multiplexing
- **Trade-off:** More complex than plain TCP
- **Benefit:** Better performance for real-time data

## Implementation Status

See [ROADMAP.md](./ROADMAP.md) for current implementation status.

**Current Phase:** Phase 1 - Foundation (MVP)

## Getting Help

### For Users
- Check [QUICKSTART.md](../QUICKSTART.md) for common questions
- Review troubleshooting section in QUICKSTART

### For Developers
- Read [CONTRIBUTING.md](./CONTRIBUTING.md)
- Check [ARCHITECTURE.md](./ARCHITECTURE.md) for technical details
- Review existing code in `src/` directory

### For Security Researchers
- Read [SECURITY.md](./SECURITY.md)
- Contact: security@remotedesk.example (replace with actual contact)
- Follow responsible disclosure guidelines

## Document Maintenance

These documents should be updated when:
- Architecture changes
- New features are added
- Security vulnerabilities are discovered
- Protocol is modified
- Development roadmap shifts

## License

All documentation is licensed under the same license as the RemoteDesk project.

---

**Last Updated:** 2024-01-14
**Version:** 0.1.0-alpha

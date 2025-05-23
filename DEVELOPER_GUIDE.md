# Volt Developer Guide

This document provides a comprehensive guide for developers who want to build on the Volt network, integrate with existing applications, or contribute to the core codebase.

## Table of Contents

1. [Introduction](#introduction)
2. [Getting Started](#getting-started)
3. [Development Environment](#development-environment)
4. [Core Concepts](#core-concepts)
5. [API Reference](#api-reference)
6. [Integration Guide](#integration-guide)
7. [Contributing](#contributing)
8. [Testing](#testing)
9. [Documentation](#documentation)
10.   [Resources](#resources)

## Introduction

Volt is a chainless token transfer network that enables instant, feeless, and privacy-preserving asset transfers without a global blockchain ledger. This guide will help you understand the architecture, APIs, and development patterns for building on Volt.

### Key Features for Developers

-  **Feeless Transactions**: No transaction fees for end users
-  **Instant Finality**: Transactions are final immediately
-  **Multi-Token Support**: Create and manage custom tokens
-  **Privacy Features**: Enhanced privacy through cryptographic proofs
-  **Ethereum Bridge**: Interoperability with Ethereum
-  **JSON-RPC API**: Standard API for integration

## Getting Started

### Prerequisites

-  **Rust**: Version 1.60 or later
-  **Cargo**: The Rust package manager
-  **Git**: For cloning the repository
-  **RocksDB**: For state persistence

### Installation

1. Clone the repository:

```bash
git clone https://github.com/volt/volt.git
cd volt
```

2. Build the project:

```bash
cargo build --release
```

3. Run a local node:

```bash
mkdir -p ~/.volt/node1
RUST_LOG=debug ./target/release/node --rpc --data-dir ~/.volt/node1 --rpc-addr 127.0.0.1:8545 --listen /ip4/127.0.0.1/tcp/30333
```

4. Initialize a wallet:

```bash
./target/release/cli init-seed
```

### Quick Start Examples

#### Checking Balance

```bash
./target/release/cli balance
```

#### Sending Tokens

```bash
./target/release/cli send --to 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef --amount 100
```

#### Issuing a Token

```bash
./target/release/cli issue-token --metadata "My Token|MTK|18"
```

## Development Environment

### IDE Setup

#### Visual Studio Code

1. Install the Rust extension
2. Install the Better TOML extension
3. Configure settings:

```json
{
   "rust-analyzer.checkOnSave.command": "clippy",
   "rust-analyzer.cargo.features": ["all"]
}
```

#### IntelliJ IDEA / CLion

1. Install the Rust plugin
2. Configure Cargo settings
3. Set up run configurations for the node and CLI

### Debugging

1. Use `RUST_LOG=debug` for detailed logging
2. Use `cargo test -- --nocapture` to see test output
3. Use VS Code or CLion's debugging features

### Local Development Network

1. Run a local node:

```bash
RUST_LOG=debug ./target/release/node --rpc --data-dir ~/.volt/node1 --rpc-addr 127.0.0.1:8545 --listen /ip4/127.0.0.1/tcp/30333
```

2. Connect to the local node:

```bash
./target/release/cli --node http://127.0.0.1:8545 balance
```

## Core Concepts

### Sparse Merkle Tree (SMT)

The SMT is the core data structure used to store account states and generate proofs. Key concepts:

-  **Leaves**: Account states stored as leaves in the tree
-  **Root**: The root hash representing the entire state
-  **Proofs**: Merkle proofs for verifying account states
-  **Updates**: Atomic updates to the tree

For more details, see [DOCUMENTATION.md](DOCUMENTATION.md#sparse-merkle-tree-smt).

### Account Model

Volt uses an account-based model:

-  **Address**: 32-byte identifier derived from the public key
-  **Balance**: Amount of tokens owned by the account
-  **Nonce**: Counter to prevent replay attacks
-  **Token ID**: ID of the token (for token-specific accounts)

For more details, see [DOCUMENTATION.md](DOCUMENTATION.md#account-model).

### Token System

Volt supports multiple tokens:

-  **Native Token**: VOLT (token ID 0)
-  **Custom Tokens**: User-created tokens (token ID > 0)
-  **Token Operations**: Issue, mint, transfer, burn

For more details, see [TOKEN_SYSTEM.md](TOKEN_SYSTEM.md).

### Network Protocol

Volt uses a peer-to-peer network based on libp2p:

-  **DHT**: Distributed Hash Table for peer discovery
-  **Gossip**: Protocol for message propagation
-  **Transport**: Layer for establishing connections

For more details, see [DOCUMENTATION.md](DOCUMENTATION.md#network-layer).

## API Reference

### JSON-RPC API

The Volt node provides a JSON-RPC API for interacting with the network. For a complete reference, see [RPC_DOCUMENTATION.md](RPC_DOCUMENTATION.md).

#### Key Endpoints

-  **Account Methods**: `getBalance`, `getNonce`, etc.
-  **Transaction Methods**: `send`, `mint`, etc.
-  **Token Methods**: `p3p_issueToken`, `p3p_mintToken`, etc.
-  **State Methods**: `getRoot`, `getProof`, etc.

### CLI API

The Volt CLI provides a command-line interface for interacting with the network. For a complete reference, see [CLI_GUIDE.md](CLI_GUIDE.md).

#### Key Commands

-  **Wallet Commands**: `init-seed`, `export-seed`, etc.
-  **Account Commands**: `balance`, etc.
-  **Transaction Commands**: `send`, etc.
-  **Token Commands**: `issue-token`, `mint-token`, etc.

### Rust API

The Volt codebase provides Rust APIs for programmatic interaction:

#### Core API

```rust
use core::{smt::SMT, types::Address};

// Create a new SMT
let mut smt = SMT::new_zero();

// Get an account
let account = smt.get_account(&address)?;

// Update an account
smt.update_account(account)?;

// Generate a proof
let proof = smt.gen_proof(&address)?;
```

#### Network API

```rust
use network::{dht::DHTManager, transport::init_swarm};

// Initialize a swarm
let (swarm, dht_manager) = init_swarm(bootstrap_nodes).await?;

// Start listening
swarm.listen_on(listen_addr)?;

// Handle events
while let Some(event) = swarm.next().await {
    // Handle the event
}
```

## Integration Guide

### Integrating with Web Applications

1. Use the JSON-RPC API to interact with the Volt network
2. Implement client-side key management
3. Handle transaction creation and signing
4. Monitor transaction status

Example using JavaScript:

```javascript
// Connect to the Volt network
const voltClient = new VoltClient('http://3.90.180.149:8545/rpc');

// Get balance
const balance = await voltClient.getBalance('0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef');

// Send transaction
const txHash = await voltClient.send(privateKey, '0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890', 100);
```

### Integrating with Mobile Applications

1. Use a library that supports the JSON-RPC API
2. Implement secure key storage
3. Handle offline transaction signing
4. Provide a user-friendly interface

### Integrating with Ethereum

1. Use the Ethereum Bridge
2. Implement cross-chain token transfers
3. Monitor bridge events
4. Handle bridge operations

For more details, see [ETHEREUM_BRIDGE.md](ETHEREUM_BRIDGE.md).

## Contributing

### Code Style

The Volt project follows the Rust style guide:

-  Use `rustfmt` for formatting
-  Use `clippy` for linting
-  Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

### Pull Request Process

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests
5. Submit a pull request

### Code Review

All pull requests are reviewed by maintainers. The review process includes:

1. Code quality review
2. Functionality testing
3. Documentation review
4. Security review

## Testing

### Unit Tests

Unit tests are located alongside the code they test. Run unit tests with:

```bash
cargo test
```

### Integration Tests

Integration tests are located in the `src/tests` directory. Run integration tests with:

```bash
cargo test --package tests
```

### End-to-End Tests

End-to-end tests simulate real-world usage scenarios. Run end-to-end tests with:

```bash
cargo test --package tests -- --ignored
```

### Performance Testing

Performance tests measure the performance of the system. Run performance tests with:

```bash
cargo bench
```

## Documentation

### Code Documentation

The Volt codebase is documented using Rust doc comments. Generate the documentation with:

```bash
cargo doc --open
```

### User Documentation

User documentation is available in the repository:

-  [README.md](README.md): Project overview
-  [DOCUMENTATION.md](DOCUMENTATION.md): Technical documentation
-  [RPC_DOCUMENTATION.md](RPC_DOCUMENTATION.md): RPC API documentation
-  [CLI_GUIDE.md](CLI_GUIDE.md): CLI usage guide
-  [NODE_SETUP.md](NODE_SETUP.md): Node setup guide
-  [TOKEN_SYSTEM.md](TOKEN_SYSTEM.md): Token system documentation
-  [ETHEREUM_BRIDGE.md](ETHEREUM_BRIDGE.md): Ethereum bridge documentation
-  [ARCHITECTURE.md](ARCHITECTURE.md): Architecture documentation
-  [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md): Project structure documentation
-  [DEVELOPER_GUIDE.md](DEVELOPER_GUIDE.md): This file

### API Documentation

API documentation is available in the [RPC_DOCUMENTATION.md](RPC_DOCUMENTATION.md) file.

## Resources

### Community

-  **Discord**: [https://discord.gg/NcKvqbwg](https://discord.gg/NcKvqbwg)
-  **GitHub**: [https://github.com/volt/volt](https://github.com/volt/volt)
-  **Website**: [https://voltnetwork.org](https://voltnetwork.org)

### Learning Resources

-  **Rust Book**: [https://doc.rust-lang.org/book/](https://doc.rust-lang.org/book/)
-  **libp2p Documentation**: [https://docs.libp2p.io/](https://docs.libp2p.io/)
-  **Sparse Merkle Tree**: [https://github.com/nervosnetwork/sparse-merkle-tree](https://github.com/nervosnetwork/sparse-merkle-tree)

### Tools

-  **Volt Explorer**: [https://explorer.voltnetwork.org](https://explorer.voltnetwork.org)
-  **Volt Wallet**: [https://app.voltnetwork.org](https://app.voltnetwork.org)
-  **Volt Bridge UI**: [https://bridge.voltnetwork.org](https://bridge.voltnetwork.org)

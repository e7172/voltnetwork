# Volt Project Structure

This document provides a detailed overview of the Volt project structure, explaining the purpose of each directory and file.

## Root Directory

```
/
├── Cargo.toml         # Workspace configuration
├── Cargo.lock         # Dependency lock file
├── README.md          # Project overview
├── DOCUMENTATION.md   # Technical documentation
├── RPC_DOCUMENTATION.md # RPC API documentation
├── PROJECT_STRUCTURE.md # This file
├── cover.png          # Project cover image
├── volt-architecture.png # Architecture diagram
├── volt-cover.png     # Project cover image
└── src/               # Source code
```

## Source Code Structure

The source code is organized into several modules, each with its own purpose:

```
src/
├── core/              # Core functionality
├── network/           # Networking layer
├── node/              # Node implementation
├── cli/               # Command-line interface
├── bridge/            # Ethereum bridge
└── tests/             # Test suite
```

### Core Module

The core module contains the fundamental data structures and algorithms used by the Volt network.

```
src/core/
├── Cargo.toml         # Module configuration
└── src/
    ├── lib.rs         # Module entry point
    ├── smt.rs         # Sparse Merkle Tree implementation
    ├── proofs.rs      # Cryptographic proofs
    ├── types.rs       # Core data types
    └── errors.rs      # Error types
```

#### Key Files

-  **smt.rs**: Implements the Sparse Merkle Tree (SMT) data structure, which is used to store account states and generate proofs.
-  **proofs.rs**: Implements cryptographic proofs for verifying account states without requiring the entire state tree.
-  **types.rs**: Defines core data types such as Address, Balance, TokenId, and AccountLeaf.
-  **errors.rs**: Defines error types for the core module.

### Network Module

The network module handles peer-to-peer communication and data storage.

```
src/network/
├── Cargo.toml         # Module configuration
└── src/
    ├── lib.rs         # Module entry point
    ├── dht.rs         # Distributed Hash Table implementation
    ├── gossip.rs      # Gossip protocol implementation
    ├── transport.rs   # Transport layer implementation
    ├── storage.rs     # Data storage implementation
    ├── types.rs       # Network message types
    └── errors.rs      # Error types
```

#### Key Files

-  **dht.rs**: Implements the Distributed Hash Table (DHT) for peer discovery and data storage.
-  **gossip.rs**: Implements the gossip protocol for message propagation.
-  **transport.rs**: Implements the transport layer for establishing connections between nodes.
-  **storage.rs**: Implements data storage for proofs and other network data.
-  **types.rs**: Defines network message types such as UpdateMsg and MintMsg.
-  **errors.rs**: Defines error types for the network module.

### Node Module

The node module implements the Volt node, which runs the network protocol and provides an RPC API.

```
src/node/
├── Cargo.toml         # Module configuration
└── src/
    ├── lib.rs         # Module entry point
    ├── main.rs        # Node entry point
    ├── rpc.rs         # RPC server implementation
    ├── config.rs      # Node configuration
    ├── metrics.rs     # Metrics collection
    ├── tests.rs       # Node tests
    └── errors.rs      # Error types
```

#### Key Files

-  **main.rs**: The main entry point for the node, which initializes the node and starts the RPC server.
-  **rpc.rs**: Implements the JSON-RPC API for interacting with the node.
-  **config.rs**: Defines the node configuration options.
-  **metrics.rs**: Implements metrics collection for monitoring node performance.
-  **errors.rs**: Defines error types for the node module.

### CLI Module

The CLI module provides a command-line interface for interacting with the Volt network.

```
src/cli/
├── Cargo.toml         # Module configuration
└── src/
    ├── lib.rs         # Module entry point
    ├── main.rs        # CLI entry point
    ├── config.rs      # CLI configuration
    ├── wallet.rs      # Wallet implementation
    ├── errors.rs      # Error types
    └── commands/      # CLI commands
        ├── mod.rs     # Commands module entry point
        ├── balance.rs # Balance command
        ├── export_seed.rs # Export seed command
        ├── init_seed.rs # Initialize seed command
        ├── issue_token.rs # Issue token command
        ├── mint.rs    # Mint command
        ├── mint_token.rs # Mint token command
        └── send.rs    # Send command
```

#### Key Files

-  **main.rs**: The main entry point for the CLI, which parses command-line arguments and executes commands.
-  **config.rs**: Defines the CLI configuration options.
-  **wallet.rs**: Implements the wallet functionality for managing keys and signing transactions.
-  **errors.rs**: Defines error types for the CLI module.
-  **commands/**: Contains implementations of CLI commands.

### Bridge Module

The bridge module implements the Ethereum bridge for cross-chain token transfers.

```
src/bridge/
├── Cargo.toml         # Module configuration
├── contracts/         # Ethereum smart contracts
│   ├── ETHBridge.sol  # Bridge contract
│   └── ETHBridge.abi  # Bridge contract ABI
└── src/
    ├── lib.rs         # Module entry point
    ├── bridge.rs      # Bridge implementation
    ├── bindings.rs    # Ethereum contract bindings
    └── errors.rs      # Error types
```

#### Key Files

-  **bridge.rs**: Implements the bridge functionality for transferring tokens between Volt and Ethereum.
-  **bindings.rs**: Generated bindings for interacting with the Ethereum smart contract.
-  **errors.rs**: Defines error types for the bridge module.
-  **contracts/ETHBridge.sol**: The Ethereum smart contract for the bridge.
-  **contracts/ETHBridge.abi**: The ABI for the Ethereum smart contract.

### Tests Module

The tests module contains integration tests for the Volt network.

```
src/tests/
├── Cargo.toml         # Module configuration
└── src/
    ├── lib.rs         # Module entry point
    ├── bridge_tests.rs # Bridge tests
    ├── core_tests.rs  # Core tests
    ├── network_tests.rs # Network tests
    ├── node_tests.rs  # Node tests
    └── token_tests.rs # Token tests
```

#### Key Files

-  **bridge_tests.rs**: Tests for the Ethereum bridge.
-  **core_tests.rs**: Tests for the core functionality.
-  **network_tests.rs**: Tests for the networking layer.
-  **node_tests.rs**: Tests for the node implementation.
-  **token_tests.rs**: Tests for token functionality.

## Dependencies

The Volt project uses several external dependencies, which are defined in the workspace Cargo.toml file:

### Core Dependencies

-  **sha2**: SHA-2 cryptographic hash functions
-  **ed25519-dalek**: Ed25519 digital signatures
-  **sparse-merkle-tree**: Sparse Merkle Tree implementation
-  **byteorder**: Reading and writing numbers in big-endian and little-endian
-  **thiserror**: Error handling
-  **anyhow**: Error handling
-  **log**: Logging
-  **tracing**: Structured logging
-  **serde**: Serialization and deserialization
-  **serde_json**: JSON serialization and deserialization
-  **bincode**: Binary serialization and deserialization
-  **bitvec**: Bit vectors

### Network Dependencies

-  **libp2p**: Peer-to-peer networking
-  **rocksdb**: Persistent key-value store
-  **tokio**: Asynchronous runtime
-  **futures**: Asynchronous programming
-  **async-trait**: Asynchronous traits

### CLI Dependencies

-  **structopt**: Command-line argument parsing
-  **bip32**: BIP32 hierarchical deterministic wallets
-  **rand**: Random number generation
-  **colored**: Colored terminal output

### Bridge Dependencies

-  **ethers**: Ethereum library
-  **hex**: Hexadecimal encoding and decoding

### Testing Dependencies

-  **tempfile**: Temporary file creation
-  **serial_test**: Serial test execution

## Build System

The Volt project uses Cargo, the Rust package manager, for building and testing. The workspace configuration in the root Cargo.toml file defines the project structure and dependencies.

## Configuration

The Volt project uses configuration files for the node and CLI:

-  **Node Configuration**: Defined in `src/node/src/config.rs`
-  **CLI Configuration**: Defined in `src/cli/src/config.rs`

## Testing

The Volt project includes a comprehensive test suite in the `src/tests` directory. The tests cover all aspects of the project, including the core functionality, networking layer, node implementation, and Ethereum bridge.

## Documentation

The Volt project includes several documentation files:

-  **README.md**: Project overview and getting started guide
-  **DOCUMENTATION.md**: Technical documentation
-  **RPC_DOCUMENTATION.md**: RPC API documentation
-  **PROJECT_STRUCTURE.md**: This file

## License

The Volt project is licensed under the MIT License.

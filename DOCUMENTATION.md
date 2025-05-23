# Volt Technical Documentation

This document provides detailed technical information about the Volt network architecture, components, and implementation.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Core Components](#core-components)
   -  [Sparse Merkle Tree (SMT)](#sparse-merkle-tree-smt)
   -  [Proofs](#proofs)
   -  [Types](#types)
3. [Network Layer](#network-layer)
   -  [Distributed Hash Table (DHT)](#distributed-hash-table-dht)
   -  [Gossip Protocol](#gossip-protocol)
   -  [Transport](#transport)
4. [Node Implementation](#node-implementation)
   -  [State Management](#state-management)
   -  [RPC Server](#rpc-server)
   -  [Consensus](#consensus)
5. [CLI Wallet](#cli-wallet)
   -  [Commands](#commands)
   -  [Wallet Management](#wallet-management)
6. [Ethereum Bridge](#ethereum-bridge)
7. [Token System](#token-system)
8. [Security Considerations](#security-considerations)
9. [Performance Optimizations](#performance-optimizations)
10.   [Future Developments](#future-developments)

## Architecture Overview

Volt is a chainless token transfer network that uses Sparse Merkle Trees (SMTs) for state management and cryptographic proofs for transaction verification. Unlike traditional blockchain networks, Volt does not maintain a global ledger of all transactions. Instead, it uses a distributed state tree where each node stores only the current state of accounts.

The key architectural components are:

1. **State Tree**: A Sparse Merkle Tree that stores account balances and nonces
2. **Proof System**: Cryptographic proofs that allow for stateless verification of transactions
3. **Network Layer**: P2P network for node discovery and message propagation
4. **RPC Server**: JSON-RPC API for interacting with the network
5. **CLI Wallet**: Command-line interface for managing accounts and tokens
6. **Ethereum Bridge**: Bridge for cross-chain token transfers

## Core Components

### Sparse Merkle Tree (SMT)

The Sparse Merkle Tree (SMT) is the core data structure used to store account states in Volt. It is implemented in `src/core/src/smt.rs`.

Key features of the SMT implementation:

-  **Account Storage**: Each leaf in the tree represents an account with a balance and nonce
-  **Multi-Token Support**: Accounts can hold multiple tokens, each with its own balance and nonce
-  **Persistence**: The SMT is persisted to disk using RocksDB
-  **Proofs**: The SMT can generate proofs of inclusion for any account

The SMT uses a 256-bit address space, allowing for a large number of accounts. Each account is identified by a 32-byte address, which is used as the key in the SMT.

#### SMT Operations

-  `update_account`: Updates an account in the tree
-  `get_account`: Gets an account from the tree
-  `gen_proof`: Generates a proof for an account
-  `verify_proof`: Verifies a proof for an account
-  `transfer`: Transfers tokens between accounts
-  `mint_token`: Mints new tokens for a specific token ID
-  `register_token`: Registers a new token in the system

### Proofs

Proofs are used to verify account states without requiring the entire state tree. They are implemented in `src/core/src/proofs.rs`.

A proof consists of:

-  **Siblings**: The sibling hashes along the path from the leaf to the root
-  **Leaf Hash**: The hash of the leaf being proven
-  **Path**: The path from the root to the leaf (as a sequence of bits)
-  **Zeros Omitted**: Number of trailing zero-siblings that were omitted

Proofs can be verified by reconstructing the root hash from the leaf hash and siblings, and comparing it to the expected root hash.

### Types

The core types used in Volt are defined in `src/core/src/types.rs`:

-  **Address**: A 32-byte array representing an account address
-  **Balance**: A 128-bit unsigned integer representing a token balance
-  **TokenId**: A 64-bit unsigned integer representing a token ID
-  **Nonce**: A 64-bit unsigned integer representing a transaction nonce
-  **Signature**: A 64-byte array representing an Ed25519 signature
-  **AccountLeaf**: A struct representing an account in the SMT
-  **TokenInfo**: A struct representing token metadata
-  **SystemMsg**: An enum representing system messages (Mint, IssueToken, etc.)

## Network Layer

The network layer is responsible for node discovery, message propagation, and data storage. It is implemented in the `src/network` directory.

### Distributed Hash Table (DHT)

The DHT is used for peer discovery and data storage. It is implemented using the Kademlia protocol from libp2p.

Key features:

-  **Peer Discovery**: Nodes can discover other nodes in the network
-  **Data Storage**: Nodes can store and retrieve data from the network
-  **Routing**: Nodes can route messages to other nodes in the network

### Gossip Protocol

The gossip protocol is used for message propagation. It is implemented using the GossipSub protocol from libp2p.

Key features:

-  **Message Propagation**: Messages are propagated to all nodes in the network
-  **Topic-Based**: Messages are published to specific topics
-  **Efficient**: Messages are only sent to nodes that are subscribed to the topic

### Transport

The transport layer is responsible for establishing connections between nodes. It supports TCP, WebSockets, and other transport protocols.

## Node Implementation

The node implementation is responsible for running a Volt node. It is implemented in the `src/node` directory.

### State Management

The node maintains a local copy of the state tree. It synchronizes with other nodes in the network to ensure that its state is up-to-date.

Key features:

-  **State Synchronization**: Nodes can synchronize their state with other nodes
-  **State Verification**: Nodes can verify the state using cryptographic proofs
-  **State Persistence**: The state is persisted to disk using RocksDB

### RPC Server

The RPC server provides a JSON-RPC API for interacting with the node. It is implemented in `src/node/src/rpc.rs`.

Available RPC methods:

-  **Account and Balance Methods**: `getRoot`, `getBalance`, `getBalanceWithToken`, `getAllBalances`, `getNonce`, `get_nonce_with_token`
-  **Proof Methods**: `getProof`, `get_proof_with_token`
-  **Token Methods**: `get_tokens`, `p3p_issueToken`, `p3p_mintToken`, `get_total_supply`, `get_max_supply`
-  **Transaction Methods**: `send`, `mint`, `broadcastUpdate`, `broadcast_mint`
-  **State Methods**: `get_full_state`, `set_full_state`
-  **Network Methods**: `get_peer_id`

### Consensus

Volt uses a consensus mechanism based on cryptographic proofs and state synchronization. Nodes verify transactions using proofs and synchronize their state with other nodes in the network.

Key features:

-  **Proof Verification**: Transactions are verified using cryptographic proofs
-  **State Synchronization**: Nodes synchronize their state with other nodes
-  **Conflict Resolution**: Conflicts are resolved using a consensus score based on account activity, nonce values, and total balance

## CLI Wallet

The CLI wallet provides a command-line interface for interacting with the Volt network. It is implemented in the `src/cli` directory.

### Commands

The CLI wallet supports the following commands:

-  **init-seed**: Initializes a new wallet seed
-  **export-seed**: Exports the wallet seed
-  **balance**: Gets the balance of the wallet
-  **send**: Sends tokens to another address
-  **mint**: Mints native tokens (treasury only)
-  **issue-token**: Issues a new token
-  **mint-token**: Mints tokens for a specific token ID

### Wallet Management

The wallet is managed using a seed phrase, which is used to derive the private key. The private key is used to sign transactions.

Key features:

-  **Seed Generation**: Generates a new seed phrase
-  **Key Derivation**: Derives the private key from the seed phrase
-  **Transaction Signing**: Signs transactions using the private key
-  **Address Generation**: Generates the address from the public key

## Ethereum Bridge

The Ethereum bridge allows for cross-chain token transfers between Volt and Ethereum. It is implemented in the `src/bridge` directory.

Key features:

-  **Token Bridging**: Tokens can be transferred between Volt and Ethereum
-  **Smart Contract**: An Ethereum smart contract handles the token transfers on the Ethereum side
-  **Event Monitoring**: The bridge monitors Ethereum events for token transfers
-  **Proof Verification**: The bridge verifies proofs for token transfers

## Token System

Volt supports multiple tokens on the same network. Each token has its own ID, metadata, and issuer.

Key features:

-  **Token Registration**: New tokens can be registered with metadata
-  **Token Minting**: Token issuers can mint new tokens
-  **Token Transfers**: Tokens can be transferred between accounts
-  **Token Metadata**: Tokens have metadata including name, symbol, and decimals

## Security Considerations

Volt implements several security measures to ensure the integrity and privacy of the network:

-  **Cryptographic Proofs**: Transactions are verified using cryptographic proofs
-  **Signature Verification**: All transactions are signed and verified
-  **Nonce Tracking**: Nonces are used to prevent replay attacks
-  **State Verification**: The state is verified using cryptographic proofs
-  **Privacy**: Account balances are only known to the account owner and those they share proofs with

## Performance Optimizations

Volt includes several performance optimizations:

-  **Sparse Merkle Tree**: Efficient storage and retrieval of account states
-  **Proof Compression**: Proofs are compressed to reduce network overhead
-  **State Synchronization**: Efficient state synchronization between nodes
-  **RocksDB**: Efficient storage of the state tree on disk
-  **Caching**: Account states are cached in memory for faster access

## Future Developments

Planned future developments for Volt include:

-  **Smart Contracts**: Support for smart contracts on the Volt network
-  **Privacy Enhancements**: Additional privacy features for transactions
-  **Scalability Improvements**: Further optimizations for scalability
-  **Cross-Chain Interoperability**: Support for more blockchain networks
-  **Governance**: Decentralized governance for the Volt network

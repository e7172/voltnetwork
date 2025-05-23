# Volt: Chainless Token Transfer Network

![Volt Network Cover](volt-cover.png)

## Overview

Volt is a novel decentralized protocol enabling instant, feeless, and privacy-preserving asset transfers without a global blockchain ledger. Leveraging Sparse Merkle Trees (SMTs) for cryptographic proofs and Distributed Hash Tables (DHTs) for peer-to-peer data storage and retrieval, Volt reduces network overhead, eliminates transaction fees, and enhances user privacy.

This architecture provides stateless verification and scalability while maintaining robust security and data integrity guarantees.

**Website:** [https://voltnetwork.org](https://voltnetwork.org)

**Wallet:** [https://app.voltnetwork.org/](https://app.voltnetwork.org/)

**Join our Discord:** [https://discord.gg/NcKvqbwg](https://discord.gg/NcKvqbwg)

## Key Features

-  **Chainless Architecture**: No blockchain, just a distributed state tree
-  **Multi-Token Support**: Create and manage your own tokens
-  **Ethereum Bridge**: Bridge tokens to and from Ethereum
-  **CLI Wallet**: Command-line interface for managing accounts and tokens
-  **Feeless Transactions**: No transaction fees for transfers
-  **Privacy-Preserving**: Enhanced privacy through cryptographic proofs
-  **Scalable**: Stateless verification for improved scalability

## Project Structure

The Volt project is organized into several key components:

-  **Core**: Core functionality including Sparse Merkle Tree implementation, proofs, and types
-  **Network**: Networking layer with DHT, gossip protocol, and transport
-  **Node**: Node implementation with RPC server and state management
-  **CLI**: Command-line interface for interacting with the network
-  **Bridge**: Ethereum bridge for cross-chain token transfers
-  **Tests**: Test suite for all components

## Getting Started

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

3. Initialize a new wallet:

```bash
./target/release/cli init-seed
```

### Running a Node

To run a node and connect to the Volt network:

1. Create a directory for the node data:

```bash
mkdir -p ~/.volt/node1
```

2. Start the node:

```bash
RUST_LOG=debug ./target/release/node --rpc --data-dir ~/.volt/node1 --rpc-addr 0.0.0.0:8545 --listen /ip4/0.0.0.0/tcp/30333 --bootstrap /ip4/3.90.180.149/tcp/30333/p2p/12D3KooWQLBSdMgmnicekuD8w9Lsy5CWVuNJcCBctxNeK3YsrDKF
```

### Using the CLI Wallet

The CLI wallet provides a comprehensive set of commands for interacting with the Volt network.

#### Initialize a Wallet

```bash
./target/release/cli init-seed
```

This will generate a new keypair and save it to the default location. The output will show your address and the path where the seed was stored.

#### Export Your Seed

```bash
./target/release/cli export-seed
```

**WARNING**: Keep this seed safe and private!

#### Check Your Balance

```bash
./target/release/cli balance
```

This will display your balance for all tokens you own.

#### Send Tokens

```bash
./target/release/cli send --to <ADDRESS> --amount <AMOUNT> --token_id <TOKEN_ID>
```

-  `<ADDRESS>`: The recipient's address (32-byte hex string)
-  `<AMOUNT>`: The amount to send
-  `<TOKEN_ID>`: (Optional) The token ID to send (defaults to 0 for native VOLT token)

#### Issue a New Token

```bash
./target/release/cli issue-token --metadata "Name|Symbol|Decimals"
```

This will register a new token with the specified metadata and assign you as the issuer.

#### Mint Tokens

```bash
./target/release/cli mint-token --token-id <TOKEN_ID> --to <ADDRESS> --amount <AMOUNT>
```

Note: Only the token issuer can mint new tokens.

## RPC Endpoints

The Volt node provides a JSON-RPC API for interacting with the network. The default RPC endpoint is `http://localhost:8545/rpc`.

### Official RPC Node Endpoint

```
http://3.90.180.149:8545
```

### Official Peer ID

```
12D3KooWQLBSdMgmnicekuD8w9Lsy5CWVuNJcCBctxNeK3YsrDKF
```

### Available RPC Methods

#### Account and Balance Methods

| Method                 | Parameters            | Description                                                      |
| ---------------------- | --------------------- | ---------------------------------------------------------------- |
| `getRoot`              | None                  | Returns the current root hash of the state tree                  |
| `getBalance`           | `[address]`           | Returns the balance of the native token for the given address    |
| `getBalanceWithToken`  | `[address, token_id]` | Returns the balance of the specified token for the given address |
| `getAllBalances`       | `[address]`           | Returns all token balances for the given address                 |
| `getNonce`             | `[address]`           | Returns the current nonce for the given address                  |
| `get_nonce_with_token` | `[address, token_id]` | Returns the current nonce for the given address and token        |

#### Proof Methods

| Method                 | Parameters            | Description                                     |
| ---------------------- | --------------------- | ----------------------------------------------- |
| `getProof`             | `[address]`           | Returns a proof for the given address           |
| `get_proof_with_token` | `[address, token_id]` | Returns a proof for the given address and token |

#### Token Methods

| Method             | Parameters  | Description                                    |
| ------------------ | ----------- | ---------------------------------------------- |
| `get_tokens`       | None        | Returns a list of all tokens                   |
| `p3p_issueToken`   | `[message]` | Issues a new token                             |
| `p3p_mintToken`    | `[message]` | Mints tokens for a specific token ID           |
| `get_total_supply` | None        | Returns the total supply of the native token   |
| `get_max_supply`   | None        | Returns the maximum supply of the native token |

#### Transaction Methods

| Method            | Parameters                                       | Description                                 |
| ----------------- | ------------------------------------------------ | ------------------------------------------- |
| `send`            | `[from, to, token_id, amount, nonce, signature]` | Sends tokens from one address to another    |
| `mint`            | `[from, signature, to, amount]`                  | Mints native tokens (treasury only)         |
| `broadcastUpdate` | `[message]`                                      | Broadcasts an update message to the network |
| `broadcast_mint`  | `[message]`                                      | Broadcasts a mint message to the network    |

#### State Methods

| Method           | Parameters | Description                           |
| ---------------- | ---------- | ------------------------------------- |
| `get_full_state` | None       | Returns the full state of the network |
| `set_full_state` | `[state]`  | Sets the full state of the network    |

#### Network Methods

| Method        | Parameters | Description                     |
| ------------- | ---------- | ------------------------------- |
| `get_peer_id` | None       | Returns the peer ID of the node |

### Example RPC Requests

#### Get Balance

```json
{
   "jsonrpc": "2.0",
   "method": "getBalance",
   "params": ["0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"],
   "id": 1
}
```

#### Send Transaction

```json
{
   "jsonrpc": "2.0",
   "method": "send",
   "params": [
      "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
      "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
      0,
      100,
      1,
      "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
   ],
   "id": 1
}
```

#### Issue Token

```json
{
   "jsonrpc": "2.0",
   "method": "p3p_issueToken",
   "params": ["0x..."], // Hex-encoded serialized message
   "id": 1
}
```

## Technical Architecture

### Sparse Merkle Tree (SMT)

Volt uses a Sparse Merkle Tree (SMT) to store and verify account states. The SMT allows for efficient proofs of inclusion, which are used to verify account balances and nonces without requiring a global ledger.

### Distributed Hash Table (DHT)

The network uses a Distributed Hash Table (DHT) for peer discovery and data storage. This allows nodes to efficiently find and retrieve data from the network without requiring a central server.

### Cryptographic Proofs

Volt uses cryptographic proofs to verify account states and transactions. These proofs allow for stateless verification, meaning that nodes do not need to store the entire state tree to verify transactions.

### Multi-Token Support

Volt supports multiple tokens on the same network. Each token has its own ID, metadata, and issuer. Token issuers can mint new tokens and manage their supply.

### Ethereum Bridge

The Ethereum bridge allows for cross-chain token transfers between Volt and Ethereum. This enables interoperability with the broader Ethereum ecosystem.

## Development

### Prerequisites

-  Rust 1.60 or later
-  Cargo
-  RocksDB

### Building from Source

```bash
cargo build --release
```

### Running Tests

```bash
cargo test
```

## Contributing

We welcome contributions to the Volt project! Please feel free to submit issues and pull requests.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

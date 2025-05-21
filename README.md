# Volt: Chainless Token Transfer Network

Volt is a novel decentralized protocol enabling instant, feeless, and privacy-preserving asset transfers without a global blockchain ledger. Leveraging Sparse Merkle Trees (SMTs) for cryptographic proofs and Distributed Hash Tables (DHTs) for peer-to-peer data storage and retrieval, Volt reduces network overhead, eliminates transaction fees, and enhances user privacy.

This architecture provides stateless verification and scalability while maintaining robust security and data integrity guarantees.

Currently, Volt is under active development and in Beta phase. We are actively seeking early adopters and contributors to help shape the future of Volt.

## OFFICIAL RPC NODE ENDPOINT:

http://3.90.180.149:8545

## PEER ID

12D3KooWRnHujasvH4mk62xyJuf8msEgAkmBowW8HvU3wkJuvArX

## Features

-  **Chainless Architecture**: No blockchain, just a distributed state tree
-  **Multi-Token Support**: Create and manage your own tokens
-  **Ethereum Bridge**: Bridge tokens to and from Ethereum
-  **CLI Wallet**: Command-line interface for managing accounts and tokens

## Getting Started

### Installation

1. Clone the repository:

```bash
git clone https://github.com/username/volt.git
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

### Running the First Node (Genesis Node)

To initialize the network:

1. Create a directory for the node data:

```bash
mkdir -p ~/.volt/node1
```

RUST_LOG=debug ./target/release/node --rpc --data-dir ~/.volt/node1 --rpc-addr 0.0.0.0:8545 --listen /ip4/0.0.0.0/tcp/30333 --bootstrap /ip4/3.90.180.149/tcp/30333/p2p/12D3KooWRnHujasvH4mk62xyJuf8msEgAkmBowW8HvU3wkJuvArX

### Using the CLI Wallet

First, create a wallet:

```bash
./target/release/cli init-seed
```

This will generate a new keypair and save it to the default location. The output will show your address and the path where the seed was stored.

Get help:

```bash
./target/release/cli -help
```

./target/release/cli init-seed

./target/release/cli export-seed

./target/release/cli balance

request-tokens --amount 1000

./target/release/cli issue-token --metadata A,A,A

#### Check your balance:

```bash
./target/release/cli balance
```

#### Send tokens:

```bash
./target/release/cli send --to address --amount 100
```

#### Issue a new token:

```bash
./target/release/cli issue-token --metadata "My Token|MTK|18"
```

This will register a new token with the specified metadata and assign you as the issuer.

#### Mint tokens:

```bash
./target/release/cli mint-token --token-id 1 --to 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef --amount 1000
```

Note: Only the token issuer can mint new tokens.

#### Burn tokens:

```bash
./target/release/cli burn-token --token-id 1 --amount 500
```

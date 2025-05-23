# Volt Ethereum Bridge

This document provides a detailed explanation of the Volt Ethereum Bridge, which enables cross-chain token transfers between the Volt network and Ethereum.

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Components](#components)
   -  [Volt Bridge Module](#volt-bridge-module)
   -  [Ethereum Smart Contract](#ethereum-smart-contract)
   -  [Relayer Service](#relayer-service)
4. [Token Bridging](#token-bridging)
   -  [Volt to Ethereum](#volt-to-ethereum)
   -  [Ethereum to Volt](#ethereum-to-volt)
5. [Security Model](#security-model)
6. [Configuration](#configuration)
7. [Usage](#usage)
   -  [CLI Commands](#cli-commands)
   -  [RPC Methods](#rpc-methods)
8. [Monitoring](#monitoring)
9. [Troubleshooting](#troubleshooting)
10.   [Future Developments](#future-developments)

## Overview

The Volt Ethereum Bridge enables seamless cross-chain token transfers between the Volt network and Ethereum. This allows users to leverage the benefits of both networks: the feeless and instant transactions of Volt, and the rich ecosystem and liquidity of Ethereum.

Key features of the bridge:

-  **Bidirectional Transfers**: Tokens can be transferred in both directions
-  **Multi-Token Support**: Support for both native VOLT and custom tokens
-  **Security**: Cryptographic proofs ensure secure transfers
-  **Decentralization**: No central authority controls the bridge
-  **Transparency**: All bridge operations are verifiable on both chains

## Architecture

The bridge uses a lock-and-mint/burn-and-release model:

1. **Volt to Ethereum**: Tokens are locked on Volt and minted on Ethereum
2. **Ethereum to Volt**: Tokens are burned on Ethereum and released on Volt

The bridge relies on cryptographic proofs to verify cross-chain operations, ensuring that tokens can only be minted on the destination chain if they have been locked or burned on the source chain.

## Components

### Volt Bridge Module

The Volt Bridge Module is implemented in `src/bridge/src/bridge.rs` and is responsible for:

1. **Locking Tokens**: When transferring from Volt to Ethereum
2. **Releasing Tokens**: When transferring from Ethereum to Volt
3. **Generating Proofs**: For verifying operations on Ethereum
4. **Verifying Proofs**: From Ethereum operations
5. **Monitoring Ethereum**: For bridge-related events

### Ethereum Smart Contract

The Ethereum Smart Contract is implemented in `src/bridge/contracts/ETHBridge.sol` and is responsible for:

1. **Minting Tokens**: When transferring from Volt to Ethereum
2. **Burning Tokens**: When transferring from Ethereum to Volt
3. **Verifying Proofs**: From Volt operations
4. **Generating Events**: For the Volt network to monitor
5. **Managing Token Mappings**: Between Volt and Ethereum tokens

### Relayer Service

The Relayer Service is an optional component that can automate the bridging process by:

1. **Monitoring Both Chains**: For bridge-related events
2. **Submitting Proofs**: To the destination chain
3. **Handling Fees**: For Ethereum transactions
4. **Retrying Failed Transactions**: To ensure reliability

## Token Bridging

### Volt to Ethereum

To bridge tokens from Volt to Ethereum:

1. **Lock Tokens**: The user locks tokens on the Volt network by calling the bridge function
2. **Generate Proof**: The Volt node generates a proof of the lock
3. **Submit Proof**: The proof is submitted to the Ethereum smart contract
4. **Mint Tokens**: The Ethereum smart contract verifies the proof and mints equivalent tokens

Example using the CLI:

```bash
./target/release/cli bridge-to-eth --token-id 1 --amount 100 --eth-address 0x1234567890abcdef1234567890abcdef12345678
```

### Ethereum to Volt

To bridge tokens from Ethereum to Volt:

1. **Burn Tokens**: The user burns tokens on Ethereum by calling the bridge function
2. **Generate Proof**: The Ethereum smart contract generates an event with the burn proof
3. **Submit Proof**: The proof is submitted to the Volt network
4. **Release Tokens**: The Volt node verifies the proof and releases the locked tokens

Example using the Ethereum contract:

```solidity
// Using web3.js or ethers.js
bridge.bridgeToVolt(tokenAddress, amount, voltAddress);
```

## Security Model

The bridge's security model is based on cryptographic proofs and relies on the security of both the Volt network and Ethereum:

1. **Cryptographic Proofs**: All bridge operations are verified using cryptographic proofs
2. **Consensus**: The Volt network and Ethereum both have their own consensus mechanisms
3. **Threshold Signatures**: Optional multi-signature scheme for additional security
4. **Rate Limiting**: Limits on the amount of tokens that can be bridged
5. **Timeouts**: Timeouts for bridge operations to prevent funds from being locked indefinitely

### Security Considerations

-  **Finality**: Ethereum has probabilistic finality, so bridge operations may need to wait for a certain number of confirmations
-  **Smart Contract Security**: The Ethereum smart contract must be secure against attacks
-  **Relayer Security**: If using a relayer service, it must be secure and reliable
-  **Network Security**: Both networks must be secure for the bridge to be secure

## Configuration

### Volt Node Configuration

To enable the bridge in a Volt node, add the following to your configuration file:

```toml
[bridge]
enabled = true
ethereum_rpc = "https://mainnet.infura.io/v3/YOUR_INFURA_KEY"
contract_address = "0x1234567890abcdef1234567890abcdef12345678"
confirmations = 12
```

### Ethereum Smart Contract Deployment

To deploy the Ethereum smart contract:

1. Compile the contract:

```bash
cd src/bridge/contracts
solc --bin --abi ETHBridge.sol -o build
```

2. Deploy the contract using your preferred method (e.g., Hardhat, Truffle, Remix)

3. Configure the contract with the Volt network parameters:

```solidity
// Using web3.js or ethers.js
bridge.initialize(voltRootHash, voltTokenRegistry);
```

## Usage

### CLI Commands

The Volt CLI provides several commands for interacting with the bridge:

#### Bridge to Ethereum

```bash
./target/release/cli bridge-to-eth --token-id <TOKEN_ID> --amount <AMOUNT> --eth-address <ETH_ADDRESS>
```

Parameters:

-  `--token-id`: The ID of the token to bridge
-  `--amount`: The amount to bridge
-  `--eth-address`: The Ethereum address to receive the tokens

#### Bridge from Ethereum

```bash
./target/release/cli bridge-from-eth --tx-hash <TX_HASH>
```

Parameters:

-  `--tx-hash`: The Ethereum transaction hash of the bridge operation

#### List Bridged Tokens

```bash
./target/release/cli list-bridged-tokens
```

This command lists all tokens that have been bridged between Volt and Ethereum.

### RPC Methods

The Volt node provides several RPC methods for interacting with the bridge:

#### Bridge to Ethereum

```json
{
   "jsonrpc": "2.0",
   "method": "bridge_toEthereum",
   "params": [
      {
         "token_id": 1,
         "amount": "100",
         "eth_address": "0x1234567890abcdef1234567890abcdef12345678"
      }
   ],
   "id": 1
}
```

#### Bridge from Ethereum

```json
{
   "jsonrpc": "2.0",
   "method": "bridge_fromEthereum",
   "params": [
      {
         "tx_hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
      }
   ],
   "id": 1
}
```

#### Get Bridge Status

```json
{
   "jsonrpc": "2.0",
   "method": "bridge_getStatus",
   "params": [
      {
         "tx_hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
      }
   ],
   "id": 1
}
```

## Monitoring

### Bridge Status

You can monitor the status of bridge operations using the following methods:

#### CLI Command

```bash
./target/release/cli bridge-status --tx-hash <TX_HASH>
```

#### RPC Method

```json
{
   "jsonrpc": "2.0",
   "method": "bridge_getStatus",
   "params": [
      {
         "tx_hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
      }
   ],
   "id": 1
}
```

### Bridge Metrics

The bridge module exposes several metrics that can be monitored:

-  **Total Bridged Volume**: The total amount of tokens bridged
-  **Active Bridge Operations**: The number of ongoing bridge operations
-  **Bridge Operation Latency**: The time it takes to complete a bridge operation
-  **Error Rate**: The rate of failed bridge operations

These metrics can be accessed through the metrics endpoint if enabled in the node configuration.

## Troubleshooting

### Common Issues

#### Bridge Operation Stuck

If a bridge operation is stuck, check the following:

1. **Ethereum Network**: Make sure the Ethereum network is functioning properly
2. **Confirmations**: Check if the transaction has enough confirmations
3. **Gas Price**: If using a relayer, check if the gas price is too low
4. **Contract State**: Check if the Ethereum contract is paused or has reached its limits

#### Invalid Proof

If a proof is rejected, check the following:

1. **Transaction Finality**: Make sure the transaction is final on the source chain
2. **Proof Format**: Check if the proof format is correct
3. **Bridge Configuration**: Verify that the bridge is properly configured
4. **Network Synchronization**: Ensure that both networks are properly synchronized

#### Token Mapping Issues

If there are issues with token mappings, check the following:

1. **Token Registry**: Verify that the token is registered in both networks
2. **Token Metadata**: Check if the token metadata matches
3. **Bridge Configuration**: Ensure that the bridge is configured to support the token

### Getting Help

If you're still having issues, you can:

1. Check the [GitHub repository](https://github.com/volt/volt) for known issues
2. Join the [Discord server](https://discord.gg/NcKvqbwg) for community support
3. Open an issue on GitHub with detailed information about your problem

## Future Developments

Planned enhancements to the Volt Ethereum Bridge include:

1. **Multi-Chain Support**: Extending the bridge to support other blockchain networks
2. **Optimistic Bridging**: Faster bridging with optimistic verification
3. **Liquidity Pools**: Adding liquidity pools to improve bridging efficiency
4. **Bridge Governance**: Decentralized governance for bridge parameters
5. **Fee Model**: Optional fee model to incentivize relayers
6. **NFT Support**: Support for bridging non-fungible tokens
7. **Layer 2 Integration**: Direct integration with Ethereum Layer 2 solutions

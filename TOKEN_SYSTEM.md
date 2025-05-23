# Volt Token System

This document provides a detailed explanation of the Volt token system, including token creation, management, and transfer mechanisms.

## Table of Contents

1. [Overview](#overview)
2. [Native Token (VOLT)](#native-token-volt)
3. [Custom Tokens](#custom-tokens)
   -  [Token Creation](#token-creation)
   -  [Token Metadata](#token-metadata)
   -  [Token Supply](#token-supply)
4. [Token Operations](#token-operations)
   -  [Minting Tokens](#minting-tokens)
   -  [Transferring Tokens](#transferring-tokens)
   -  [Burning Tokens](#burning-tokens)
5. [Token Storage](#token-storage)
6. [Ethereum Bridge](#ethereum-bridge)
7. [Security Considerations](#security-considerations)
8. [Best Practices](#best-practices)
9. [Future Developments](#future-developments)

## Overview

The Volt network supports multiple tokens on the same network infrastructure. Each token has its own unique identifier, metadata, and supply management. The token system is designed to be flexible, allowing for various use cases such as cryptocurrencies, utility tokens, security tokens, and more.

Key features of the Volt token system:

-  **Multi-Token Support**: Multiple tokens can exist on the same network
-  **Custom Metadata**: Tokens can have custom metadata (name, symbol, decimals)
-  **Supply Management**: Token issuers can control the token supply
-  **Ethereum Bridge**: Tokens can be bridged to and from Ethereum
-  **Feeless Transfers**: Token transfers are feeless

## Native Token (VOLT)

The native token of the Volt network is called VOLT. It has the following characteristics:

-  **Token ID**: 0 (reserved for the native token)
-  **Name**: Volt Token
-  **Symbol**: VOLT
-  **Decimals**: 18
-  **Issuer**: System (zero address)
-  **Supply**: Managed by the treasury

The native token is used for:

-  **Governance**: Voting on network upgrades and parameters
-  **Staking**: Securing the network through staking
-  **Payments**: Making payments within the Volt ecosystem

## Custom Tokens

### Token Creation

Custom tokens can be created by any user on the Volt network. To create a new token, you need to:

1. Generate a keypair for the token issuer
2. Define the token metadata (name, symbol, decimals)
3. Call the `issue-token` command or the `p3p_issueToken` RPC method

Example using the CLI:

```bash
./target/release/cli issue-token --metadata "My Token|MTK|18"
```

Example using the RPC API:

```json
{
   "jsonrpc": "2.0",
   "method": "p3p_issueToken",
   "params": ["0x..."], // Hex-encoded serialized message
   "id": 1
}
```

When a token is created, it is assigned a unique token ID (starting from 1, as 0 is reserved for the native token).

### Token Metadata

Token metadata includes:

-  **Name**: The name of the token (e.g., "My Token")
-  **Symbol**: The symbol of the token (e.g., "MTK")
-  **Decimals**: The number of decimal places for the token (e.g., 18)

The metadata is stored in the format "Name|Symbol|Decimals" and is associated with the token ID in the token registry.

### Token Supply

Each token has its own supply management:

-  **Total Supply**: The total amount of tokens in circulation
-  **Max Supply**: The maximum amount of tokens that can be created (optional)

The token issuer can control the supply by minting and burning tokens.

## Token Operations

### Minting Tokens

Only the token issuer can mint new tokens. To mint tokens, you need to:

1. Have the private key of the token issuer
2. Specify the token ID, recipient address, and amount
3. Call the `mint-token` command or the `p3p_mintToken` RPC method

Example using the CLI:

```bash
./target/release/cli mint-token --token-id 1 --to 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef --amount 1000
```

Example using the RPC API:

```json
{
   "jsonrpc": "2.0",
   "method": "p3p_mintToken",
   "params": [
      {
         "from": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
         "to": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
         "token_id": 1,
         "amount": "1000",
         "nonce": 0,
         "signature": "0x..."
      }
   ],
   "id": 1
}
```

When tokens are minted, the total supply of the token is increased.

### Transferring Tokens

Any user can transfer tokens they own to another address. To transfer tokens, you need to:

1. Have the private key of the sender
2. Specify the token ID, recipient address, and amount
3. Call the `send` command or the `send` RPC method

Example using the CLI:

```bash
./target/release/cli send --to 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef --amount 100 --token_id 1
```

Example using the RPC API:

```json
{
   "jsonrpc": "2.0",
   "method": "send",
   "params": ["0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef", "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890", 1, 100, 0, "0x..."],
   "id": 1
}
```

Token transfers are feeless and instant.

### Burning Tokens

Token holders can burn their tokens, reducing the total supply. To burn tokens, you need to:

1. Have the private key of the token holder
2. Specify the token ID and amount
3. Call the `burn-token` command or the corresponding RPC method

Example using the CLI:

```bash
./target/release/cli burn-token --token-id 1 --amount 100
```

When tokens are burned, the total supply of the token is decreased.

## Token Storage

Tokens are stored in the Sparse Merkle Tree (SMT) along with account balances. Each account can hold multiple tokens, with each token having its own balance and nonce.

The token registry is also stored in the SMT, mapping token IDs to token information (issuer, metadata, total supply).

## Ethereum Bridge

The Volt network includes an Ethereum bridge that allows for cross-chain token transfers. The bridge consists of:

1. **Ethereum Smart Contract**: A contract deployed on Ethereum that handles token transfers
2. **Bridge Module**: A module in the Volt node that monitors Ethereum events and processes transfers

To bridge a token from Volt to Ethereum:

1. Lock the tokens on the Volt network
2. Generate a proof of the lock
3. Submit the proof to the Ethereum smart contract
4. Receive equivalent tokens on Ethereum

To bridge a token from Ethereum to Volt:

1. Burn the tokens on Ethereum
2. Generate a proof of the burn
3. Submit the proof to the Volt network
4. Receive equivalent tokens on Volt

## Security Considerations

### Token Issuance

-  **Private Key Security**: Keep the token issuer's private key secure
-  **Metadata Verification**: Verify the token metadata before issuing
-  **Supply Management**: Plan your token supply carefully

### Token Transfers

-  **Address Verification**: Always verify recipient addresses
-  **Amount Verification**: Double-check transfer amounts
-  **Nonce Management**: Be aware of the nonce for each transaction

### Ethereum Bridge

-  **Bridge Security**: Understand the security model of the bridge
-  **Proof Verification**: Ensure proofs are properly verified
-  **Gas Costs**: Be aware of Ethereum gas costs for bridge operations

## Best Practices

### Token Creation

1. **Meaningful Metadata**: Use clear and meaningful names and symbols
2. **Appropriate Decimals**: Choose an appropriate number of decimals (18 is standard)
3. **Supply Planning**: Plan your token supply strategy in advance

### Token Management

1. **Key Security**: Secure the token issuer's private key
2. **Regular Audits**: Regularly audit token balances and supply
3. **Transparent Communication**: Communicate supply changes to token holders

### User Experience

1. **Clear Documentation**: Provide clear documentation for token holders
2. **User-Friendly Interfaces**: Develop user-friendly interfaces for token operations
3. **Support Channels**: Establish support channels for token holders

## Future Developments

Planned enhancements to the Volt token system include:

1. **Token Standards**: Standardized token interfaces similar to ERC-20 and ERC-721
2. **Token Governance**: On-chain governance for token parameters
3. **Token Metadata Extensions**: Extended metadata for tokens (e.g., logo, description)
4. **Multi-Signature Support**: Multi-signature control for token issuance and management
5. **Token Freezing**: Ability to freeze tokens in case of security incidents
6. **Token Vesting**: Built-in vesting schedules for token distributions
7. **Token Swaps**: Decentralized token swaps within the Volt network

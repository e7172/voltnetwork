# Volt CLI Wallet Guide

This guide provides detailed instructions for using the Volt CLI wallet to manage accounts, send transactions, issue tokens, and more.

## Table of Contents

1. [Installation](#installation)
2. [Configuration](#configuration)
3. [Wallet Management](#wallet-management)
   -  [Initializing a Wallet](#initializing-a-wallet)
   -  [Exporting a Seed](#exporting-a-seed)
4. [Account Management](#account-management)
   -  [Checking Balance](#checking-balance)
   -  [Viewing All Token Balances](#viewing-all-token-balances)
5. [Transactions](#transactions)
   -  [Sending Tokens](#sending-tokens)
   -  [Minting Tokens](#minting-tokens)
6. [Token Management](#token-management)
   -  [Issuing a New Token](#issuing-a-new-token)
   -  [Minting Custom Tokens](#minting-custom-tokens)
7. [Advanced Usage](#advanced-usage)
   -  [Custom Node Connection](#custom-node-connection)
   -  [Custom Wallet Path](#custom-wallet-path)
8. [Troubleshooting](#troubleshooting)

## Installation

Before using the CLI wallet, you need to build it from source:

```bash
# Clone the repository
git clone https://github.com/volt/volt.git
cd volt

# Build the project
cargo build --release

# The CLI wallet will be available at
# ./target/release/cli
```

## Configuration

The CLI wallet uses a configuration file to store settings such as the node URL. By default, the configuration file is located at `~/.volt/config.toml`.

You can specify a custom configuration file using the `--config` option:

```bash
./target/release/cli --config /path/to/config.toml <command>
```

A typical configuration file looks like this:

```toml
# Node URL to connect to
node = "http://3.90.180.149:8545"

# Default wallet file path
wallet = "~/.volt/wallet.dat"
```

## Wallet Management

### Initializing a Wallet

Before you can use the CLI wallet, you need to initialize a wallet:

```bash
./target/release/cli init-seed
```

This command will generate a new seed phrase and save it to the default wallet file location (`~/.volt/wallet.dat`). The output will show your address and the path where the seed was stored.

Example output:

```
Generating new seed...
Seed initialized: /home/user/.volt/wallet.dat
Your address: 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
```

### Exporting a Seed

If you need to backup your wallet or move it to another device, you can export the seed phrase:

```bash
./target/release/cli export-seed
```

This will display your seed phrase. **WARNING**: Keep this seed safe and private! Anyone with access to your seed phrase can access your funds.

Example output:

```
Seed: abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about
WARNING: Keep this seed safe and private!
```

## Account Management

### Checking Balance

To check your account balance for the native token:

```bash
./target/release/cli balance
```

Example output:

```
Wallet address: 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
Balance: 1000
```

### Viewing All Token Balances

The `balance` command will also attempt to show all token balances if the node supports the `getAllBalances` RPC method:

```bash
./target/release/cli balance
```

Example output with multiple tokens:

```
Wallet address: 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef

Token balances:
Token ID 0 (VOLT): 1000
Token ID 1 (TEST): 500
Token ID 2 (MTK): 750
```

## Transactions

### Sending Tokens

To send tokens to another address:

```bash
./target/release/cli send --to <ADDRESS> --amount <AMOUNT> [--token_id <TOKEN_ID>]
```

Parameters:

-  `--to`: The recipient's address (32-byte hex string)
-  `--amount`: The amount to send
-  `--token_id`: (Optional) The token ID to send (defaults to 0 for native VOLT token)

Example:

```bash
./target/release/cli send --to 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890 --amount 100
```

Example output:

```
Transaction sent: 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
```

To send a custom token:

```bash
./target/release/cli send --to 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890 --amount 50 --token_id 1
```

### Minting Tokens

The `mint` command is restricted to the treasury address and is used to mint native tokens:

```bash
./target/release/cli mint --to <ADDRESS> --amount <AMOUNT>
```

Parameters:

-  `--to`: The recipient's address (32-byte hex string)
-  `--amount`: The amount to mint

Example:

```bash
./target/release/cli mint --to 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890 --amount 1000
```

Example output:

```
Tokens minted: 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
```

## Token Management

### Issuing a New Token

To issue a new token:

```bash
./target/release/cli issue-token --metadata "<NAME>|<SYMBOL>|<DECIMALS>"
```

Parameters:

-  `--metadata`: Token metadata in the format "Name|Symbol|Decimals"

Example:

```bash
./target/release/cli issue-token --metadata "My Token|MTK|18"
```

Example output:

```
Token issued: 1
```

The output shows the token ID assigned to your new token.

### Minting Custom Tokens

After issuing a token, you can mint new tokens as the token issuer:

```bash
./target/release/cli mint-token --token-id <TOKEN_ID> --to <ADDRESS> --amount <AMOUNT>
```

Parameters:

-  `--token-id`: The token ID to mint
-  `--to`: The recipient's address (32-byte hex string)
-  `--amount`: The amount to mint

Example:

```bash
./target/release/cli mint-token --token-id 1 --to 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890 --amount 1000
```

Example output:

```
Tokens minted: 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
```

## Advanced Usage

### Custom Node Connection

You can specify a custom node URL using the `--node` option:

```bash
./target/release/cli --node http://localhost:8545 <command>
```

This is useful if you're running your own node or want to connect to a different network.

### Custom Wallet Path

You can specify a custom wallet file path using the `--wallet` option:

```bash
./target/release/cli --wallet /path/to/wallet.dat <command>
```

This is useful if you want to manage multiple wallets or store your wallet in a custom location.

## Troubleshooting

### Connection Issues

If you're having trouble connecting to the node, check the following:

1. Make sure the node URL is correct and includes the protocol (http:// or https://)
2. Check if the node is running and accessible from your network
3. Try using the `--node` option to specify the node URL explicitly

Example:

```bash
./target/release/cli --node http://3.90.180.149:8545 balance
```

### Invalid Seed

If you see an error about an invalid seed, your wallet file might be corrupted. Try initializing a new wallet:

```bash
./target/release/cli init-seed --wallet /path/to/new-wallet.dat
```

### Insufficient Balance

If you see an "insufficient balance" error when sending tokens, check your balance:

```bash
./target/release/cli balance
```

Make sure you have enough tokens to cover the amount you're trying to send.

### Transaction Errors

If a transaction fails, check the following:

1. Make sure you have enough balance
2. Check if you're using the correct token ID
3. Verify that the recipient address is valid
4. Check if the node is synchronized with the network

### Debug Mode

You can enable debug logging to get more information about what's happening:

```bash
RUST_LOG=debug ./target/release/cli <command>
```

This will show detailed logs that can help diagnose issues.

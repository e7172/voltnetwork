# Volt Node Setup Guide

This guide provides detailed instructions for setting up and running a Volt node.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Installation](#installation)
3. [Configuration](#configuration)
4. [Running a Node](#running-a-node)
   -  [Genesis Node](#genesis-node)
   -  [Regular Node](#regular-node)
5. [Node Monitoring](#node-monitoring)
6. [Troubleshooting](#troubleshooting)
7. [Advanced Configuration](#advanced-configuration)
8. [Security Considerations](#security-considerations)
9. [Upgrading](#upgrading)

## Prerequisites

Before setting up a Volt node, ensure you have the following:

-  **Operating System**: Linux, macOS, or Windows
-  **Rust**: Version 1.60 or later
-  **Cargo**: The Rust package manager
-  **Git**: For cloning the repository
-  **RocksDB**: For state persistence
-  **Minimum Hardware Requirements**:
   -  2 CPU cores
   -  4 GB RAM
   -  50 GB storage
   -  Stable internet connection

### Installing Rust and Cargo

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Installing RocksDB Dependencies

#### Ubuntu/Debian

```bash
sudo apt-get update
sudo apt-get install -y libclang-dev librocksdb-dev
```

#### macOS

```bash
brew install rocksdb
```

#### Windows

For Windows, it's recommended to use WSL (Windows Subsystem for Linux) and follow the Ubuntu/Debian instructions.

## Installation

### Cloning the Repository

```bash
git clone https://github.com/volt/volt.git
cd volt
```

### Building the Node

```bash
cargo build --release
```

This will create the node binary at `target/release/node`.

## Configuration

The Volt node can be configured using command-line arguments or a configuration file.

### Command-Line Arguments

-  `--config`: Path to the configuration file
-  `--data-dir`: Path to the data directory
-  `--bootstrap`: Bootstrap nodes to connect to
-  `--listen`: Listen address for the P2P network
-  `--rpc`: Enable JSON-RPC server
-  `--rpc-addr`: JSON-RPC server address
-  `--metrics`: Enable metrics server
-  `--metrics-addr`: Metrics server address

### Configuration File

You can create a configuration file (e.g., `config.toml`) with the following format:

```toml
# Data directory
data_dir = "/path/to/data"

# Bootstrap nodes
bootstrap = [
  "/ip4/3.90.180.149/tcp/30333/p2p/12D3KooWQLBSdMgmnicekuD8w9Lsy5CWVuNJcCBctxNeK3YsrDKF"
]

# Listen address
listen = "/ip4/0.0.0.0/tcp/30333"

# RPC configuration
rpc = true
rpc_addr = "127.0.0.1:8545"

# Metrics configuration
metrics = true
metrics_addr = "127.0.0.1:9090"
```

## Running a Node

### Genesis Node

If you're starting a new network, you'll need to run a genesis node:

```bash
mkdir -p ~/.volt/node1
RUST_LOG=debug ./target/release/node --rpc --data-dir ~/.volt/node1 --rpc-addr 0.0.0.0:8545 --listen /ip4/0.0.0.0/tcp/30333
```

This will start a node with no bootstrap nodes, effectively creating a new network.

### Regular Node

To join an existing network, you'll need to specify bootstrap nodes:

```bash
mkdir -p ~/.volt/node1
RUST_LOG=debug ./target/release/node --rpc --data-dir ~/.volt/node1 --rpc-addr 0.0.0.0:8545 --listen /ip4/0.0.0.0/tcp/30333 --bootstrap /ip4/3.90.180.149/tcp/30333/p2p/12D3KooWQLBSdMgmnicekuD8w9Lsy5CWVuNJcCBctxNeK3YsrDKF
```

This will start a node that connects to the specified bootstrap node and synchronizes the state.

### Running as a Service

#### Systemd (Linux)

Create a systemd service file at `/etc/systemd/system/volt-node.service`:

```ini
[Unit]
Description=Volt Node
After=network.target

[Service]
User=volt
ExecStart=/path/to/volt/target/release/node --config /path/to/config.toml
Restart=on-failure
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
```

Then enable and start the service:

```bash
sudo systemctl enable volt-node
sudo systemctl start volt-node
```

#### Launchd (macOS)

Create a launchd plist file at `~/Library/LaunchAgents/com.volt.node.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.volt.node</string>
    <key>ProgramArguments</key>
    <array>
        <string>/path/to/volt/target/release/node</string>
        <string>--config</string>
        <string>/path/to/config.toml</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/path/to/volt/logs/node.log</string>
    <key>StandardErrorPath</key>
    <string>/path/to/volt/logs/node.err</string>
</dict>
</plist>
```

Then load the service:

```bash
launchctl load ~/Library/LaunchAgents/com.volt.node.plist
```

## Node Monitoring

### Logs

The node logs can be viewed using the `RUST_LOG` environment variable:

```bash
RUST_LOG=debug ./target/release/node --config /path/to/config.toml
```

Log levels:

-  `error`: Only errors
-  `warn`: Warnings and errors
-  `info`: Informational messages, warnings, and errors
-  `debug`: Debug messages, informational messages, warnings, and errors
-  `trace`: All messages

### Metrics

If you enable the metrics server, you can access metrics at `http://<metrics-addr>/metrics`. These metrics can be scraped by Prometheus and visualized using Grafana.

Key metrics:

-  **Node Status**: Whether the node is running and connected to the network
-  **Peer Count**: Number of connected peers
-  **Transaction Count**: Number of transactions processed
-  **Memory Usage**: Memory used by the node
-  **CPU Usage**: CPU used by the node
-  **Disk Usage**: Disk space used by the node

### RPC API

You can use the RPC API to check the status of the node:

```bash
curl -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"get_peer_id","params":[],"id":1}' http://localhost:8545/rpc
```

This should return the peer ID of the node.

## Troubleshooting

### Common Issues

#### Node Won't Start

If the node won't start, check the following:

1. Make sure the data directory exists and is writable
2. Check if another process is using the same ports
3. Look at the logs for error messages

#### Node Can't Connect to Bootstrap Nodes

If the node can't connect to bootstrap nodes, check the following:

1. Make sure the bootstrap node addresses are correct
2. Check if the bootstrap nodes are running and accessible
3. Check if your firewall is blocking the connections

#### Node Crashes

If the node crashes, check the following:

1. Make sure you have enough memory and disk space
2. Check the logs for error messages
3. Try running with a lower log level to reduce memory usage

#### State Synchronization Issues

If the node is having trouble synchronizing the state, check the following:

1. Make sure you're connected to at least one bootstrap node
2. Check if the bootstrap nodes have the latest state
3. Try restarting the node

### Getting Help

If you're still having issues, you can:

1. Check the [GitHub repository](https://github.com/volt/volt) for known issues
2. Join the [Discord server](https://discord.gg/NcKvqbwg) for community support
3. Open an issue on GitHub with detailed information about your problem

## Advanced Configuration

### Custom Bootstrap Nodes

You can specify multiple bootstrap nodes:

```bash
./target/release/node --bootstrap /ip4/1.2.3.4/tcp/30333/p2p/PEER_ID_1 --bootstrap /ip4/5.6.7.8/tcp/30333/p2p/PEER_ID_2
```

### Custom Data Directory

You can specify a custom data directory:

```bash
./target/release/node --data-dir /path/to/data
```

### Custom RPC Address

You can specify a custom RPC address:

```bash
./target/release/node --rpc --rpc-addr 0.0.0.0:8545
```

### Custom Listen Address

You can specify a custom listen address:

```bash
./target/release/node --listen /ip4/0.0.0.0/tcp/30333
```

### Enabling Metrics

You can enable metrics collection:

```bash
./target/release/node --metrics --metrics-addr 0.0.0.0:9090
```

## Security Considerations

### Network Security

-  **Firewall**: Configure your firewall to only allow necessary connections
-  **RPC Access**: Restrict RPC access to trusted clients
-  **TLS**: Consider using TLS for RPC connections

### Data Security

-  **Backups**: Regularly backup your data directory
-  **Permissions**: Set appropriate permissions on the data directory
-  **Encryption**: Consider encrypting sensitive data

### System Security

-  **Updates**: Keep your system and the Volt node software up to date
-  **Monitoring**: Monitor your system for suspicious activity
-  **Resource Limits**: Set appropriate resource limits to prevent DoS attacks

## Upgrading

### Upgrading the Node Software

1. Stop the node
2. Backup the data directory
3. Pull the latest changes from the repository
4. Build the new version
5. Start the node

```bash
# Stop the node
sudo systemctl stop volt-node

# Backup the data directory
cp -r ~/.volt/node1 ~/.volt/node1.backup

# Pull the latest changes
cd /path/to/volt
git pull

# Build the new version
cargo build --release

# Start the node
sudo systemctl start volt-node
```

### Upgrading the Database

If the database schema has changed, you may need to migrate the data:

1. Stop the node
2. Backup the data directory
3. Run the migration script (if provided)
4. Start the node

If no migration script is provided, you may need to resynchronize the state from the network:

1. Stop the node
2. Delete the data directory
3. Start the node with bootstrap nodes

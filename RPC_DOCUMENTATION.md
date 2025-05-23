# Volt RPC API Documentation

This document provides detailed information about the JSON-RPC API endpoints available in the Volt network.

## Table of Contents

1. [Introduction](#introduction)
2. [Endpoint](#endpoint)
3. [Request Format](#request-format)
4. [Response Format](#response-format)
5. [Error Handling](#error-handling)
6. [API Methods](#api-methods)
   -  [Account and Balance Methods](#account-and-balance-methods)
   -  [Proof Methods](#proof-methods)
   -  [Token Methods](#token-methods)
   -  [Transaction Methods](#transaction-methods)
   -  [State Methods](#state-methods)
   -  [Network Methods](#network-methods)
7. [Examples](#examples)

## Introduction

The Volt node provides a JSON-RPC API for interacting with the network. This API allows you to query account balances, send transactions, issue tokens, and more.

## Endpoint

The default RPC endpoint is:

```
http://localhost:8545/rpc
```

The official public RPC endpoint is:

```
http://3.90.180.149:8545
```

## Request Format

All requests should be sent as HTTP POST requests with a JSON body. The JSON body should follow the JSON-RPC 2.0 specification:

```json
{
  "jsonrpc": "2.0",
  "method": "methodName",
  "params": [param1, param2, ...],
  "id": 1
}
```

-  `jsonrpc`: Must be "2.0"
-  `method`: The name of the method to call
-  `params`: An array of parameters for the method
-  `id`: A unique identifier for the request

## Response Format

Responses follow the JSON-RPC 2.0 specification:

```json
{
  "jsonrpc": "2.0",
  "result": resultValue,
  "error": null,
  "id": 1
}
```

If an error occurs, the response will have an error object instead of a result:

```json
{
  "jsonrpc": "2.0",
  "result": null,
  "error": {
    "code": errorCode,
    "message": "Error message",
    "data": errorData
  },
  "id": 1
}
```

## Error Handling

Common error codes:

-  `-32600`: Invalid request
-  `-32601`: Method not found
-  `-32602`: Invalid params
-  `-32603`: Internal error

## API Methods

### Account and Balance Methods

#### `getRoot`

Returns the current root hash of the state tree.

**Parameters**: None

**Returns**: The root hash as a hex string

**Example**:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "getRoot",
  "params": [],
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
  "error": null,
  "id": 1
}
```

#### `getBalance`

Returns the balance of the native token for the given address.

**Parameters**:

1. `address` (string): The address to query (32-byte hex string)

**Returns**: The balance as a number

**Example**:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "getBalance",
  "params": ["0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"],
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": 1000,
  "error": null,
  "id": 1
}
```

#### `getBalanceWithToken`

Returns the balance of the specified token for the given address.

**Parameters**:

1. `address` (string): The address to query (32-byte hex string)
2. `token_id` (number): The token ID

**Returns**: The balance as a number

**Example**:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "getBalanceWithToken",
  "params": ["0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef", 1],
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": 500,
  "error": null,
  "id": 1
}
```

#### `getAllBalances`

Returns all token balances for the given address.

**Parameters**:

1. `address` (string): The address to query (32-byte hex string)

**Returns**: An array of token balances

**Example**:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "getAllBalances",
  "params": ["0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"],
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": [
    {
      "token_id": 0,
      "balance": 1000
    },
    {
      "token_id": 1,
      "balance": 500
    }
  ],
  "error": null,
  "id": 1
}
```

#### `getNonce`

Returns the current nonce for the given address.

**Parameters**:

1. `address` (string): The address to query (32-byte hex string)

**Returns**: The nonce as a number

**Example**:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "getNonce",
  "params": ["0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"],
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": 5,
  "error": null,
  "id": 1
}
```

#### `get_nonce_with_token`

Returns the current nonce for the given address and token.

**Parameters**:

1. `address` (string): The address to query (32-byte hex string)
2. `token_id` (number): The token ID

**Returns**: The nonce as a number

**Example**:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "get_nonce_with_token",
  "params": ["0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef", 1],
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": 3,
  "error": null,
  "id": 1
}
```

### Proof Methods

#### `getProof`

Returns a proof for the given address.

**Parameters**:

1. `address` (string): The address to query (32-byte hex string)

**Returns**: A proof object

**Example**:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "getProof",
  "params": ["0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"],
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "siblings": ["0x...", "0x..."],
    "leaf_hash": "0x...",
    "path": [true, false, ...],
    "zeros_omitted": 0
  },
  "error": null,
  "id": 1
}
```

#### `get_proof_with_token`

Returns a proof for the given address and token.

**Parameters**:

1. `address` (string): The address to query (32-byte hex string)
2. `token_id` (number): The token ID

**Returns**: A proof object

**Example**:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "get_proof_with_token",
  "params": ["0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef", 1],
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "siblings": ["0x...", "0x..."],
    "leaf_hash": "0x...",
    "path": [true, false, ...],
    "zeros_omitted": 0
  },
  "error": null,
  "id": 1
}
```

### Token Methods

#### `get_tokens`

Returns a list of all tokens.

**Parameters**: None

**Returns**: An array of token objects

**Example**:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "get_tokens",
  "params": [],
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": [
    {
      "token_id": 0,
      "issuer": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
      "metadata": "VOLT|Volt Token|18",
      "total_supply": 1000000
    },
    {
      "token_id": 1,
      "issuer": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
      "metadata": "TEST|Test Token|18",
      "total_supply": 500000
    }
  ],
  "error": null,
  "id": 1
}
```

#### `p3p_issueToken`

Issues a new token.

**Parameters**:

1. `message` (string): Hex-encoded serialized message

**Returns**: The token ID as a number

**Example**:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "p3p_issueToken",
  "params": ["0x..."], // Hex-encoded serialized message
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": 2,
  "error": null,
  "id": 1
}
```

#### `p3p_mintToken`

Mints tokens for a specific token ID.

**Parameters**:

1. `message` (object or string): Token mint message (either as an object or hex-encoded string)

**Returns**: Transaction hash

**Example**:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "p3p_mintToken",
  "params": [{
    "from": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    "to": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
    "token_id": 1,
    "amount": "1000",
    "nonce": 3,
    "signature": "0x..."
  }],
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "tx_hash": "0x...",
    "status": "ok"
  },
  "error": null,
  "id": 1
}
```

#### `get_total_supply`

Returns the total supply of the native token.

**Parameters**: None

**Returns**: The total supply as a string

**Example**:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "get_total_supply",
  "params": [],
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": "1000000000",
  "error": null,
  "id": 1
}
```

#### `get_max_supply`

Returns the maximum supply of the native token.

**Parameters**: None

**Returns**: The maximum supply as a string

**Example**:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "get_max_supply",
  "params": [],
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": "340282366920938463463374607431768211455",
  "error": null,
  "id": 1
}
```

### Transaction Methods

#### `send`

Sends tokens from one address to another.

**Parameters**:

1. `from` (string): Sender address (32-byte hex string)
2. `to` (string): Recipient address (32-byte hex string)
3. `token_id` (number): Token ID
4. `amount` (number): Amount to send
5. `nonce` (number): Current nonce for the sender
6. `signature` (string): Transaction signature (64-byte hex string)

**Returns**: Transaction hash

**Example**:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "send",
  "params": [
    "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
    0,
    100,
    5,
    "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
  ],
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": "0x...",
  "error": null,
  "id": 1
}
```

#### `mint`

Mints native tokens (treasury only).

**Parameters**:

1. `from` (string): Treasury address (32-byte hex string)
2. `signature` (string): Transaction signature (64-byte hex string)
3. `to` (string): Recipient address (32-byte hex string)
4. `amount` (number): Amount to mint

**Returns**: Transaction hash

**Example**:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "mint",
  "params": [
    "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
    1000
  ],
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "tx_hash": "0x...",
    "status": "ok"
  },
  "error": null,
  "id": 1
}
```

#### `broadcastUpdate`

Broadcasts an update message to the network.

**Parameters**:

1. `message` (object): Update message

**Returns**: Transaction hash

**Example**:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "broadcastUpdate",
  "params": [{
    "from": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    "to": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
    "token_id": 0,
    "amount": 100,
    "root": "0x...",
    "post_root": "0x...",
    "proof_from": { /* proof object */ },
    "proof_to": { /* proof object */ },
    "nonce": 5,
    "signature": "0x..."
  }],
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": "0x...",
  "error": null,
  "id": 1
}
```

#### `broadcast_mint`

Broadcasts a mint message to the network.

**Parameters**:

1. `message` (string): Hex-encoded serialized mint message

**Returns**: Transaction hash

**Example**:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "broadcast_mint",
  "params": ["0x..."], // Hex-encoded serialized message
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": "0x...",
  "error": null,
  "id": 1
}
```

### State Methods

#### `get_full_state`

Returns the full state of the network.

**Parameters**: None

**Returns**: Full state object

**Example**:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "get_full_state",
  "params": [],
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "accounts": [
      {
        "addr": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        "bal": 1000,
        "nonce": 5,
        "token_id": 0
      },
      {
        "addr": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        "bal": 500,
        "nonce": 2,
        "token_id": 0
      }
    ],
    "root": "0x..."
  },
  "error": null,
  "id": 1
}
```

#### `set_full_state`

Sets the full state of the network.

**Parameters**:

1. `state` (object): Full state object

**Returns**: Boolean indicating success

**Example**:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "set_full_state",
  "params": [{
    "accounts": [
      {
        "addr": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        "bal": 1000,
        "nonce": 5,
        "token_id": 0
      },
      {
        "addr": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        "bal": 500,
        "nonce": 2,
        "token_id": 0
      }
    ],
    "root": "0x..."
  }],
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": true,
  "error": null,
  "id": 1
}
```

### Network Methods

#### `get_peer_id`

Returns the peer ID of the node.

**Parameters**: None

**Returns**: Peer ID as a string

**Example**:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "get_peer_id",
  "params": [],
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": "12D3KooWQLBSdMgmnicekuD8w9Lsy5CWVuNJcCBctxNeK3YsrDKF",
  "error": null,
  "id": 1
}
```

## Examples

### Checking Account Balance

```javascript
const axios = require('axios');

async function getBalance(address) {
   const response = await axios.post('http://3.90.180.149:8545/rpc', {
      jsonrpc: '2.0',
      method: 'getBalance',
      params: [address],
      id: 1,
   });

   return response.data.result;
}

getBalance('0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef')
   .then((balance) => console.log(`Balance: ${balance}`))
   .catch((error) => console.error('Error:', error));
```

### Sending Tokens

```javascript
const axios = require('axios');
const ed25519 = require('@noble/ed25519');

async function sendTokens(privateKey, to, amount, tokenId = 0) {
   // Derive public key and address from private key
   const publicKey = await ed25519.getPublicKey(privateKey);
   const from = publicKey.toString('hex');

   // Get current nonce
   const nonceResponse = await axios.post('http://3.90.180.149:8545/rpc', {
      jsonrpc: '2.0',
      method: tokenId === 0 ? 'getNonce' : 'get_nonce_with_token',
      params: [from, ...(tokenId !== 0 ? [tokenId] : [])],
      id: 1,
   });

   const nonce = nonceResponse.data.result;

   // Create transaction object
   const transaction = {
      from,
      to,
      token_id: tokenId,
      amount,
      nonce,
   };

   // Serialize transaction
   const message = JSON.stringify(transaction);

   // Sign transaction
   const signature = await ed25519.sign(Buffer.from(message), privateKey);

   // Send transaction
   const response = await axios.post('http://3.90.180.149:8545/rpc', {
      jsonrpc: '2.0',
      method: 'send',
      params: [from, to, tokenId, amount, nonce, signature.toString('hex')],
      id: 1,
   });

   return response.data.result;
}

// Example usage
const privateKey = '0x...'; // Your private key
const to = '0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890';
const amount = 100;

sendTokens(privateKey, to, amount)
   .then((txHash) => console.log(`Transaction hash: ${txHash}`))
   .catch((error) => console.error('Error:', error));
```

### Issuing a New Token

```javascript
const axios = require('axios');
const ed25519 = require('@noble/ed25519');
const bincode = require('bincode');

async function issueToken(privateKey, metadata) {
   // Derive public key and address from private key
   const publicKey = await ed25519.getPublicKey(privateKey);
   const issuer = publicKey.toString('hex');

   // Get current nonce
   const nonceResponse = await axios.post('http://3.90.180.149:8545/rpc', {
      jsonrpc: '2.0',
      method: 'getNonce',
      params: [issuer],
      id: 1,
   });

   const nonce = nonceResponse.data.result;

   // Create issue token message
   const message = {
      type: 'IssueToken',
      issuer,
      token_id: 0, // Will be assigned by the system
      metadata,
      nonce,
      signature: new Uint8Array(64).fill(0), // Empty signature for now
   };

   // Serialize message
   const messageBytes = bincode.serialize(message);

   // Sign message
   const signature = await ed25519.sign(messageBytes, privateKey);

   // Update message with signature
   message.signature = signature;

   // Serialize final message
   const finalMessageBytes = bincode.serialize(message);

   // Send transaction
   const response = await axios.post('http://3.90.180.149:8545/rpc', {
      jsonrpc: '2.0',
      method: 'p3p_issueToken',
      params: [Buffer.from(finalMessageBytes).toString('hex')],
      id: 1,
   });

   return response.data.result;
}

// Example usage
const privateKey = '0x...'; // Your private key
const metadata = 'MyToken|MTK|18'; // Name|Symbol|Decimals

issueToken(privateKey, metadata)
   .then((tokenId) => console.log(`Token ID: ${tokenId}`))
   .catch((error) => console.error('Error:', error));
```

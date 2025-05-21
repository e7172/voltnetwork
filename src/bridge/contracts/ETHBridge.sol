// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * @title ETHBridge
 * @dev A bridge contract for transferring ETH between Ethereum and the stateless token network.
 */
contract ETHBridge {
    // Events
    event Locked(address indexed from, bytes32 indexed to, uint256 amount);
    event Unlocked(address indexed to, bytes32 indexed from, uint256 amount);

    // State variables
    mapping(bytes32 => bool) public usedProofs;
    bytes32 public currentRoot;

    /**
     * @dev Locks ETH in the contract and emits a Locked event.
     * @param to The address on the stateless token network to send the tokens to.
     */
    function lock(bytes32 to) external payable {
        require(msg.value > 0, "Amount must be greater than 0");
        
        emit Locked(msg.sender, to, msg.value);
    }

    /**
     * @dev Unlocks ETH from the contract and sends it to the specified address.
     * @param to The address on Ethereum to send the tokens to.
     * @param amount The amount of tokens to unlock.
     * @param proof The Merkle proof for the stateless token network.
     * @param proofPath The path in the Merkle tree.
     * @param from The address on the stateless token network that is sending the tokens.
     */
    function unlock(
        address payable to,
        uint256 amount,
        bytes32[] calldata proof,
        bool[] calldata proofPath,
        bytes32 from
    ) external {
        require(to != address(0), "Invalid recipient address");
        require(amount > 0, "Amount must be greater than 0");
        require(address(this).balance >= amount, "Insufficient balance");
        
        // Create a unique identifier for this proof
        bytes32 proofId = keccak256(abi.encodePacked(from, to, amount, proof, proofPath));
        
        // Ensure this proof hasn't been used before
        require(!usedProofs[proofId], "Proof already used");
        
        // Mark the proof as used
        usedProofs[proofId] = true;
        
        // Verify the Merkle proof
        require(verifyProof(proof, proofPath, from, currentRoot), "Invalid proof");
        
        // Transfer the ETH
        to.transfer(amount);
        
        emit Unlocked(to, from, amount);
    }

    /**
     * @dev Updates the current root of the stateless token network.
     * @param newRoot The new root hash.
     */
    function updateRoot(bytes32 newRoot) external {
        // In a production environment, this would be restricted to authorized parties
        // or use a more sophisticated consensus mechanism
        currentRoot = newRoot;
    }

    /**
     * @dev Verifies a Merkle proof.
     * @param proof The Merkle proof.
     * @param path The path in the Merkle tree.
     * @param leaf The leaf node.
     * @param root The root hash.
     * @return True if the proof is valid, false otherwise.
     */
    function verifyProof(
        bytes32[] calldata proof,
        bool[] calldata path,
        bytes32 leaf,
        bytes32 root
    ) internal pure returns (bool) {
        require(proof.length == path.length, "Proof and path length mismatch");
        
        bytes32 computedHash = leaf;
        
        for (uint256 i = 0; i < proof.length; i++) {
            bytes32 proofElement = proof[i];
            
            if (path[i]) {
                // Hash(current computed hash + current element of the proof)
                computedHash = keccak256(abi.encodePacked(proofElement, computedHash));
            } else {
                // Hash(current element of the proof + current computed hash)
                computedHash = keccak256(abi.encodePacked(computedHash, proofElement));
            }
        }
        
        return computedHash == root;
    }

    /**
     * @dev Returns the balance of the contract.
     * @return The balance of the contract.
     */
    function getBalance() external view returns (uint256) {
        return address(this).balance;
    }

    /**
     * @dev Checks if a proof has been used.
     * @param proofId The ID of the proof.
     * @return True if the proof has been used, false otherwise.
     */
    function isProofUsed(bytes32 proofId) external view returns (bool) {
        return usedProofs[proofId];
    }
}

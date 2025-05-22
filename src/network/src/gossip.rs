//! Gossip implementation for broadcasting state updates.

use crate::errors::NetworkError;
use crate::types::UpdateMsg;
use libp2p::gossipsub::{
    Behaviour as Gossipsub, ConfigBuilder as GossipsubConfigBuilder, Event as GossipsubEvent, IdentTopic,
    MessageAuthenticity, MessageId, ValidationMode,
};
use libp2p::identity::Keypair;
use libp2p::gossipsub;
use libp2p::PeerId;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// The topic for state updates.
pub const STATE_UPDATES_TOPIC: &str = "state_updates";

/// Creates a new Gossipsub instance.
pub fn new_gossipsub(
    local_key: &Keypair,
    peer_id: &PeerId,
) -> Result<Gossipsub, NetworkError> {
    // Create a Gossipsub configuration
    let gossipsub_config = GossipsubConfigBuilder::default()
        .heartbeat_interval(std::time::Duration::from_secs(10))
        .validation_mode(ValidationMode::Strict)
        .message_id_fn(message_id_fn)
        .mesh_n_low(0)        // Allow publishing with 0 peers (for testing)
        .mesh_n(1)            // Target just 1 peer in mesh (minimum)
        .mesh_outbound_min(0) // Don't require any outbound peers
        .mesh_n_high(2)       // Upper bound for mesh peers
        .gossip_lazy(1)       // Gossip to at least 1 peer
        .history_length(5)    // Keep last 5 messages
        .history_gossip(1)    // Gossip to 1 peer
        .build()
        .map_err(|e| NetworkError::GossipError(e.to_string()))?;
    
    log::info!("Created Gossipsub configuration: {:?}", gossipsub_config);

    // Create a Gossipsub instance
    let mut gossipsub = Gossipsub::new(
        MessageAuthenticity::Signed(local_key.clone()),
        gossipsub_config,
    )
    .map_err(|e| NetworkError::GossipError(e.to_string()))?;

    // Subscribe to the state updates topic
    let topic = IdentTopic::new(STATE_UPDATES_TOPIC);
    match gossipsub.subscribe(&topic) {
        Ok(_) => {
            log::info!("Successfully subscribed to topic: {}", topic);
        },
        Err(e) => {
            log::error!("Failed to subscribe to topic {}: {}", topic, e);
            return Err(NetworkError::GossipError(e.to_string()));
        }
    }

    Ok(gossipsub)
}

/// Broadcasts an update message to the network.
pub async fn broadcast_update(
    gossipsub: &mut Gossipsub,
    update: &UpdateMsg,
) -> Result<(), NetworkError> {
    log::info!("Broadcasting update message: {:?}", update);
    
    // Serialize the update message
    let data = bincode::serialize(update)
        .map_err(|e| NetworkError::SerializationError(e.to_string()))?;

    // Create a topic
    let topic = IdentTopic::new(STATE_UPDATES_TOPIC);
    log::info!("Using topic: {}", topic);

    // Log mesh information
    log::info!("Attempting to publish to topic: {}", topic);

    // Publish the message
    match gossipsub.publish(topic, data) {
        Ok(_) => {
            log::info!("Successfully published update message to gossip network");
            Ok(())
        },
        Err(e) => {
            log::error!("Failed to publish update message: {}", e);
            Err(NetworkError::GossipError(e.to_string()))
        }
    }
}

/// Handles a Gossipsub event.
pub fn handle_gossipsub_event(
    event: GossipsubEvent,
) -> Result<Option<UpdateMsg>, NetworkError> {
    match event {
        GossipsubEvent::Message {
            propagation_source,
            message_id,
            message,
        } => {
            log::info!("Received gossip message from {}, id: {}, topic: {}",
                      propagation_source, message_id, message.topic);
            
            // Check if the message is on the state updates topic
            if message.topic.as_str() == STATE_UPDATES_TOPIC {
                log::info!("Message is on state updates topic");
                
                // Deserialize the message
                match bincode::deserialize::<UpdateMsg>(&message.data) {
                    Ok(update) => {
                        log::info!("Successfully deserialized update message: {:?}", update);
                        Ok(Some(update))
                    },
                    Err(e) => {
                        log::error!("Failed to deserialize update message: {}", e);
                        Err(NetworkError::SerializationError(e.to_string()))
                    }
                }
            } else {
                log::debug!("Message is not on state updates topic");
                Ok(None)
            }
        }
        GossipsubEvent::Subscribed { peer_id, topic } => {
            log::info!("Peer {} subscribed to topic {}", peer_id, topic);
            Ok(None)
        }
        GossipsubEvent::Unsubscribed { peer_id, topic } => {
            log::info!("Peer {} unsubscribed from topic {}", peer_id, topic);
            Ok(None)
        }
        GossipsubEvent::GossipsubNotSupported { peer_id } => {
            log::warn!("Peer {} does not support gossipsub", peer_id);
            Ok(None)
        }
        #[allow(unreachable_patterns)]
        _ => {
            log::debug!("Received other gossipsub event");
            Ok(None)
        },
    }
}

/// Computes a message ID for a Gossipsub message.
pub fn message_id_fn(message: &gossipsub::Message) -> MessageId {
    let mut hasher = DefaultHasher::new();
    message.data.hash(&mut hasher);
    MessageId::from(hasher.finish().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::{proofs::Proof, types::Address};
    use libp2p::identity::Keypair;
    use rand::Rng;

    #[test]
    fn test_gossipsub_creation() {
        let local_key = Keypair::generate_ed25519();
        let peer_id = PeerId::from(local_key.public());

        let gossipsub = new_gossipsub(&local_key, &peer_id);
        assert!(gossipsub.is_ok());
    }

    #[test]
    fn test_message_id_fn() {
        let mut rng = rand::thread_rng();
        let mut data1 = [0u8; 32];
        let mut data2 = [0u8; 32];
        rng.fill(&mut data1);
        rng.fill(&mut data2);

        let topic = gossipsub::IdentTopic::new(STATE_UPDATES_TOPIC);

        let message1 = gossipsub::Message {
            source: None,
            data: data1.to_vec(),
            sequence_number: None,
            topic: topic.hash(),
        };

        let message2 = gossipsub::Message {
            source: None,
            data: data2.to_vec(),
            sequence_number: None,
            topic: topic.hash(),
        };

        let id1 = message_id_fn(&message1);
        let id2 = message_id_fn(&message2);

        // Different messages should have different IDs
        assert_ne!(id1, id2);

        // Same message should have same ID
        assert_eq!(id1, message_id_fn(&message1));
    }
}

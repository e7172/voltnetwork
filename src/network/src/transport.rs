//! Transport implementation for the network layer.

use crate::dht::DHTManager;
use crate::errors::NetworkError;
use crate::gossip::{message_id_fn, new_gossipsub, STATE_UPDATES_TOPIC};
use crate::types::{ProofRequest, ProofResponse, UpdateMsg};
use ::futures::StreamExt;
use libp2p::{
    core::{upgrade, transport::Transport},
    identify,
    identity::Keypair,
    kad::{store::MemoryStore, Kademlia, KademliaConfig, KademliaEvent, record::Key as KadKey},
    noise,
    ping,
    gossipsub::{Behaviour as Gossipsub, Event as GossipsubEvent, MessageId},
    Multiaddr, PeerId, Swarm,
    swarm::{SwarmBuilder, SwarmEvent},
    tcp, yamux,
};
use libp2p::swarm::derive_prelude::*;
use std::collections::HashSet;
use std::time::Duration;

/// The network behavior for the node.
use libp2p::swarm::NetworkBehaviour;
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "NetworkBehaviourEvent")]
pub struct NodeBehaviour {
    /// Kademlia DHT for storing and retrieving proofs
    pub kademlia: Kademlia<MemoryStore>,
    /// Gossipsub for broadcasting state updates
    pub gossipsub: Gossipsub,
    /// Ping for keeping connections alive
    pub ping: ping::Behaviour,
    /// Identify for discovering peer information
    pub identify: identify::Behaviour,
}

/// Events emitted by the network behavior.
#[derive(Debug)]
pub enum NetworkBehaviourEvent {
    /// Kademlia event
    Kademlia(KademliaEvent),
    /// Gossipsub event
    Gossipsub(GossipsubEvent),
    /// Ping event
    Ping(ping::Event),
    /// Identify event
    Identify(identify::Event),
}

impl From<KademliaEvent> for NetworkBehaviourEvent {
    fn from(event: KademliaEvent) -> Self {
        NetworkBehaviourEvent::Kademlia(event)
    }
}

impl From<GossipsubEvent> for NetworkBehaviourEvent {
    fn from(event: GossipsubEvent) -> Self {
        NetworkBehaviourEvent::Gossipsub(event)
    }
}

impl From<ping::Event> for NetworkBehaviourEvent {
    fn from(event: ping::Event) -> Self {
        NetworkBehaviourEvent::Ping(event)
    }
}

impl From<identify::Event> for NetworkBehaviourEvent {
    fn from(event: identify::Event) -> Self {
        NetworkBehaviourEvent::Identify(event)
    }
}

/// Events emitted by the network.
#[derive(Debug)]
pub enum NetworkEvent {
    /// A state update was received
    UpdateReceived(UpdateMsg),
    /// A proof request was received
    ProofRequestReceived(ProofRequest, PeerId),
    /// A proof response was received
    ProofResponseReceived(ProofResponse),
    /// A new peer was discovered
    PeerDiscovered(PeerId),
    /// A peer was disconnected
    PeerDisconnected(PeerId),
    /// A peer was identified
    PeerIdentified(PeerId, Multiaddr),
}

/// Initializes the network swarm.
pub async fn init_swarm(
    bootstrap_nodes: Vec<Multiaddr>,
) -> Result<(Swarm<NodeBehaviour>, DHTManager), NetworkError> {
    // Generate a random identity
    let local_key = Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    // Create a custom transport with TCP, Noise, and Yamux
    let tcp_transport = libp2p::tcp::tokio::Transport::new(libp2p::tcp::Config::default().nodelay(true));

    let transport = tcp_transport
        .upgrade(libp2p::core::upgrade::Version::V1)
        .authenticate(libp2p::noise::Config::new(&local_key).expect("Failed to create noise config"))
        .multiplex(libp2p::yamux::Config::default())
        .boxed();

    // Create a Kademlia instance
    let mut kademlia_config = KademliaConfig::default();
    kademlia_config.set_query_timeout(Duration::from_secs(30));

    let store = MemoryStore::new(local_peer_id);
    let mut kademlia = Kademlia::with_config(local_peer_id, store, kademlia_config);

    // Add bootstrap nodes
    use libp2p::multiaddr::Protocol;
    for addr in bootstrap_nodes.iter() {
        // pull out any Protocol::P2p(peer_id) entries
        let maybe_peer = addr
            .iter()
            .filter_map(|protocol| {
                if let Protocol::P2p(peer_id) = protocol {
                    // protocol is owned, peer_id: PeerId
                    Some(peer_id.clone())
                } else {
                    None
                }
            })
            .next();

        if let Some(peer_id) = maybe_peer {
            kademlia.add_address(&peer_id, addr.clone());
            log::info!("Bootstrapped Kademlia to {} at {}", peer_id, addr);
        } else {
            log::warn!("Bootstrap address {} missing /p2p/<PeerId>", addr);
        }
    }    // Create a Gossipsub instance
    let gossipsub = new_gossipsub(&local_key, &local_peer_id)?;

    // Create a Ping instance
    let ping = ping::Behaviour::new(ping::Config::new());

    // Create an Identify instance
    let identify = identify::Behaviour::new(identify::Config::new(
        "/stateless-token/1.0.0".to_string(),
        local_key.public(),
    ));

    // Create the network behavior
    let behaviour = NodeBehaviour {
        kademlia,
        gossipsub,
        ping,
        identify,
    };

    // Create the swarm
    let swarm = SwarmBuilder::with_executor(
        transport,
        behaviour,
        local_peer_id,
        Box::new(|fut| {
            tokio::spawn(fut);
        }),
    )
    .build();

    // Create the DHT manager
    let dht_manager = DHTManager::new();

    Ok((swarm, dht_manager))
}

/// Handles a network event.
pub async fn handle_network_event(
    event: SwarmEvent<NetworkBehaviourEvent, impl std::fmt::Debug>,
    dht_manager: &DHTManager,
    known_peers: &mut HashSet<PeerId>,
    swarm: &mut Swarm<NodeBehaviour>,
) -> Result<Option<NetworkEvent>, NetworkError> {
    match event {
        SwarmEvent::Behaviour(NetworkBehaviourEvent::Gossipsub(gossipsub_event)) => {
            if let Some(update) = crate::gossip::handle_gossipsub_event(gossipsub_event)? {
                return Ok(Some(NetworkEvent::UpdateReceived(update)));
            }
        }
        SwarmEvent::Behaviour(NetworkBehaviourEvent::Kademlia(kademlia_event)) => {
            dht_manager.handle_event(kademlia_event, &mut swarm.behaviour_mut().kademlia);
        }
        SwarmEvent::Behaviour(NetworkBehaviourEvent::Identify(identify::Event::Received {
            peer_id,
            info,
            ..
        })) => {
            // Add the peer's addresses to Kademlia
            if let Some(addr) = info.listen_addrs.into_iter().next() {
                swarm
                    .behaviour_mut()
                    .kademlia
                    .add_address(&peer_id, addr.clone());

                return Ok(Some(NetworkEvent::PeerIdentified(peer_id, addr)));
            }
        }
        SwarmEvent::ConnectionEstablished {
            peer_id, endpoint, ..
        } => {
            if known_peers.insert(peer_id) {
                return Ok(Some(NetworkEvent::PeerDiscovered(peer_id)));
            }
        }
        SwarmEvent::ConnectionClosed {
            peer_id, endpoint, ..
        } => {
            if known_peers.remove(&peer_id) {
                return Ok(Some(NetworkEvent::PeerDisconnected(peer_id)));
            }
        }
        _ => {}
    }

    Ok(None)
}

/// Handles a network event synchronously.
pub fn handle_network_event_sync(
    event: SwarmEvent<NetworkBehaviourEvent, impl std::fmt::Debug>,
    dht_manager: &DHTManager,
    known_peers: &mut HashSet<PeerId>,
    swarm: &mut Swarm<NodeBehaviour>,
) -> Result<Option<NetworkEvent>, NetworkError> {
    match event {
        SwarmEvent::Behaviour(NetworkBehaviourEvent::Gossipsub(gossipsub_event)) => {
            if let Some(update) = crate::gossip::handle_gossipsub_event(gossipsub_event)? {
                return Ok(Some(NetworkEvent::UpdateReceived(update)));
            }
        }
        SwarmEvent::Behaviour(NetworkBehaviourEvent::Kademlia(kademlia_event)) => {
            dht_manager.handle_event_sync(kademlia_event, &mut swarm.behaviour_mut().kademlia);
        }
        SwarmEvent::Behaviour(NetworkBehaviourEvent::Identify(identify::Event::Received {
            peer_id,
            info,
            ..
        })) => {
            // Add the peer's addresses to Kademlia
            if let Some(addr) = info.listen_addrs.into_iter().next() {
                swarm
                    .behaviour_mut()
                    .kademlia
                    .add_address(&peer_id, addr.clone());

                return Ok(Some(NetworkEvent::PeerIdentified(peer_id, addr)));
            }
        }
        SwarmEvent::ConnectionEstablished {
            peer_id, ..
        } => {
            if known_peers.insert(peer_id) {
                return Ok(Some(NetworkEvent::PeerDiscovered(peer_id)));
            }
        }
        SwarmEvent::ConnectionClosed {
            peer_id, ..
        } => {
            if known_peers.remove(&peer_id) {
                return Ok(Some(NetworkEvent::PeerDisconnected(peer_id)));
            }
        }
        _ => {}
    }

    Ok(None)
}


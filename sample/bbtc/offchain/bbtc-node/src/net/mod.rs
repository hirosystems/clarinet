use async_std::{io, task};
use futures::{future, prelude::*};
use libp2p::futures::StreamExt;
use libp2p::{
    floodsub::{self, Floodsub, FloodsubEvent},
    identity::Keypair,
    mdns::{Mdns, MdnsEvent},
    swarm::NetworkBehaviourEventProcess,
    Multiaddr, NetworkBehaviour, PeerId, Swarm,
};
use std::{
    error::Error,
    task::{Context, Poll},
};

#[derive(NetworkBehaviour)]
struct NodeBehaviour {
    floodsub: Floodsub,
    mdns: Mdns,

    #[behaviour(ignore)]
    #[allow(dead_code)]
    ignored_member: bool,
}

impl NetworkBehaviourEventProcess<FloodsubEvent> for NodeBehaviour {
    fn inject_event(&mut self, message: FloodsubEvent) {
        if let FloodsubEvent::Message(message) = message {
            println!(
                "Received: '{:?}' from {:?}",
                String::from_utf8_lossy(&message.data),
                message.source
            );
        }
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for NodeBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(list) => {
                for (peer, _) in list {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(list) => {
                for (peer, _) in list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}

pub fn start_networking(
    local_key: &Keypair,
    bootstrap_node: Option<Multiaddr>,
) -> Result<(), Box<dyn Error>> {
    let local_peer_id = PeerId::from_public_key(local_key.public());

    println!("Starting p2p stack with peer_id {}", local_peer_id);

    let transport = libp2p::build_development_transport(local_key.clone())?;
    let floodsub_topic = floodsub::Topic::new("chat");

    let mut swarm = {
        let mdns = task::block_on(Mdns::new())?;
        let mut behaviour = NodeBehaviour {
            floodsub: Floodsub::new(local_peer_id.clone()),
            mdns,
            ignored_member: false,
        };

        behaviour.floodsub.subscribe(floodsub_topic.clone());
        Swarm::new(transport, behaviour, local_peer_id)
    };

    if let Some(bootstrap_node) = bootstrap_node {
        Swarm::dial_addr(&mut swarm, bootstrap_node.clone())?;
        println!("Using {} as a bootstrap node", bootstrap_node)
    }
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    // Listen on all interfaces and whatever port the OS assigns
    Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/0".parse()?)?;

    let mut listening = false;
    task::block_on(future::poll_fn(move |cx: &mut Context<'_>| {
        loop {
            match stdin.try_poll_next_unpin(cx)? {
                Poll::Ready(Some(line)) => swarm
                    .floodsub
                    .publish(floodsub_topic.clone(), line.as_bytes()),
                Poll::Ready(None) => panic!("Stdin closed"),
                Poll::Pending => break,
            }
        }
        loop {
            match swarm.poll_next_unpin(cx) {
                Poll::Ready(Some(event)) => println!("{:?}", event),
                Poll::Ready(None) => return Poll::Ready(Ok(())),
                Poll::Pending => {
                    if !listening {
                        for addr in Swarm::listeners(&swarm) {
                            println!("Listening on {:?}", addr);
                            listening = true;
                        }
                    }
                    break;
                }
            }
        }
        Poll::Pending
    }))
}

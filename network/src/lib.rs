use network::Network;
use tokio::{runtime::Runtime, sync::mpsc::{UnboundedReceiver, UnboundedSender}};
use types::{config::network_config::NetworkConfig, network::network_message::RpcRequest};

pub mod single;
pub mod network;

pub fn start(config: NetworkConfig, network_rx: UnboundedReceiver<RpcRequest>,
    raft_tx: UnboundedSender<RpcRequest>) -> Runtime{
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .thread_name("network")
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime!");
    let network = Network::new(config, network_rx, raft_tx);
    runtime.spawn(network.start());
    runtime
}
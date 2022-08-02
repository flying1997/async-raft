use network::Network;
use rl_logger::debug;
use tokio::{runtime::Runtime, sync::{mpsc::{UnboundedReceiver, UnboundedSender}, oneshot}};
use types::{config::network_config::NetworkConfig, network::network_message::{NetworkProtocol, RpcContent}};
use futures::executor::block_on;

pub mod single;
pub mod network;

pub fn start(config: NetworkConfig, network_rx: UnboundedReceiver<(NetworkProtocol, oneshot::Sender<NetworkProtocol>)>,
    raft_tx: UnboundedSender<NetworkProtocol>) -> Runtime{
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .thread_name("network")
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime!");
    let network = Network::new(config, network_rx, raft_tx);
    let (tx, rx) = oneshot::channel();
    runtime.spawn(async {
        network.start(Some(tx)).await
    
    });
    block_on(rx);
    debug!("Network started!!");
    runtime
}
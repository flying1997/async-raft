#![cfg_attr(feature = "docinclude", feature(external_doc))]
#![cfg_attr(feature = "docinclude", doc(include = "../README.md"))]

pub mod config;
mod core;
pub mod metrics;
pub mod raft;
mod replication;
mod network_task;
pub mod raft_sender;
use std::sync::Arc;

use memstore::{MemStore};
use raft_sender::RaftSender;
use tokio::{runtime::{self, Runtime}, sync::{mpsc::{self, UnboundedReceiver}, oneshot, watch}};
use types::{app_data::NodeId, client::{ClientRequest, ClientResponse}, config::NodeConfig, network::network_message::{RpcContent, RpcResponceState, RpcType}, raft::RaftMsg};

pub use crate::{
    config::{Config, ConfigBuilder, SnapshotPolicy},
    core::State,
    core::RaftCore,
    metrics::RaftMetrics,
    raft::Raft,
};

pub fn start_consensus( id: NodeId, config: &NodeConfig, network_tasks: UnboundedReceiver<RpcContent>,
    raft_interface: Arc<Raft<ClientRequest, ClientResponse, RaftSender, MemStore>>,
    network: Arc<RaftSender>, storage: Arc<MemStore>, rx_api: mpsc::UnboundedReceiver<RaftMsg<ClientRequest, ClientResponse>>,
    tx_metrics: watch::Sender<RaftMetrics>, rx_shutdown: oneshot::Receiver<()>) -> Runtime{
    let runtime = runtime::Builder::new_multi_thread()
        .thread_name("consensus")
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime!");
    let config = Arc::new(Config::build(String::from("node")).validate().unwrap());
    let network_tx = network.get_network();
    runtime.spawn(async move{
        let mut network_tasks = network_tasks;
        // let raft_interface = raft_interface;

        loop{
            let rev = network_tasks.recv().await.unwrap();
            let req: RpcType = bcs::from_bytes(&rev.get_data()).unwrap();
            let from = rev.get_from();
            let to = rev.get_to();
            let serial = rev.get_serial();
            let (tx, rx) = oneshot::channel();
            match req {
                RpcType::AppendEntries(req) => {
                    let data = bcs::to_bytes(&raft_interface.append_entries(req).await.unwrap()).unwrap();
                    let res = RpcContent::new_rpc_responce(to, from, serial, data, RpcResponceState::Success);
                    network_tx.send((res, tx));
                }
                RpcType::Vote(req) => {
                    let data = bcs::to_bytes(&raft_interface.vote(req).await.unwrap()).unwrap();
                    let res = RpcContent::new_rpc_responce(to, from, serial, data, RpcResponceState::Success);
                    network_tx.send((res, tx));
                }
                RpcType::Snapshot(req) => { 
                    let data = bcs::to_bytes(&raft_interface.install_snapshot(req).await.unwrap()).unwrap();
                    let res = RpcContent::new_rpc_responce(to, from, serial, data, RpcResponceState::Success);
                    network_tx.send((res, tx));
                }
                RpcType::TransferTxn(req) => {

                }
            }
        }
    });


    runtime.spawn(async move{
        RaftCore::spawn(id, config, network, storage, rx_api, tx_metrics, rx_shutdown);
    
    }
    );
    
    runtime    
}


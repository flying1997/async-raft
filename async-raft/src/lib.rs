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
use types::{app_data::NodeId, client::{ClientRequest, ClientResponse}, config::NodeConfig, network::network_message::{self, NetworkProtocol, RpcContent, RpcResponceState, RpcType}, raft::RaftMsg};

pub use crate::{
    config::{Config, ConfigBuilder, SnapshotPolicy},
    core::State,
    core::RaftCore,
    metrics::RaftMetrics,
    raft::Raft,
};

pub fn start_consensus( id: NodeId, config: &NodeConfig, network_tasks: UnboundedReceiver<NetworkProtocol>,
    raft_interface: Arc<Raft<ClientRequest, ClientResponse, RaftSender, MemStore>>,
    network: Arc<RaftSender>, storage: Arc<MemStore>, rx_api: mpsc::UnboundedReceiver<RaftMsg<ClientRequest, ClientResponse>>,
    tx_metrics: watch::Sender<RaftMetrics>, rx_shutdown: oneshot::Receiver<()>) -> Runtime{
    let runtime = runtime::Builder::new_multi_thread()
        .thread_name("raft consensus")
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime!");
    let config = Arc::new(Config::build(String::from("node")).validate().unwrap());
    let network_tx = network.get_network();
    runtime.spawn(async move{
        let mut network_tasks = network_tasks;
        // let raft_interface = raft_interface;
        //
        loop{
            let rev = network_tasks.recv().await.unwrap();
            // let req: NetworkProtocol = bcs::from_bytes(&rev).unwrap();
            match rev {
                NetworkProtocol::RpcRequest(req) =>{
                    let from = req.get_from();
                    let to = req.get_to();
                    let serial = req.get_serial();
                    let req: RpcType = bcs::from_bytes(&req.get_data()).unwrap();
                    let (tx, _) = oneshot::channel();
                    match req {
                        RpcType::Raft(r) =>{
                            match r{
                                network_message::Raft::AppendEntries(req) => {
                                    let data = bcs::to_bytes(&raft_interface.append_entries(req).await.unwrap()).unwrap();
                                    let res = NetworkProtocol::RpcResponce(RpcContent::new(to, from, Some(serial), data), RpcResponceState::Success);
                                    network_tx.send((res, tx));
                                }
                                network_message::Raft::Vote(req) => {
                                    let data = bcs::to_bytes(&raft_interface.vote(req).await.unwrap()).unwrap();
                                    let res = NetworkProtocol::RpcResponce(RpcContent::new(to, from, Some(serial), data), RpcResponceState::Success);
                                    network_tx.send((res, tx));
                                }
                                network_message::Raft::Snapshot(req) => { 
                                    let data = bcs::to_bytes(&raft_interface.install_snapshot(req).await.unwrap()).unwrap();
                                    let res = NetworkProtocol::RpcResponce(RpcContent::new(to, from, Some(serial), data), RpcResponceState::Success);
                                    network_tx.send((res, tx));
                                }
                                network_message::Raft::TransferTxn(req) => {

                                }
                            }
                        }
                    RpcType::Pbft => todo!(),
                
                    }
                }
                _ => todo!(),
            }

          
            
        }
    });


    runtime.spawn(async move{
        RaftCore::spawn(id, config, network, storage, rx_api, tx_metrics, rx_shutdown);
    
    }
    );
    
    runtime    
}


pub mod config;
pub mod error;
pub mod hash;
pub mod message_log;
pub mod node;
pub mod state;
pub mod storage;


pub mod pbft_sender;

pub mod timing;

use std::sync::Arc;

use node::PbftNode;
use pbft_sender::PbftSender;
use state::PbftState;
use tokio::{runtime::{self, Runtime}, sync::mpsc::UnboundedReceiver, sync::mpsc};
use tokio::sync::oneshot;
use types::network::network_message::{RpcContent, RpcType};
use types::network::network_message;
use crate::config::PbftConfig;


pub fn start_consensus(config: PbftConfig, network_tasks: UnboundedReceiver<RpcContent>,) -> Runtime{
    let runtime = runtime::Builder::new_multi_thread()
        .thread_name("pbft consensus")
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime!");
    
    let (consensus_tx, consensus_rx) = mpsc::unbounded_channel();

    //recevice msg from network and handle it
    runtime.spawn(async move{
        let mut network_tasks = network_tasks;
        let rev = network_tasks.recv().await.unwrap();
        let req: RpcType = bcs::from_bytes(&rev.get_data()).unwrap();
        let from = rev.get_from();
        let to = rev.get_to();
        let serial = rev.get_serial();
        // let (tx, rx) = oneshot::channel();
        match req {
            RpcType::Pbft =>{

            }
            _ => todo!(),
        }
    });



    let peer_id = Vec::new();
    let state = PbftState::new(peer_id.clone(), 0, &config);
    //send msg to network
    let pbft_sender = PbftSender::new();

    // let mut node = PbftNode::new(
    //     &config,
    //     Arc::new(pbft_sender),
    //     &state,
    // );
    runtime.spawn(async move{
        PbftNode::start(peer_id, &config, Arc::new(pbft_sender), consensus_tx.clone(), consensus_rx);
    });
    

    runtime
    
}

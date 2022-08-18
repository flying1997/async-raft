pub mod config;
pub mod error;
pub mod hash;
pub mod message_log;
pub mod node;
pub mod state;
pub mod storage;


pub mod pbft_sender;

pub mod timing;

use std::{sync::Arc, thread, time};

use node::PbftNode;
use pbft_sender::PbftSender;
use rl_logger::info;
use tokio::{runtime::{self, Runtime}, sync::mpsc::{UnboundedReceiver, UnboundedSender}, sync::mpsc};
use tokio::sync::oneshot;
use types::{network::network_message::{NetworkProtocol}, pbft::consensus::PbftMessage};
use crate::config::PbftConfig;


pub fn start_consensus(config: PbftConfig, mut network_tasks: UnboundedReceiver<NetworkProtocol>,
    network_sender: UnboundedSender<(NetworkProtocol, oneshot::Sender<NetworkProtocol>)>
    ) -> Runtime{
    let runtime = runtime::Builder::new_multi_thread()
        .thread_name("pbft consensus")
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime!");
    
    let (consensus_tx, consensus_rx) = mpsc::unbounded_channel();
    let consensus_tx_clone = consensus_tx.clone();
    //recevice msg from network and handle it
    info!("Start Pbft network!");
    // let (s_tx, s_rx) = oneshot::channel();
    runtime.spawn(async move{
        while let Some(rev) = network_tasks.recv().await{   
            match rev {
                NetworkProtocol::SendToOne(_, msg) =>{
                    let msg: PbftMessage = bcs::from_bytes(&msg.get_data()).unwrap();
                    // info!("Recevice Message: {:?}", msg);
                    consensus_tx.send(msg);
                }
                _ => todo!()
            }
        }
    });
    let one = time::Duration::from_secs(1);
    thread::sleep(one);
    // network_task.
    let peer_id = vec![config.id as u8];
    //send msg to network
    let pbft_sender = PbftSender::new(network_sender);

    runtime.spawn(async move{
        PbftNode::start(peer_id,config.id, &config, Arc::new(pbft_sender), consensus_tx_clone,consensus_rx);
    });
    

    runtime
    
}

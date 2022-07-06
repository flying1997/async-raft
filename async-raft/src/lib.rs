#![cfg_attr(feature = "docinclude", feature(external_doc))]
#![cfg_attr(feature = "docinclude", doc(include = "../README.md"))]

pub mod config;
mod core;
pub mod metrics;
pub mod raft;
mod replication;
mod network_task;
use network_task::NetworkTask;
use tokio::{runtime::{self, Runtime}, sync::mpsc::{self, UnboundedReceiver}};
use types::network::network_message::RpcRequest;

pub use crate::{
    config::{Config, ConfigBuilder, SnapshotPolicy},
    core::State,
    metrics::RaftMetrics,
    raft::Raft,
};

pub fn start_consensus(network_tasks: UnboundedReceiver<RpcRequest>) -> Runtime{
    let runtime = runtime::Builder::new_multi_thread()
        .thread_name("consensus")
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime!");
    // let (tx, rx) = mpsc::unbounded_channel();
    // let network_task = NetworkTask{
    //     network_tasks,
    //     consensus_tx: tx, 
    // };
    // runtime.spawn(network_task.start());

    
    runtime    
}


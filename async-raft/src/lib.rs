#![cfg_attr(feature = "docinclude", feature(external_doc))]
#![cfg_attr(feature = "docinclude", doc(include = "../README.md"))]

pub mod config;
mod core;
pub mod metrics;
pub mod raft;
mod replication;
mod network_task;
mod raft_sender;
use std::sync::Arc;

use memstore::{MemStore};
use raft_sender::RaftSender;
use tokio::{runtime::{self, Runtime}, sync::{mpsc::{self}, oneshot, watch}};
use types::{app_data::NodeId, client::{ClientRequest, ClientResponse}, raft::RaftMsg};

pub use crate::{
    config::{Config, ConfigBuilder, SnapshotPolicy},
    core::State,
    core::RaftCore,
    metrics::RaftMetrics,
    raft::Raft,
};

pub fn start_consensus( id: NodeId, config: Arc<Config>, network: Arc<RaftSender>, storage: Arc<MemStore>, rx_api: mpsc::UnboundedReceiver<RaftMsg<ClientRequest, ClientResponse>>,
    tx_metrics: watch::Sender<RaftMetrics>, rx_shutdown: oneshot::Receiver<()>) -> Runtime{
    let runtime = runtime::Builder::new_multi_thread()
        .thread_name("consensus")
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime!");
    let raft = RaftCore::spawn(id, config, network, storage, rx_api, tx_metrics, rx_shutdown);
    runtime.spawn(raft);
    
    runtime    
}


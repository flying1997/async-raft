use std::{collections::HashSet, path::PathBuf, str::FromStr, sync::{Arc, atomic::{AtomicBool, Ordering}}};

use memstore::MemStore;
use rl_logger::{Level, Rlogger, debug, info, prelude::FileWriter};
use tokio::{runtime::Runtime, 
    sync::{
        mpsc,
        oneshot,
        watch,
    }
};
use types::{config::{NodeConfig, logger_config::LoggerConfig}, network::network_message::RpcContent};
use network::network::Network;
use async_raft::{
    Raft,
    raft_sender::RaftSender,
    metrics::RaftMetrics,
};
pub struct Handler{
    network_runtime: Runtime,
    consensus_runtime: Runtime,
}

pub fn start(node_config: &NodeConfig, log_file: Option<PathBuf>) {

    // Setup logger environment and build Rlogger
    //let log_file = Some(PathBuf::from("./log"));
    let _logger = _setup_logger_env(node_config.get_log_config(),log_file);
    // Log some essential info, since the logger is set up
    info!(config = node_config, "NodeConfig = config");

    let _handlers = setup_environment(node_config);

    let term = Arc::new(AtomicBool::new(false));
    while !term.load(Ordering::Acquire) {
        std::thread::park();
    }
}

pub fn setup_environment(node_config: &NodeConfig) -> Handler{
    let id = node_config.get_network_config().unwrap().get_address().get_id();
    let (network_tx, network_rx) = mpsc::unbounded_channel();
    
    let (tx_metrics, rx_metrics) = watch::channel(RaftMetrics::new_initial(id));
    
    let (consensus_tx, consensus_rx) = mpsc::unbounded_channel();

    info!("Start Network!");
    let network_runtime = network::start(node_config.get_network_config().unwrap(), network_rx, consensus_tx);
    
    
    // start raft consensus
    let memstore = Arc::new(MemStore::new(id));
    let (tx_shutdown, rx_shutdown) = oneshot::channel();
    let (tx_api, rx_api) = mpsc::unbounded_channel();
    let raft_interface = Arc::new(Raft::create(tx_api, rx_metrics, tx_shutdown));
    let raft_sender = Arc::new(RaftSender::new(id, raft_interface.clone(), network_tx.clone()));
    
    let consensus_runtime = async_raft::start_consensus(id, node_config, consensus_rx, raft_interface.clone(), raft_sender, memstore, rx_api, tx_metrics, rx_shutdown);
    
    let mut members = HashSet::new();
    members.insert(0);
    members.insert(1);
    members.insert(2);
    members.insert(3);
    let exce = consensus_runtime.handle().clone();
    exce.spawn(async move{
        raft_interface.initialize(members).await;
    });
    



    Handler {  
        network_runtime,
        consensus_runtime,
    }
}

fn _setup_logger_env(logger_config: Option<LoggerConfig>, log_file: Option<PathBuf>) -> Option<Arc<Rlogger>> {
    let log_config = match logger_config {
        None => LoggerConfig::default(),
        Some(l) => l
    };
    let mut logger = rl_logger::Logger::new();
    logger
        .channel_size(log_config.channel_size)
        .is_async(log_config.is_async)
        .level(Level::from_str(&log_config.level).unwrap())
        .read_env();
    if let Some(log_file) = log_file {
        logger.printer(Box::new(FileWriter::new(log_file)));
    }
    let logger = Some(logger.build());
    return logger;
}

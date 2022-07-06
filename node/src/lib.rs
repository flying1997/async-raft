use std::{path::PathBuf, str::FromStr, sync::{Arc, atomic::{AtomicBool, Ordering}}};

use rl_logger::{Level, Rlogger, info, prelude::FileWriter};
use tokio::{runtime::Runtime, sync::mpsc};
use types::config::{NodeConfig, logger_config::LoggerConfig};
use network::network::Network;
pub struct Handler{
    network_runtime: Runtime,
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
    let (network_tx, network_rx) = mpsc::unbounded_channel();
    let (raft_tx, raft_rx) = mpsc::unbounded_channel();
    let network_runtime = network::start(node_config.get_network_config().unwrap(), network_rx, raft_tx);
    
    // let (tx_api, rx_api) = mpsc::unbounded_channel();
    // let (tx_metrics, rx_metrics) = watch::channel(RaftMetrics::new_initial(id));
    // let (tx_shutdown, rx_shutdown) = oneshot::channel();
    // let raft_handle = RaftCore::spawn(id, config, network, storage, rx_api, tx_metrics, rx_shutdown);



    Handler {  
        network_runtime,
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

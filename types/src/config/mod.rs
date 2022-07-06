use std::{fs::File, io::Read};
use serde_yaml;
use self::{logger_config::LoggerConfig, network_config::NetworkConfig};
use serde::{Deserialize, Serialize};

pub mod network_config;
pub mod logger_config;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct NodeConfig{
    network: Option<NetworkConfig>,
    logger: Option<LoggerConfig>,
}

impl  NodeConfig {
    pub fn get_log_config(&self) -> Option<LoggerConfig>{
        self.logger.clone()
    }
    pub fn get_network_config(&self) -> Option<NetworkConfig> {
        self.network.clone()
    }
    pub fn from_dir(dir: &String) -> Self {
        let mut file = File::open(&dir).expect("Cannot Open Cluster Config File.");
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("Cannot read yaml from string.");
        let node_config: NodeConfig = serde_yaml::from_str(&contents).expect("Error in parse 'ClusterConfig'");
        node_config
    }
}
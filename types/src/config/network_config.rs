use std::str::FromStr;

use crate::network::network_address::{self, NetworkAddress};

use serde::{Deserialize, Serialize};


#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NetworkConfig{
    listen_address: String,
    peer_address: Vec<String>,
}

impl NetworkConfig{
    pub fn get_address(&self) -> NetworkAddress{
        NetworkAddress::from_str(self.listen_address.as_str()).unwrap()
    }
    pub fn get_peer_address(&self) -> Vec<NetworkAddress>{
        let mut peers = Vec::new();
        for i in &self.peer_address{
            peers.push(NetworkAddress::from_str(i.as_str()).unwrap());
        }
        peers
    }
}
impl Default for NetworkConfig{
    fn default() -> Self {
        Self { 
            listen_address: Default::default(), 
            peer_address: Default::default()
        }
    }
}
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum NetworkMessage{
    HeartBeat,
    RpcRequest(RpcRequest),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcRequest{
    from: u64, // receiver
    to: u64,
    data: Vec<u8>,
}
impl RpcRequest{
    pub fn get_from(&self) -> u64{
        self.from
    }
    pub fn get_to(&self) -> u64{
        self.to
    }
    pub fn get_data(&self) -> Vec<u8>{
        self.data.clone()
    }
}
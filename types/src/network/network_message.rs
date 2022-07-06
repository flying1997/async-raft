use serde::{Deserialize, Serialize};

use crate::{client::ClientRequest, raft::{AppendEntriesRequest, ClientWriteRequest, InstallSnapshotRequest, VoteRequest}};

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
    pub fn new(from: u64, to: u64, data: Vec<u8>) -> Self{
        Self{
            from,
            to,
            data,
        }
    }
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
pub struct RpcResponce{
    state: u64,
    data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum  RpcType {
    AppendEntries(AppendEntriesRequest<ClientRequest>),
    Snapshot(InstallSnapshotRequest),
    Vote(VoteRequest),
    TransferTxn(ClientWriteRequest<ClientRequest>),
}
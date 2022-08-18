
use serde::{Deserialize, Serialize};

use crate::{
    client::ClientRequest, 
    raft::{AppendEntriesRequest, 
        ClientWriteRequest, 
        InstallSnapshotRequest, 
        VoteRequest,
    },
    mutex::Mutex,
};
use once_cell::sync::Lazy;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkProtocol{
    HeartBeat,
    RpcRequest(RpcContent),
    RpcResponce(RpcContent, RpcResponceState),

    SendToOne(u64, Message),
    // Broadcast,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMessage{
    len: u64,
    data: Vec<u8>,
}

impl NetworkMessage{
    pub fn new(data: Vec<u8>) ->Self{
        Self{
            len: data.len() as u64,
            data,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcContent{
    from: u64, // receiver
    to: u64,
    serial: u64,
    data: Vec<u8>,
}
impl RpcContent{
    pub fn new(from: u64, to: u64, serial: Option<u64>, data: Vec<u8>) -> Self{
        let serial  = match serial{
            Some(i) => {i}
            None => get_availale_serial(),
        };
        Self{
            from,
            to,
            serial,
            data,
            // rpc_type,
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
    pub fn get_serial(&self) -> u64{
        self.serial
    }

}

#[derive(Debug, Serialize, Deserialize)]
pub enum  RpcType {
    Raft(Raft),
    Pbft,
}   

#[derive(Debug, Serialize, Deserialize)]
pub enum Raft{
    AppendEntries(AppendEntriesRequest<ClientRequest>),
    Snapshot(InstallSnapshotRequest),
    Vote(VoteRequest),
    TransferTxn(ClientWriteRequest<ClientRequest>),
}
#[derive(Debug, Serialize, Deserialize)]
pub enum Pbft{

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RpcResponceState{
    Success,
}
pub struct SerialNum(u64);

pub static mut RPC_SERIAL:Lazy<Mutex<SerialNum>> = Lazy::new(|| Mutex::new(SerialNum(0)));

fn get_availale_serial() -> u64 {
    unsafe {
        let mut rpc_serial = (&*RPC_SERIAL).lock();
        let serial_num = rpc_serial.0;
        rpc_serial.0 = rpc_serial.0.checked_add(1).unwrap_or(0);
        serial_num
    }
}
#[derive(Debug)]
//

pub enum ConnectionType {
    Close,
    RpcErr(u64),
    RpcResponse(RpcContent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct Message{

    data: Vec<u8>,
}
impl Message {
    pub fn new(data: Vec<u8>) -> Self{
        Self{
            data,
        }
    }
    pub fn get_data(&self) -> Vec<u8> {
        self.data.clone()
    }
}
pub struct MessageResponse{
    
}
impl MessageResponse{
    pub fn new() -> Self{
        Self{

        }
    }
}
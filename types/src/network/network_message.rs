
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
pub enum NetworkMessage{
    HeartBeat,
    RpcRequest,
    RpcResponce(RpcResponceState),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcContent{
    from: u64, // receiver
    to: u64,
    serial: u64,
    rpc_type: NetworkMessage,
    data: Vec<u8>,
}
impl RpcContent{
    // pub fn new(from: u64, to: u64, data: Vec<u8>) -> Self{
    //     Self{
    //         from,
    //         to,
    //         serial: get_availale_serial(),
    //         data,
    //     }
    // }
    pub fn new(from: u64, to: u64, serial: u64, data: Vec<u8>, rpc_type: NetworkMessage) -> Self{
        Self{
            from,
            to,
            serial,
            data,
            rpc_type,
        }
    }
    pub fn new_rpc_request(from: u64, to: u64, data: Vec<u8>) -> Self{
        Self{
            from,
            to, 
            serial: get_availale_serial(),
            data,
            rpc_type: NetworkMessage::RpcRequest
        }
    }
    pub fn new_rpc_responce(from: u64, to: u64, serial: u64, data: Vec<u8>, state: RpcResponceState) -> Self{
        Self{
            from,
            to, 
            serial,
            data,
            rpc_type: NetworkMessage::RpcResponce(state),
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

    pub fn get_rpc_type(&self) -> NetworkMessage{
        self.rpc_type.clone()
    }
}

// #[derive(Debug, Serialize, Deserialize)]
// pub struct RpcResponce{
//     state: RpcResponceState,
//     serial: u64,
//     data: Vec<u8>,
// }

// impl RpcResponce{
//     pub fn new(state: RpcResponceState, serial: u64, data: Vec<u8>) -> Self{
//         Self{
//             state,
//             serial,
//             data
//         }
//     }
//     pub fn get_state(&self) -> RpcResponceState{
//         self.state.clone()
//     }
//     pub fn get_data(&self) -> Vec<u8>{
//         self.data.clone()
//     }
//     pub fn get_serial(&self) -> u64{
//         self.serial
//     }
// }
#[derive(Debug, Serialize, Deserialize)]
pub enum  RpcType {
    AppendEntries(AppendEntriesRequest<ClientRequest>),
    Snapshot(InstallSnapshotRequest),
    Vote(VoteRequest),
    TransferTxn(ClientWriteRequest<ClientRequest>),
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
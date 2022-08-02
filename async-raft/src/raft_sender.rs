use std::sync::Arc;

use common_trait::network::RaftNetwork;
use memstore::{MemStore};
use rl_logger::{debug, error};
use tokio::sync::{mpsc::UnboundedSender, oneshot};
use types::{client::{ClientRequest, ClientResponse}, network::network_message::{NetworkProtocol, RpcContent, RpcResponceState, RpcType}, raft::{AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse, VoteRequest, VoteResponse}};
use anyhow::{Result, anyhow};
use crate::Raft;
use types::network::network_message::{
    Raft::{
        AppendEntries,
        Vote,
        Snapshot,
        TransferTxn,
    }
};
use async_trait::async_trait;
pub struct RaftSender {
    node_id: u64,
    raft_interface: Arc<Raft<ClientRequest, ClientResponse, RaftSender, MemStore>>,
    network_tx: UnboundedSender<(NetworkProtocol, oneshot::Sender<NetworkProtocol>)>,
}

impl RaftSender {
    pub fn new(
        node_id: u64,
        raft_interface: Arc<Raft<ClientRequest, ClientResponse, RaftSender, MemStore>>,
        network_tx: UnboundedSender<(NetworkProtocol, oneshot::Sender<NetworkProtocol>)>,
    ) -> Self {
        Self{ 
            node_id,
            raft_interface,
            network_tx,
        }
    }
    pub fn get_network(&self) -> UnboundedSender<(NetworkProtocol, oneshot::Sender<NetworkProtocol>)>{
        self.network_tx.clone()
    }
}

#[async_trait]
impl RaftNetwork<ClientRequest> for RaftSender {
    /// Send an AppendEntries RPC to the target Raft node (§5).
    async fn append_entries(&self, target: u64, rpc: AppendEntriesRequest<ClientRequest>) -> Result<AppendEntriesResponse> {
        if rpc.entries.len() > 0 {
            debug!(target = target,payload=rpc,"Send the payload");
        }
        if target == self.node_id {
            return Ok(self.raft_interface.append_entries(rpc).await?)
        }
        let sender = self.network_tx.clone();
        let (tx, rx) = oneshot::channel();

        let _ = sender.send((NetworkProtocol::RpcRequest(RpcContent::new(self.node_id, target, None,
         bcs::to_bytes(&RpcType::Raft(AppendEntries(rpc))).unwrap())), tx));
        let t = rx.await?; 
        if let NetworkProtocol::RpcResponce(req, state) = t{
            match state {
                RpcResponceState::Success =>{
                    let res: AppendEntriesResponse = bcs::from_bytes(&req.get_data()).unwrap();
                    Ok(res)
                }
            }
        }else{
            Err(anyhow!("RpcType Error!!"))
        }
        // {
            // RpcTaskResponce::Timeout => {
            //     error!("[RaftSender] Rpc timeout.");
            //     return Err(anyhow!("Rpc Timeout"))
            // },
            // RpcTaskResponce::Isolated => {
            //     error!("[RaftSender] Rpc isolated.");
            //     return Err(anyhow!("Rpc Isolated"))
            // },
            // RpcTaskResponce::Responce(buf) => {
            //     let append_rsp: AppendEntriesResponse = bcs::from_bytes(&buf).unwrap();
            //     return Ok(append_rsp);
            // },
        // }
    }

    /// Send an InstallSnapshot RPC to the target Raft node (§7).
    async fn install_snapshot(&self, target: u64, rpc: InstallSnapshotRequest) -> Result<InstallSnapshotResponse> {
        if target == self.node_id {
            return Ok(self.raft_interface.install_snapshot(rpc).await?)
        }
        let sender = self.network_tx.clone();
        let (tx, rx) = oneshot::channel();

        let _ = sender.send((NetworkProtocol::RpcRequest(RpcContent::new(self.node_id, target, None,
            bcs::to_bytes(&RpcType::Raft(Snapshot(rpc))).unwrap())), tx));
        let t = rx.await?; 
        if let NetworkProtocol::RpcResponce(req, state) = t{
            match state {
                RpcResponceState::Success =>{
                    let res: InstallSnapshotResponse = bcs::from_bytes(&req.get_data()).unwrap();
                    Ok(res)
                }
            }
        }else{
            Err(anyhow!("RpcType Error!!"))
        }
        // let _ = sender.send((RpcTask::new(self.node_id, target, RpcType::Snapshot(rpc)), tx));
        // match rx.await? {
        //     RpcTaskResponce::Timeout => {
        //         error!("[RaftSender] Rpc timeout.");
        //         return Err(anyhow!("Rpc Timeout"))
        //     },
        //     RpcTaskResponce::Isolated => {
        //         return Err(anyhow!("Rpc Timeout"))
        //     },
        //     RpcTaskResponce::Responce(buf) => {
        //         let install_rsp: InstallSnapshotResponse = bcs::from_bytes(&buf).unwrap();
        //         return Ok(install_rsp);
        //     },
        // }
    }

    /// Send a RequestVote RPC to the target Raft node (§5).
    async fn vote(&self, target: u64, rpc: VoteRequest) -> Result<VoteResponse> {
        debug!("Vote Message:{:?}", rpc);
        if target == self.node_id {
            return Ok(self.raft_interface.vote(rpc).await?)
        }
        let sender = self.network_tx.clone();
        let (tx, rx) = oneshot::channel();

        let _ = sender.send((NetworkProtocol::RpcRequest(RpcContent::new(self.node_id, target, None,
            bcs::to_bytes(&RpcType::Raft(Vote(rpc))).unwrap())), tx));
        let t = rx.await?; 
        if let NetworkProtocol::RpcResponce(req, state) = t{
            match state {
                RpcResponceState::Success =>{
                    let res: VoteResponse = bcs::from_bytes(&req.get_data()).unwrap();
                    Ok(res)
                }
            }
        }else{
            Err(anyhow!("RpcType Error!!"))
        }
        // let _ = sender.send((RpcTask::new(self.node_id, target, RpcType::Vote(rpc)), tx));
        // match rx.await? {
        //     RpcTaskResponce::Timeout => {
        //         error!("[RaftSender] Rpc timeout.");
        //         return Err(anyhow!("Rpc Timeout"))
        //     },
        //     RpcTaskResponce::Isolated => {
        //         return Err(anyhow!("Rpc Timeout"))
        //     },
        //     RpcTaskResponce::Responce(buf) => {
        //         let vote_rsp: VoteResponse = bcs::from_bytes(&buf).unwrap();
        //         return Ok(vote_rsp);
        //     },
        // }
    }

    // async fn trans_tx_to_leader(&self, target: u64, rpc: ClientWriteRequest<MemClientRequest>) -> Result<ClientWriteResponse<MemClientResponse>, ClientWriteError<MemClientRequest>> {
    //     if target == self.node_id {
    //         unreachable!()
    //     }
    //     let entry = (&rpc.entry).clone();
    //     let sender = self.network_tx.clone();
    //     let (tx, rx) = oneshot::channel();
    //     let _ = sender.send((RpcTask::new(self.node_id, target, RpcType::TransferTxn(rpc)), tx));
    //     let _ = rx.await;
    //     if let EntryPayload::Normal(en) = entry {
    //         Err(ClientWriteError::ForwardToLeader(en.data, Some(target)))
    //     }
    //     else {
    //         unreachable!();
    //     }
    // }
}
// use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
// use types::{app_data::{AppData, AppDataResponse}, network::network_message::RpcRequest, raft::RaftMsg};
// use bcs;

// pub struct NetworkTask<D, R>
// where
//     D: AppData,
//     R: AppDataResponse
//  {
//     network_tasks: UnboundedReceiver<RpcRequest>,
//     consensus_tx: UnboundedSender<RaftMsg<D, R>>,
// }

// impl<D, R> NetworkTask<D, R>
// where
//     D: AppData,
//     R: AppDataResponse
// {
//     pub async fn start(mut self){
//         loop{
//             tokio::select! {
//                 //receive message from network
//                 rev = self.network_tasks.recv()  =>{
//                     // let msg = bcs::from_bytes(rev.unwrap().as_bytes());
//                     // self.consensus_tx.send(msg);
//                 }
//             }
//         }
//     }
// }
use std::{collections::HashMap, str::FromStr};

use rl_logger::debug;
use types::{config::{network_config::NetworkConfig}, network::{network_address::NetworkAddress, network_message::{NetworkMessage, RpcRequest}}};
use bcs::Result as BcsResult;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, sync::{mpsc::{self, UnboundedReceiver, UnboundedSender}}};
pub struct Network{
    id: u64,
    nodes: HashMap<u64, String>,  
    peers: HashMap<String, u64>, 
    stream_close: HashMap<u64, UnboundedSender<()>>,
    rpc_table: HashMap<u64, UnboundedSender<RpcRequest>>,
    network_rx: UnboundedReceiver<RpcRequest>,
    raft_tx: UnboundedSender<RpcRequest>,
}
impl Network {
    pub fn new(config: NetworkConfig, network_rx: UnboundedReceiver<RpcRequest>,
        raft_tx: UnboundedSender<RpcRequest>,) -> Self{ 
        let network_address = config.get_address();
        let peer_adrress = config.get_peer_address();
        let id = network_address.get_id();
        let mut nodes = HashMap::new();
        let mut peers = HashMap::new();
        for peer in peer_adrress{
            nodes.insert(peer.get_id(), peer.get_address());
            peers.insert(peer.get_address(), peer.get_id());
        }
        Self{
            id,
            nodes,
            peers,
            stream_close: HashMap::new(),
            rpc_table: HashMap::new(),
            network_rx,
            raft_tx,
        }
    }    
    pub fn handle_request(&mut self, mut stream: TcpStream, addr: String){
        let receive_id = self.peers.get(&addr).unwrap();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let (rpc_tx, mut rpc_rx) = mpsc::unbounded_channel();
        if self.stream_close.contains_key(receive_id) {
            self.stream_close.get(receive_id).unwrap().send(());
        }
        self.stream_close.insert(*receive_id, tx);
        self.rpc_table.insert(*receive_id, rpc_tx);
        let raft_tx = self.raft_tx.clone();
        tokio::spawn(async move{
            loop{
                let mut buff = [0u8; 8192];
                tokio::select! {
                    _ = rx.recv() =>{
                        break;
                    }
                    rpc = rpc_rx.recv() =>{
                        let rpc = rpc.unwrap();
                        let req = NetworkMessage::RpcRequest(rpc);
                        let msg = bcs::to_bytes(&req).unwrap();
                        stream.write_all(&msg).await;
                        debug!("Sender Message {:?}", req);
                    }
                    n = stream.read(&mut buff) =>{
                        if let Ok(n) = n{
                            let req: BcsResult<NetworkMessage> = bcs::from_bytes(&buff[..n]);
                            debug!("Receive Message:{:?}", req);
                            if let Ok(req) = req{
                                match req{
                                    NetworkMessage::RpcRequest(data) =>{
                                        raft_tx.send(data);
                                    },
                                    NetworkMessage::HeartBeat =>{

                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    }
    pub async fn start(mut self) {
        let id = self.id;
        let address = self.nodes.get(&id).unwrap();
        let mut listener = TcpListener::bind(address).await.expect("Network bind error!");
        loop{

            tokio::select! {
                stream = listener.accept() =>{
                    match stream{
                        Ok((stream, addr)) =>{
                            self.handle_request(stream, addr.to_string());
                        }
                        Err(e) => {

                        }
                    }
                }
                rev = self.network_rx.recv() =>{
                    let rev = rev.unwrap();
                    let tx = self.rpc_table.get(&rev.get_to()).unwrap();
                    tx.send(rev);
                }

            }
            //receive request
            
            
            
            
        }
    }

    pub async fn connect(&mut self){
        let len = self.nodes.len() as u64;
        for i in self.id+1..len{
            let stream = TcpStream::connect(self.nodes.get(&i).unwrap()).await.unwrap();
            self.handle_request(stream, self.nodes.get(&i).unwrap().clone());
        }
    }
}
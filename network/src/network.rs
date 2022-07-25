use std::{collections::HashMap, iter::StepBy, str::FromStr, sync::Arc, time::Duration};

use async_raft::{Raft, raft_sender::RaftSender};
use memstore::MemStore;
use rl_logger::{debug, warn, error};
use types::{client::{ClientRequest, ClientResponse}, config::{network_config::NetworkConfig}, network::{network_address::NetworkAddress, network_message::{NetworkMessage, RpcContent, RpcResponceState}}};
use bcs::Result as BcsResult;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, sync::{mpsc::{self, UnboundedReceiver, UnboundedSender}, oneshot}, time::interval};

/// every stream correspond a thread
pub struct Network{
    id: u64,
    nodes: HashMap<u64, String>,  
    peers: HashMap<String, u64>, 
    stream_close_rx: Option<UnboundedReceiver<u64>>, //control other peers stream thread whether shutdown
    stream_tx_table: HashMap<u64, UnboundedSender<RpcContent>>, // sender message to destination stream
    rpc_table: HashMap<u64, oneshot::Sender<RpcContent>>,
    network_rx: UnboundedReceiver<(RpcContent, oneshot::Sender<RpcContent>)>, // receive consensus tasks from Consensus
    // raft_interface: Arc<Raft<ClientRequest, ClientResponse, RaftSender, MemStore>>,
    raft_tx: UnboundedSender<RpcContent>,
}
impl Network {
    pub fn new(config: NetworkConfig, network_rx: UnboundedReceiver<(RpcContent, oneshot::Sender<RpcContent>)>,
        raft_tx: UnboundedSender<RpcContent>,
        // raft_interface: Arc<Raft<ClientRequest, ClientResponse, RaftSender, MemStore>>,
    ) -> Self{ 
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
            stream_close_rx: None,
            stream_tx_table: HashMap::new(),
            rpc_table: HashMap::new(),
            network_rx,
            raft_tx,
        }
    }    
    pub fn handle_request(&mut self, mut stream: TcpStream, addr: String, stream_tx: UnboundedSender<RpcContent>,
    stream_close_tx: UnboundedSender<u64>){
        let receive_id = *self.peers.get(&addr).unwrap();
        let (rpc_tx, mut rpc_rx) = mpsc::unbounded_channel();

        self.stream_tx_table.insert(receive_id, rpc_tx);
        let raft_tx = self.raft_tx.clone();
        tokio::spawn(async move{
            let id = receive_id;
            let mut heart_ticker = interval(Duration::from_secs(3));
            heart_ticker.tick().await;
            loop{
                // match stream.t
                let mut buff = [0u8; 8192];
                tokio::select! {
                    _ = heart_ticker.tick() => {
                        stream_close_tx.send(id);
                        break;
                    }
                    rpc = rpc_rx.recv() =>{
                        let rpc = rpc.unwrap();
                        // let req = NetworkMessage::RpcRequest(rpc);
                        let msg = bcs::to_bytes(&rpc).unwrap();             
                        stream.write_all(&msg).await;
                        debug!("Sender Message {:?}", rpc);
                    }
                    n = stream.read(&mut buff) =>{
                        if let Ok(n) = n{
                            let req: BcsResult<RpcContent> = bcs::from_bytes(&buff[..n]);
                            debug!("Receive Message:{:?}", req);
                            if let Ok(req) = req{
                                match req.get_rpc_type(){
                                    NetworkMessage::RpcRequest =>{
                                        // let _ = stream_tx.send(rpc);
                                        raft_tx.send(req);
                                    },
                                    NetworkMessage::RpcResponce(_) =>{
                                        let _ = stream_tx.send(req);
                                    }
                                    NetworkMessage::HeartBeat =>{
                                        heart_ticker.reset();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    }
    pub async fn start(mut self, mut finish: Option<oneshot::Sender<()>>) {
        debug!("id: {}, nodes:{:?}, peers:{:?}", self.id, self.nodes, self.peers);
        let id = self.id;
        let address = self.nodes.get(&id).unwrap();
        let listener = TcpListener::bind(address).await.expect("Network bind error!");
        let (stream_tx, mut stream_rx) = mpsc::unbounded_channel();
        let (stream_close_tx, stream_close_rx) = mpsc::unbounded_channel();
        self.stream_close_rx = Some(stream_close_rx);
        self.connect(stream_tx.clone(), stream_close_tx.clone()).await;
        loop{
            debug!("Stream table:{:?}", self.stream_tx_table.keys());
            if self.stream_tx_table.len() == self.peers.len()-1{
                if let Some(tx) = finish.take(){
                    tx.send(());
                }
            }
            tokio::select! {
                stream = listener.accept() =>{
                    match stream{
                        Ok((mut stream, addr)) =>{
                            let mut buff =  [0u8; 1024];
                            let n = stream.read(&mut buff[..1024]).await.unwrap();
                            let v = String::from(std::str::from_utf8(&buff[..n]).unwrap());
                             debug!("client Addr: {}, {:?}", addr, v);
                            self.handle_request(stream, v, stream_tx.clone(), stream_close_tx.clone());
                        }
                        Err(_e) => {

                        }
                    }
                }
                rev = self.network_rx.recv() =>{
                    let (rev, rpc_tx) = rev.unwrap();
                    debug!("recevice message from consensus: {:?}", rev);
                   
                    match self.stream_tx_table.get(&rev.get_to()){
                        Some(tx) =>{
                            self.rpc_table.insert(rev.get_serial(), rpc_tx);
                            let _= tx.send(rev);
                        }
                        None =>{
                            warn!("Romote node {} losed!!", rev.get_to());
                        }
                    }
                    
                }

                rev = stream_rx.recv() => {
                    let rpc = rev.unwrap();
                    match self.rpc_table.remove(&rpc.get_serial()){
                        Some(tx) => {
                            let _ = tx.send(rpc);
                        }
                        None => {
                            warn!("Rpc error from remote node!");
                        }
                    }
                }
            
                rev = self.stream_close_rx.as_mut().unwrap().recv() =>{
                    let id = rev.unwrap();
                    self.stream_tx_table.remove(&id);
                }
            }
            // debug!("Next Stream table:{:?}", self.stream_tx_table);
            
            //receive request
            
            
            
            
        }
    }

    pub async fn connect(&mut self, tx: UnboundedSender<RpcContent>, stream_close_tx: UnboundedSender<u64>){
        let len = self.peers.len() as u64;
        for i in 0..len{
            if i == self.id{
                continue;
            }
            match TcpStream::connect(self.nodes.get(&i).unwrap()).await{
                Ok(mut stream) =>{ 
                    let _ = stream.write_all(self.nodes.get(&self.id).unwrap().as_bytes()).await;
                    self.handle_request(stream, self.nodes.get(&i).unwrap().clone(), tx.clone(), stream_close_tx.clone());
                   
                }
                Err(err) =>{
                    error!("{:?}", err);
                }
            }
           
        }
    }
}
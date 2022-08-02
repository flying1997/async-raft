use std::{collections::HashMap, iter::StepBy, str::FromStr, sync::Arc, time::Duration};

use rl_logger::{debug, error, info, warn};
use types::{config::{network_config::NetworkConfig}, network::{network_address::NetworkAddress, network_message::{ConnectionType, NetworkProtocol, RpcContent, RpcResponceState, RpcType}}};
use bcs::Result as BcsResult;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, sync::{mpsc::{self, UnboundedReceiver, UnboundedSender}, oneshot}, time::interval};

/// every stream correspond a thread
pub struct Network{
    id: u64,
    nodes: HashMap<u64, String>,  
    peers: HashMap<String, u64>, 

    stream_tx_table: HashMap<u64, UnboundedSender<NetworkProtocol>>, // sender message to destination stream
    
    rpc_table: HashMap<u64, oneshot::Sender<NetworkProtocol>>,

    network_rx: UnboundedReceiver<(NetworkProtocol, oneshot::Sender<NetworkProtocol>)>, // receive consensus tasks from Consensus
    consensus_tx: UnboundedSender<NetworkProtocol>,
}
impl Network {
    pub fn new(config: NetworkConfig, network_rx: UnboundedReceiver<(NetworkProtocol, oneshot::Sender<NetworkProtocol>)>,
    consensus_tx: UnboundedSender<NetworkProtocol>,
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
            stream_tx_table: HashMap::new(),
            rpc_table: HashMap::new(),
            network_rx,
            consensus_tx,
        }
    }    
    pub fn handle_request(&mut self, mut stream: TcpStream, addr: String, stream_tx: UnboundedSender<ConnectionType>){
        let receive_id = *self.peers.get(&addr).unwrap();
        let (rpc_tx, mut rpc_rx) = mpsc::unbounded_channel();

        self.stream_tx_table.insert(receive_id, rpc_tx);
        let consensus_tx = self.consensus_tx.clone();
        tokio::spawn(async move{
            let id = receive_id;
            // let mut heart_ticker = interval(Duration::from_secs(3));
            // heart_ticker.tick().await;
            loop{
                // match stream.t
                let mut buff = [0u8; 8192];
                tokio::select! {
                    // _ = heart_ticker.tick() => {
                    //     stream_close_tx.send(id);
                    //     break;
                    // }
                    req = rpc_rx.recv() =>{
                        let req = req.unwrap();
                        let msg = bcs::to_bytes(&req).unwrap();             
                        match stream.write_all(&msg).await {
                            Ok(e) => {

                            }
                            Err(e) =>{
                                match req {
                                    NetworkProtocol::RpcRequest(req) =>{
                                        // let _ = stream_tx.send(rpc);
                                        stream_tx.send(ConnectionType::RpcErr(req.get_serial()));
                                        error!("{:?}", e);
                                    },
                                     _ => todo!(),
                                }
                            }
                        };
                        // debug!("Sender Message {:?}", rpc);
                    }
                    n = stream.read(&mut buff) =>{
                        if let Ok(n) = n{
                            let req: BcsResult<NetworkProtocol> = bcs::from_bytes(&buff[..n]);
                            debug!("Receive Message:{:?}", req);
                            if let Ok(req) = req{
                                match req{
                                    NetworkProtocol::RpcRequest(_) =>{
                                        // let _ = stream_tx.send(rpc);
                                        consensus_tx.send(req);
                                    },
                                    NetworkProtocol::RpcResponce(req,_) =>{
                                        let _ = stream_tx.send(ConnectionType::RpcResponse(req));
                                    }
                                    NetworkProtocol::SendToOne(_, _) =>{
                                        consensus_tx.send(req);
                                    }
                                    NetworkProtocol::HeartBeat => todo!(),
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
        
        // manager romote connections
        let (stream_tx, mut stream_rx) = mpsc::unbounded_channel();


        self.connect(stream_tx.clone()).await;
        loop{
            debug!("Stream table:{:?}", self.stream_tx_table.keys());
            // info!("Rpctable size:{}", self.rpc_table.len());
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
                            self.handle_request(stream, v, stream_tx.clone());
                        }
                        Err(_e) => {

                        }
                    }
                }
                rev = self.network_rx.recv() =>{
                    let (rev, rpc_tx) = rev.unwrap();
                    debug!("recevice message from consensus: {:?}", rev);
                    self.handle_message(rev, rpc_tx);
                    // match self.stream_tx_table.get(&rev.get_to()){
                    //     Some(tx) =>{
                    //         if let NetworkMessage::RpcRequest = rev.get_rpc_type(){
                    //             self.rpc_table.insert(rev.get_serial(), rpc_tx);
                    //         }
                    //         let _= tx.send(rev);
                    //     }
                    //     None =>{
                    //         warn!("Romote node {} losed!!", rev.get_to());
                    //     }
                    // }
                    
                }

                msg = stream_rx.recv() => {
                    let msg = msg.unwrap();
                    match msg{
                        ConnectionType::RpcErr(seq) =>{
                            match self.rpc_table.remove(&seq){
                                Some(tx) => {
                                    
                                }
                                None => {
                                    warn!("Rpc error from remote node!");
                                }
                            }
                        }
                        ConnectionType::RpcResponse(req) =>{
                            match self.rpc_table.remove(&req.get_serial()){
                                Some(tx) => {
                                    let _ = tx.send(NetworkProtocol::RpcResponce(req, RpcResponceState::Success));
                                }
                                None => {
                                    warn!("Rpc error from remote node!");
                                }
                            }
                        }
                        ConnectionType::Close => todo!(),
                    }
                }
            
               
            }
            // debug!("Next Stream table:{:?}", self.stream_tx_table);
            
            //receive request
            
            
            
            
        }
    }
    pub fn handle_message(&mut self, protocol: NetworkProtocol, tx: oneshot::Sender<NetworkProtocol>) {
        match protocol{
            NetworkProtocol::HeartBeat => todo!(),
            NetworkProtocol::RpcRequest(ref req) =>{
                self.rpc_table.insert(req.get_serial(), tx);
                self.send_to_node(req.get_to(), protocol);
                
            }
            NetworkProtocol::RpcResponce(ref req, _) => {
                self.send_to_node(req.get_to(), protocol);
            }
            NetworkProtocol::SendToOne(to,_) => {
                self.send_to_node(to, protocol);
            },
        }
    }
    pub fn send_to_node(&mut self, to: u64, msg: NetworkProtocol) {
        match self.stream_tx_table.get(&to) {
            Some(stream_tx) =>{
                stream_tx.send(msg);
            }
            None => {
                warn!("Romote node {} losed!!", to);
            }
        }
    }
    pub async fn connect(&mut self, tx: UnboundedSender<ConnectionType>){
        let len = self.peers.len() as u64;
        for i in 0..len{
            if i == self.id{
                continue;
            }
            match TcpStream::connect(self.nodes.get(&i).unwrap()).await{
                Ok(mut stream) =>{ 
                    let _ = stream.write_all(self.nodes.get(&self.id).unwrap().as_bytes()).await;
                    self.handle_request(stream, self.nodes.get(&i).unwrap().clone(), tx.clone());
                   
                }
                Err(err) =>{
                    error!("{:?}", err);
                }
            }
           
        }
    }
}
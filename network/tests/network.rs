use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use tokio::sync::mpsc;

#[test]
fn test_addr(){
    // let addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080);
    // println!("{:?}", addr.to_string().(String::from("127.0.0.1:8080").));
}
#[test]
fn test_network(){
    let (network_tx, network_rx) = mpsc::unbounded_channel();
    let (raft_tx, raft_rx) = mpsc::unbounded_channel();
    let mut network = network::network::Network::new(1, network_rx, raft_tx);
    network.start();
}
use rl_logger::error;
use types::{app_data::{ NodeId}, network::network_message::{Message, MessageResponse, NetworkProtocol}};
use tokio::sync::{mpsc::{UnboundedSender}, oneshot};
use anyhow::Result;
pub struct PbftSender{
    network_sender: UnboundedSender<(NetworkProtocol, oneshot::Sender<NetworkProtocol>)>,
}

impl PbftSender{
    pub fn new(network_sender: UnboundedSender<(NetworkProtocol, oneshot::Sender<NetworkProtocol>)>,) -> Self{
        Self{
            network_sender,
        }
    }
}
// #[async_trait]
impl PbftNetwork for PbftSender{
    fn send_to(&self, target: NodeId, msg: Message) -> Result<MessageResponse>{
        // todo!()
        let (tx, rx) = oneshot::channel();
        self.network_sender.send((NetworkProtocol::SendToOne(target, msg), tx));
        let res = MessageResponse::new();
        Ok(res)
    }

    fn broadcast(&self, target: Vec<NodeId>, msg: Message) -> Result<MessageResponse> {
        // todo!()
        for i in target.iter(){
            let (tx, _) = oneshot::channel();

            match self.network_sender.send((NetworkProtocol::SendToOne(*i, msg.clone()), tx)){
                Err(e) => {
                    error!("Send message to {} node error! error:{:?}", i, e);
                }
                _ => {}
            }
        }
        let res = MessageResponse::new();
        Ok(res)
    }
}
// #[async_trait]
pub trait PbftNetwork: Send + Sync + 'static
{
    fn send_to(&self, target: NodeId, msg: Message) -> Result<MessageResponse>;

    fn broadcast(&self, target: Vec<NodeId>, msg: Message) -> Result<MessageResponse>;

}

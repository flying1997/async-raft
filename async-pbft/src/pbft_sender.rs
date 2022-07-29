use protobuf::well_known_types::Api;
use types::{app_data::{AppData, NodeId}, client::ClientRequest};

use anyhow::Result;
use async_trait::async_trait;

pub struct PbftSender{

}

impl PbftSender{
    pub fn new() -> Self{
        Self{}
    }
}
#[async_trait]
impl PbftNetwork for PbftSender{
    async fn send_to(&self, target: NodeId, rpc: MessageRequest) -> Result<MessageResponse> {
        todo!()
    }

    async fn broadcast(&self, target: NodeId, rpc: MessageRequest) -> Result<MessageResponse> {
        todo!()
    }
}

pub struct MessageRequest{
}

pub struct MessageResponse{

}
#[async_trait]
pub trait PbftNetwork: Send + Sync + 'static
{
    async fn send_to(&self, target: NodeId, rpc: MessageRequest) -> Result<MessageResponse>;

    async fn broadcast(&self, target: NodeId, rpc: MessageRequest) -> Result<MessageResponse>;

}

/*
 * Copyright 2018 Intel Corporation
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * ------------------------------------------------------------------------------
 */

use std::error;
use std::fmt;
use std::sync::mpsc::RecvError;

use protobuf::ProtobufError;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone,Hash, PartialEq, Eq)]
pub enum PbftMessageType{
    PrePrepare,
    Prepare,
    Commit,
    ViewChange,
    NewView,
}
/// All information about a block that is relevant to consensus
#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub struct Block {
    pub block_id: BlockId,
    pub previous_id: BlockId,
    pub signer_id: PeerId,
    pub block_num: u64,
    pub payload: Vec<u8>,
}



///  
/// info:additional message 
/// if NewView, ViewChange messages from other nodes  
/// =
#[derive(Serialize, Deserialize, Debug, Clone,Hash, PartialEq, Eq)]
pub struct PbftMessage{
    msg_type: PbftMessageType,
    view: u64, //v
    seq: u64, //n
    signer_id: Vec<u8>,//i
    block_id: Vec<u8>, //d
    info: Vec<u8>, 
}
pub fn is_in_view(msg_type: &PbftMessageType) -> bool{
    match msg_type {
        PbftMessageType::ViewChange => true,
        PbftMessageType::NewView => true,
        _ => false
    }
}
impl PbftMessage {
    pub fn new(msg_type: PbftMessageType, view: u64, seq: u64, signer_id: Vec<u8>, block_id: Vec<u8>,) -> Self{
        Self{
            msg_type,
            view,
            seq,
            signer_id,
            block_id,
            info: Vec::new(),
        }
    }
    pub fn new_view(view: u64, seq: u64, signer_id: Vec<u8>,  info: Vec<u8>) ->Self{
        Self{
            msg_type: PbftMessageType::NewView,
            view,
            seq,
            signer_id,
            info,
            block_id: Vec::new(),
        }
    }
    pub fn get_view(&self) -> u64{
        self.view
    }
    pub fn set_info(&mut self, info: Vec<u8>) {
        self.info = info;
    }
    pub fn get_msg_type(&self) -> PbftMessageType{
        self.msg_type.clone()
    }
    pub fn get_signer_id(&self) -> PeerId{
        self.signer_id.clone()
    }
    pub fn get_seq_num(&self) ->u64{
        self.seq
    }
    pub fn get_block_id(&self)->Vec<u8>{
        self.block_id.clone()
    }
    pub fn get_info(&self) -> Vec<u8>{
        self.info.clone()
    }
}
pub type BlockId = Vec<u8>;

/// All information about a block that is relevant to consensus
// impl Eq for Block {}
// impl fmt::Debug for Block {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(
//             f,
//             "Block(block_num: {:?}, block_id: {:?}, previous_id: {:?}, signer_id: {:?}, payload: {})",
//             self.block_num,
//             self.block_id,
//             self.previous_id,
//             self.signer_id,
//             hex::encode(&self.payload),
//             // hex::encode(&self.summary),
//         )
//     }
// }

pub type PeerId = Vec<u8>;

/// Information about a peer that is relevant to consensus
#[derive(Default, Debug, PartialEq, Hash)]
pub struct PeerInfo {
    pub peer_id: PeerId,
}
impl Eq for PeerInfo {}

/// A consensus-related message sent between peers
#[derive(Default, Debug, Clone)]
pub struct PeerMessage {
    pub header: PeerMessageHeader,
    pub header_bytes: Vec<u8>,
    pub header_signature: Vec<u8>,
    pub content: Vec<u8>,
}

/// A header associated with a consensus-related message sent from a peer, can be used to verify
/// the origin of the message
#[derive(Default, Debug, Clone)]
pub struct PeerMessageHeader {
    /// The public key of the validator where this message originated
    ///
    /// NOTE: This may not be the validator that sent the message
    pub signer_id: Vec<u8>,
    pub content_sha512: Vec<u8>,
    pub message_type: String,
    pub name: String,
    pub version: String,
}


/// State provided to an engine when it is started
// #[derive(Debug, Default)]
// pub struct StartupState {
//     pub chain_head: Block,
//     pub peers: Vec<PeerInfo>,
//     pub local_peer_info: PeerInfo,
// }



/// Errors that occur on sending a message.
#[derive(Debug)]
pub enum SendError {
    DisconnectedError,
    TimeoutError,
    UnknownError,
}

impl std::error::Error for SendError {}

impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            SendError::DisconnectedError => write!(f, "DisconnectedError"),
            SendError::TimeoutError => write!(f, "TimeoutError"),
            SendError::UnknownError => write!(f, "UnknownError"),
        }
    }
}

/// Errors that occur on receiving a message.
#[derive(Debug, Clone)]
pub enum ReceiveError {
    TimeoutError,
    ChannelError(RecvError),
    DisconnectedError,
}

impl std::error::Error for ReceiveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ReceiveError::ChannelError(err) => Some(&*err),
            _ => None,
        }
    }
}

impl std::fmt::Display for ReceiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            ReceiveError::TimeoutError => write!(f, "TimeoutError"),
            ReceiveError::ChannelError(ref err) => write!(f, "ChannelError: {}", err),
            ReceiveError::DisconnectedError => write!(f, "DisconnectedError"),
        }
    }
}
#[derive(Debug)]
pub enum Error {
    EncodingError(String),
    SendError(String),
    ReceiveError(String),
    InvalidState(String),
    UnknownBlock(String),
    UnknownPeer(String),
    NoChainHead,
    BlockNotReady,
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Error::*;
        match *self {
            EncodingError(ref s) => write!(f, "EncodingError: {}", s),
            SendError(ref s) => write!(f, "SendError: {}", s),
            ReceiveError(ref s) => write!(f, "ReceiveError: {}", s),
            InvalidState(ref s) => write!(f, "InvalidState: {}", s),
            UnknownBlock(ref s) => write!(f, "UnknownBlock: {}", s),
            UnknownPeer(ref s) => write!(f, "UnknownPeer: {}", s),
            NoChainHead => write!(f, "NoChainHead"),
            BlockNotReady => write!(f, "BlockNotReady"),
        }
    }
}
impl From<ProtobufError> for Error {
    fn from(error: ProtobufError) -> Error {
        use self::ProtobufError::*;
        match error {
            IoError(err) => Error::EncodingError(format!("{}", err)),
            WireError(err) => Error::EncodingError(format!("{:?}", err)),
            Utf8(err) => Error::EncodingError(format!("{}", err)),
            MessageNotInitialized { message: err } => Error::EncodingError(err.to_string()),
        }
    }
}

impl From<SendError> for Error {
    fn from(error: SendError) -> Error {
        Error::SendError(format!("{}", error))
    }
}


impl From<ReceiveError> for Error {
    fn from(error: ReceiveError) -> Error {
        Error::ReceiveError(format!("{}", error))
    }
}
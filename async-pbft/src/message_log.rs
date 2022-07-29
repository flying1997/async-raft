/*
 * Copyright 2018 Bitwise IO, Inc.
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
 * -----------------------------------------------------------------------------
 */

//! The message log used by PBFT nodes to save messages

#![allow(unknown_lints)]

use std::collections::{HashMap, HashSet};
use std::fmt;

use rl_logger::trace;
use types::pbft::consensus::{Block, BlockId, PbftMessage, PbftMessageType};

use crate::config::PbftConfig;

/// Struct for storing messages that a PbftNode receives
pub struct PbftLog {
    /// All blocks received from the validator that have not been validated yet
    unvalidated_blocks: HashMap<BlockId, Block>,

    /// All blocks received from the validator that have been validated and not yet garbage
    /// collected
    blocks: HashSet<Block>,

    /// All messages accepted by the node that have not been garbage collected
    messages: HashSet<PbftMessage>,

    /// Maximum log size
    max_log_size: u64,
}

// impl fmt::Display for PbftLog {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         let msg_infos: Vec<PbftMessageInfo> =
//             self.messages.iter().map(|msg| msg.info().clone()).collect();
//         let string_infos: Vec<String> = msg_infos
//             .iter()
//             .map(|info: &PbftMessageInfo| -> String {
//                 format!(
//                     "    {{ {}, view: {}, seq: {}, signer: {} }}",
//                     info.get_msg_type(),
//                     info.get_view(),
//                     info.get_seq_num(),
//                     hex::encode(info.get_signer_id()),
//                 )
//             })
//             .collect();

//         write!(f, "\nPbftLog:\n{}", string_infos.join("\n"))
//     }
// }

impl PbftLog {
    /// Create a new, empty `PbftLog` with the `max_log_size` specified in the `config`
    pub fn new(config: &PbftConfig) -> Self {
        PbftLog {
            unvalidated_blocks: HashMap::new(),
            blocks: HashSet::new(),
            messages: HashSet::new(),
            max_log_size: config.max_log_size,
        }
    }

    /// Add an already validated `Block` to the log
    pub fn add_validated_block(&mut self, block: Block) {
        trace!("Adding validated block to log: {:?}", block);
        self.blocks.insert(block);
    }

    /// Add an unvalidated `Block` to the log
    pub fn add_unvalidated_block(&mut self, block: Block) {
        trace!("Adding unvalidated block to log: {:?}", block);
        self.unvalidated_blocks
            .insert(block.block_id.clone(), block);
    }

    /// Move the `Block` corresponding to `block_id` from `unvalidated_blocks` to `blocks`. Return
    /// the block itself to be used by the calling code.
    pub fn block_validated(&mut self, block_id: BlockId) -> Option<Block> {
        trace!("Marking block as validated: {:?}", block_id);
        self.unvalidated_blocks.remove(&block_id).map(|block| {
            self.blocks.insert(block.clone());
            block
        })
    }

    /// Drop the `Block` corresponding to `block_id` from `unvalidated_blocks`.
    pub fn block_invalidated(&mut self, block_id: BlockId) -> bool {
        trace!("Dropping invalidated block: {:?}", block_id);
        self.unvalidated_blocks.remove(&block_id).is_some()
    }

    /// Get all `Block`s in the message log with the specified block number
    pub fn get_blocks_with_num(&self, block_num: u64) -> Vec<&Block> {
        self.blocks
            .iter()
            .filter(|block| block.block_num == block_num)
            .collect()
    }

    /// Get the `Block` with the specified block ID
    pub fn get_block_with_id(&self, block_id: &[u8]) -> Option<&Block> {
        self.blocks
            .iter()
            .find(|block| block.block_id.as_slice() == block_id)
    }

    /// Get the `Block` with the specified block ID from `unvalidated_blocks`.
    pub fn get_unvalidated_block_with_id(&self, block_id: &[u8]) -> Option<&Block> {
        self.unvalidated_blocks.get(block_id)
    }

    /// Add a parsed PBFT message to the log
    pub fn add_message(&mut self, msg: PbftMessage) {
        trace!("Adding message to log: {:?}", msg);
        self.messages.insert(msg);
    }

    /// Check if the log has a PrePrepare at the given view and sequence number that matches the
    /// given block ID
    pub fn has_pre_prepare(&self, seq_num: u64, view: u64, block_id: &[u8]) -> bool {
        self.get_messages_of_type_seq_view(PbftMessageType::PrePrepare, seq_num, view)
            .iter()
            .any(|msg| msg.get_block_id() == block_id)
    }

    /// Obtain all messages from the log that match the given type and sequence_number
    pub fn get_messages_of_type_seq(
        &self,
        msg_type: PbftMessageType,
        sequence_number: u64,
    ) -> Vec<&PbftMessage> {
        self.messages
            .iter()
            .filter(|&msg| {
                // let info = (*msg).info();
                msg.get_msg_type() == msg_type
                    && msg.get_seq_num() == sequence_number
            })
            .collect()
    }

    /// Obtain all messages from the log that match the given type and view
    pub fn get_messages_of_type_view(
        &self,
        msg_type: PbftMessageType,
        view: u64,
    ) -> Vec<&PbftMessage> {
        self.messages
            .iter()
            .filter(|&msg| {
                // let info = (*msg).info();
                msg.get_msg_type() == msg_type && msg.get_view() == view
            })
            .collect()
    }

    /// Obtain all messages from the log that match the given type, sequence number, and view
    pub fn get_messages_of_type_seq_view(
        &self,
        msg_type: PbftMessageType,
        sequence_number: u64,
        view: u64,
    ) -> Vec<&PbftMessage> {
        self.messages
            .iter()
            .filter(|&msg| {
                // let info = (*msg).info();
                msg.get_msg_type() == msg_type
                    && msg.get_seq_num() == sequence_number
                    && msg.get_view() == view
            })
            .collect()
    }

    /// Obtain all messages from the log that match the given type, sequence number, view, and
    /// block_id
    pub fn get_messages_of_type_seq_view_block(
        &self,
        msg_type: PbftMessageType,
        sequence_number: u64,
        view: u64,
        block_id: &[u8],
    ) -> Vec<&PbftMessage> {
        self.messages
            .iter()
            .filter(|&msg| {
                // let info = (*msg).info();
                let msg_block_id = msg.get_block_id();
                msg.get_msg_type() == msg_type
                    && msg.get_seq_num() == sequence_number
                    && msg.get_view() == view
                    && msg_block_id == block_id
            })
            .collect()
    }

    /// Garbage collect the log if it has reached the `max_log_size`
    #[allow(clippy::ptr_arg)]
    pub fn garbage_collect(&mut self, current_seq_num: u64) {
        // If the max log size has been reached, filter out all old messages
        if self.messages.len() as u64 >= self.max_log_size {
            // The node needs to keep messages from the previous sequence number in case it
            // needs to build the next consensus seal
            self.messages
                .retain(|msg| msg.get_seq_num() >= current_seq_num - 1);

            self.blocks
                .retain(|block| block.block_num >= current_seq_num - 1);
        }
    }

    #[cfg(test)]
    pub fn set_max_log_size(&mut self, size: u64) {
        self.max_log_size = size;
    }
}

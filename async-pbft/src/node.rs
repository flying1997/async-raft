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

//! The core PBFT algorithm

use std::collections::HashSet;
use std::convert::From;
use std::sync::Arc;

use rl_logger::{debug, error, info, trace, warn};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use types::pbft::consensus::{Block, BlockId, PbftMessage, PbftMessageType, PeerId, PeerInfo, is_in_view};
use common_trait::pbft::service::Service;
use crypto::secp256k1::{create_context, secp256k1::Secp256k1PublicKey};

use crate::config::{get_members_from_settings, PbftConfig};
use crate::error::PbftError;
use crate::hash::verify_sha512;

use crate::message_log::PbftLog;
use crate::pbft_sender::PbftNetwork;
use crate::state::{PbftMode, PbftPhase, PbftState};
use crate::timing::{retry_until_ok, Timeout};

/// Contains the core logic of the PBFT node
pub struct PbftNode<N: PbftNetwork> {
    /// Used for interactions with the validator
    pub service: Arc<N>,
    pub state: PbftState,
    /// Log of messages this node has received and accepted
    pub msg_log: PbftLog,
    pub events: UnboundedReceiver<PbftMessage>, //
    pub self_tx: UnboundedSender<PbftMessage>,
}

impl<N:PbftNetwork> PbftNode<N> {
    /// start pbftnode
    pub fn start(peer_id: PeerId, config: &PbftConfig,
        service: Arc<N>, self_tx: UnboundedSender<PbftMessage>, events: UnboundedReceiver<PbftMessage>) -> JoinHandle<()>{
        let state = PbftState::new(peer_id, 0, config);
        let node = PbftNode{
            service,
            state,
            msg_log:PbftLog::new(config),
            events,
            self_tx,
        };
        tokio::spawn(node.main())
    }
    //the main event loop
    pub async fn main(mut self){

        loop {
            let incoming_message = self.events.recv().await.unwrap();
    
            self.on_peer_message(incoming_message).await;
                
    
            // If the block publishing delay has passed, attempt to publish a block
            // block_publishing_ticker.tick(|| log_any_error(node.try_publish(state)));
    
            // If the idle timeout has expired, initiate a view change
            if self.check_idle_timeout_expired() {
                warn!("Idle timeout expired; proposing view change");
                self.start_view_change(self.state.view + 1);
            }
    
            // If the commit timeout has expired, initiate a view change
            if self.check_commit_timeout_expired() {
                warn!("Commit timeout expired; proposing view change");
                self.start_view_change(self.state.view + 1);
            }
    
            // Check the view change timeout if the node is view changing so we can start a new
            // view change if we don't get a NewView in time
            if let PbftMode::ViewChanging(v) = self.state.mode {
                if self.check_view_change_timeout_expired() {
                    warn!(
                        "View change timeout expired; proposing view change for view {}",
                        v + 1
                   );
                   self.start_view_change(v + 1);
                }
            }
        }
    }
    // ---------- Methods for handling Updates from the Validator ----------

    /// Handle a peer message from another PbftNode
    ///
    /// Handle all messages from other nodes. Such messages include `PrePrepare`, `Prepare`,
    /// `Commit`, `ViewChange`, and `NewView`. Make sure the message is from a PBFT member. If the
    /// node is view changing, ignore all messages that aren't `ViewChange`s or `NewView`s.
    pub async fn on_peer_message(
        &mut self,
        msg: PbftMessage,
    ) -> Result<(), PbftError> {
        // trace!("{}: Got peer message: {}", state, msg.info());

        // Make sure this message is from a known member of the PBFT network
        if !self.state.member_ids.contains(&msg.get_signer_id()) {
            return Err(PbftError::InvalidMessage(format!(
                "Received message from node ({:?}) that is not a member of the PBFT network",
                hex::encode(msg.get_signer_id()),
            )));
        }

        let msg_type = msg.get_msg_type();

        // If this node is in the process of a view change, ignore all messages except ViewChanges
        // and NewViews
        let view_ = is_in_view(&msg_type);
        if matches!(self.state.mode, PbftMode::ViewChanging(_))
            && !view_
        {
            debug!(
                "{}: Node is view changing; ignoring {:?} message",
                self.state, msg_type
            );
            return Ok(());
        }

        match msg_type {
            PbftMessageType::PrePrepare => self.handle_pre_prepare(msg)?,
            PbftMessageType::Prepare => self.handle_prepare(msg)?,
            PbftMessageType::Commit => self.handle_commit(msg)?,
            PbftMessageType::ViewChange => self.handle_view_change(&msg)?,
            PbftMessageType::NewView => self.handle_new_view(&msg)?,
            
            
            _ => warn!("Received message with unknown type: {:?}", msg_type),
        }

        Ok(())
    }

    /// Handle a `PrePrepare` message
    ///
    /// A `PrePrepare` message is accepted and added to the log if the following are true:
    /// - The message signature is valid (already verified by validator)
    /// - The message is from the primary
    /// - The message's view matches the node's current view
    /// - A `PrePrepare` message does not already exist at this view and sequence number with a
    ///   different block
    ///
    /// Once a `PrePrepare` for the current sequence number is accepted and added to the log, the
    /// node will try to switch to the `Preparing` phase.
    fn handle_pre_prepare(
        &mut self,
        msg: PbftMessage,
    ) -> Result<(), PbftError> {
        // Check that the message is from the current primary
        if msg.get_signer_id() != self.state.get_primary_id() {
            warn!(
                "Got PrePrepare from a secondary node {:?}; ignoring message",
                msg.get_signer_id()
            );
            return Ok(());
        }

        // Check that the message is for the current view
        if msg.get_view() != self.state.view {
            return Err(PbftError::InvalidMessage(format!(
                "Node is on view {}, but a PrePrepare for view {} was received",
                self.state.view,
                msg.get_view(),
            )));
        }

        // Check that no `PrePrepare`s already exist with this view and sequence number but a
        // different block; if this is violated, the primary is faulty so initiate a view change
        let mismatched_blocks = self
            .msg_log
            .get_messages_of_type_seq_view(
                PbftMessageType::PrePrepare,
                msg.get_seq_num(),
                msg.get_view(),
            )
            .iter()
            .filter_map(|existing_msg| {
                let block_id = existing_msg.get_block_id();
                if block_id != msg.get_block_id() {
                    Some(block_id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if !mismatched_blocks.is_empty() {
            self.start_view_change(self.state.view + 1)?;
            return Err(PbftError::FaultyPrimary(format!(
                "When checking PrePrepare with block {:?}, found PrePrepare(s) with same view and \
                 seq num but mismatched block(s): {:?}",
                hex::encode(&msg.get_block_id()),
                mismatched_blocks,
            )));
        }


        // Add message to the log
        self.msg_log.add_message(msg.clone());

        // If the node is in the PrePreparing phase, this message is for the current sequence
        // number, and the node already has this block: switch to Preparing
        self.try_preparing(msg.get_block_id())
    }

    /// Handle a `Prepare` message
    ///
    /// Once a `Prepare` for the current sequence number is accepted and added to the log, the node
    /// will check if it has the required 2f + 1 `Prepared` messages to move on to the Committing
    /// phase
    fn handle_prepare(
        &mut self,
        msg: PbftMessage,
    ) -> Result<(), PbftError> {
        let block_id = msg.get_block_id();

        // Check that the message is for the current view
        if msg.get_view() != self.state.view {
            return Err(PbftError::InvalidMessage(format!(
                "Node is on view {}, but a Prepare for view {} was received",
                self.state.view,
                msg.get_view(),
            )));
        }

        // The primary is not allowed to send a Prepare; its PrePrepare counts as its "vote"
        if msg.get_signer_id() == self.state.get_primary_id() {
            self.start_view_change(self.state.view + 1)?;
            return Err(PbftError::FaultyPrimary(format!(
                "Received Prepare from primary at view {}, seq_num {}",
                self.state.view, self.state.seq_num
            )));
        }

        self.msg_log.add_message(msg.clone());

        // If this message is for the current sequence number and the node is in the Preparing
        // phase, check if the node is ready to move on to the Committing phase
        if msg.get_seq_num() == self.state.seq_num && self.state.phase == PbftPhase::Preparing {
            // The node is ready to move on to the Committing phase (i.e. the predicate `prepared`
            // is true) when its log has 2f + 1 Prepare messages from different nodes that match
            // the PrePrepare message received earlier (same view, sequence number, and block)
            let has_matching_pre_prepare =
                self.msg_log
                    .has_pre_prepare(msg.get_seq_num(), msg.get_view(), &block_id);
            let has_required_prepares = self
                .msg_log
                // Only get Prepares with matching seq_num, view, and block_id
                .get_messages_of_type_seq_view_block(
                    PbftMessageType::Prepare,
                    msg.get_seq_num(),
                    msg.get_view(),
                    &block_id,
                )
                // Check if there are at least 2f + 1 Prepares
                .len() as u64
                > 2 * self.state.f;
            if has_matching_pre_prepare && has_required_prepares {
                self.state.switch_phase(PbftPhase::Committing)?;
                self.broadcast_pbft_message(
                    self.state.view,
                    self.state.seq_num,
                    PbftMessageType::Commit,
                    block_id,
                )?;
            }
        }

        Ok(())
    }

    /// Handle a `Commit` message
    ///
    /// Once a `Commit` for the current sequence number is accepted and added to the log, the node
    /// will check if it has the required 2f + 1 `Commit` messages to actually commit the block
    fn handle_commit(
        &mut self,
        msg: PbftMessage,
    ) -> Result<(), PbftError> {
        let block_id = msg.get_block_id();

        // Check that the message is for the current view
        if msg.get_view() != self.state.view {
            return Err(PbftError::InvalidMessage(format!(
                "Node is on view {}, but a Commit for view {} was received",
                self.state.view,
                msg.get_view(),
            )));
        }

        self.msg_log.add_message(msg.clone());

        // If this message is for the current sequence number and the node is in the Committing
        // phase, check if the node is ready to commit the block
        if msg.get_seq_num() == self.state.seq_num && self.state.phase == PbftPhase::Committing {
            // The node is ready to commit the block (i.e. the predicate `committable` is true)
            // when its log has 2f + 1 Commit messages from different nodes that match the
            // PrePrepare message received earlier (same view, sequence number, and block)
            let has_matching_pre_prepare =
                self.msg_log
                    .has_pre_prepare(msg.get_seq_num(), msg.get_view(), &block_id);
            let has_required_commits = self
                .msg_log
                // Only get Commits with matching seq_num, view, and block_id
                .get_messages_of_type_seq_view_block(
                    PbftMessageType::Commit,
                    msg.get_seq_num(),
                    msg.get_view(),
                    &block_id,
                )
                // Check if there are at least 2f + 1 Commits
                .len() as u64
                > 2 * self.state.f;
            if has_matching_pre_prepare && has_required_commits {
                // todo! commit block
                // self.service.commit_block(block_id.clone()).map_err(|err| {
                //     PbftError::ServiceError(
                //         format!("Failed to commit block {:?}", hex::encode(&block_id)),
                //         err,
                //     )
                // })?;
                self.state.switch_phase(PbftPhase::Finishing(false))?;
                // Stop the commit timeout, since the network has agreed to commit the block
                self.state.commit_timeout.stop();

                //todo!  pull a new block to start a new consensus
            }
        }

        Ok(())
    }

    /// Handle a `ViewChange` message
    ///
    /// When a `ViewChange` is received, check that it isn't outdated and add it to the log. If the
    /// node isn't already view changing but it now has f + 1 ViewChange messages, start view
    /// changing early. If the node is the primary and has 2f view change messages now, broadcast
    /// the NewView message to the rest of the nodes to move to the new view.
    fn handle_view_change(
        &mut self,
        msg: &PbftMessage,
    ) -> Result<(), PbftError> {
        // Ignore old view change messages (already on a view >= the one this message is
        // for or already trying to change to a later view)
        let msg_view = msg.get_view();
        if msg_view <= self.state.view
            || match self.state.mode {
                PbftMode::ViewChanging(v) => msg_view < v,
                _ => false,
            }
        {
            debug!("Ignoring stale view change message for view {}", msg_view);
            return Ok(());
        }

        self.msg_log.add_message(msg.clone());

        // Even if the node hasn't detected a faulty primary yet, start view changing if there are
        // f + 1 ViewChange messages in the log for this proposed view (but if already view
        // changing, only do this for a later view); this will prevent starting the view change too
        // late
        let is_later_view = match self.state.mode {
            PbftMode::ViewChanging(v) => msg_view > v,
            PbftMode::Normal => true,
        };
        let start_view_change = self
            .msg_log
            // Only get ViewChanges with matching view
            .get_messages_of_type_view(PbftMessageType::ViewChange, msg_view)
            // Check if there are at least f + 1 ViewChanges
            .len() as u64
            > self.state.f;
        if is_later_view && start_view_change {
            info!(
                "{}: Received f + 1 ViewChange messages; starting early view change",
                self.state
            );
            // Can exit early since the node will self-send another ViewChange message here
            return self.start_view_change(msg_view);
        }

        let messages = self
            .msg_log
            .get_messages_of_type_view(PbftMessageType::ViewChange, msg_view);

        // If there are 2f + 1 ViewChange messages and the view change timeout is not already
        // started, update the timeout and start it
        if !self.state.view_change_timeout.is_active() && messages.len() as u64 > self.state.f * 2 {
            self.state.view_change_timeout = Timeout::new(
                self.state
                    .view_change_duration
                    .checked_mul((msg_view - self.state.view) as u32)
                    .expect("View change timeout has overflowed"),
            );
            self.state.view_change_timeout.start();
        }

        // If this node is the new primary and the required 2f ViewChange messages (not including
        // the primary's own) are present in the log, broadcast the NewView message
        let messages_from_other_nodes = messages
            .iter()
            .filter(|msg| msg.get_signer_id() != self.state.id)
            .cloned()
            .collect::<Vec<_>>();

        if self.state.is_primary_at_view(msg_view)
            && messages_from_other_nodes.len() as u64 >= 2 * self.state.f
        {
            // let mut new_view = PbftNewView::new();

            // new_view.set_info(PbftMessageInfo::new_from(
            //     PbftMessageType::NewView,
            //     msg_view,
            //     state.seq_num - 1,
            //     state.id.clone(),
            // ));

            // new_view.set_view_changes(Self::signed_votes_from_messages(
            //     messages_from_other_nodes.as_slice(),
            // ));
            let msg = PbftMessage::new_view(msg_view, self.state.seq_num-1, self.state.id.clone(), bcs::to_bytes(&messages_from_other_nodes).unwrap());
            
            // trace!("Created NewView message: {:?}", new_view);

            self.broadcast_message(msg)?;
        }

        Ok(())
    }

    /// Handle a `NewView` message
    ///
    /// When a `NewView` is received, verify that it is valid; if it is, update the view and the
    /// node's state.
    fn handle_new_view(
        &mut self,
        msg: &PbftMessage,
    ) -> Result<(), PbftError> {
        // let new_view = msg.get_new_view_message();

        match self.verify_new_view(msg) {
            Ok(_) => trace!("NewView passed verification"),
            Err(err) => {
                return Err(PbftError::InvalidMessage(format!(
                    "NewView failed verification - Error was: {}",
                    err
                )));
            }
        }

        // If this node was the primary before, cancel any block that may have been initialized
        if self.state.is_primary() {
            // self.service.cancel_block().unwrap_or_else(|err| {
            //     info!("Failed to cancel block when becoming secondary: {:?}", err);
            // });
        }

        // Update view
        self.state.view = msg.get_view();
        self.state.view_change_timeout.stop();

        info!("{}: Updated to view {}", self.state, self.state.view);

        // Reset state to Normal mode, reset the phase (unless waiting for a BlockCommit) and
        // restart the idle timeout
        self.state.mode = PbftMode::Normal;
        if !matches!(self.state.phase, PbftPhase::Finishing(_)) {
            self.state.phase = PbftPhase::PrePreparing;
        }
        self.state.idle_timeout.start();

        // Initialize a new block if this node is the new primary
        if self.state.is_primary() {
            // self.service.initialize_block(None).map_err(|err| {
            //     PbftError::ServiceError("Couldn't initialize block after view change".into(), err)
            // })?;
        }

        Ok(())
    }

    /// Handle a `SealRequest` message
    ///
    /// A node is requesting a consensus seal for the last block. If the block was the last one
    /// committed by this node, build the seal and send it to the requesting node; if the block has
    /// not been committed yet but it's the next one to be committed, add the request to the log
    /// and the node will build/send the seal when it's done committing. If this is an older block
    /// (state.seq_num > msg.seq_num + 1) or this node is behind (state.seq_num < msg.seq_num), the
    /// node will not be able to build the requseted seal, so just ignore the message.
    // fn handle_seal_request(
    //     &mut self,
    //     msg: ParsedMessage,
    //     state: &mut PbftState,
    // ) -> Result<(), PbftError> {
    //     if state.seq_num == msg.info().get_seq_num() + 1 {
    //         return self.send_seal_response(state, &msg.info().get_signer_id().to_vec());
    //     } else if state.seq_num == msg.info().get_seq_num() {
    //         self.msg_log.add_message(msg);
    //     }
    //     Ok(())
    // }

    /// Handle a `Seal` message
    ///
    /// A node has responded to the seal request by sending a seal for the last block; validate the
    /// seal and commit the block.
    // fn handle_seal_response(
    //     &mut self,
    //     msg: &ParsedMessage,
    //     state: &mut PbftState,
    // ) -> Result<(), PbftError> {
    //     let seal = msg.get_seal();

    //     // If the node has already committed the block, ignore
    //     if let PbftPhase::Finishing(_) = state.phase {
    //         return Ok(());
    //     }

    //     // Get the previous ID of the block this seal is for so it can be used to verify the seal
    //     let previous_id = self
    //         .msg_log
    //         .get_block_with_id(seal.block_id.as_slice())
    //         // Make sure the node actually has the block
    //         .ok_or_else(|| {
    //             PbftError::InvalidMessage(format!(
    //                 "Received a seal for a block ({:?}) that the node does not have",
    //                 hex::encode(&seal.block_id),
    //             ))
    //         })
    //         .and_then(|block| {
    //             // Make sure the block is at the node's current sequence number
    //             if block.block_num != state.seq_num {
    //                 Err(PbftError::InvalidMessage(format!(
    //                     "Received a seal for block {:?}, but block_num does not match node's \
    //                      seq_num: {} != {}",
    //                     hex::encode(&seal.block_id),
    //                     block.block_num,
    //                     state.seq_num,
    //                 )))
    //             } else {
    //                 Ok(block.previous_id.clone())
    //             }
    //         })?;

    //     // Verify the seal
    //     match self.verify_consensus_seal(seal, previous_id, state) {
    //         Ok(_) => {
    //             trace!("Consensus seal passed verification");
    //         }
    //         Err(err) => {
    //             return Err(PbftError::InvalidMessage(format!(
    //                 "Consensus seal failed verification - Error was: {}",
    //                 err
    //             )));
    //         }
    //     }

    //     // Catch up
    //     self.catchup(state, seal, false)
    // }

    /// Handle a `BlockNew` update from the Validator
    ///
    /// The validator has received a new block; check if it is a block that should be considered,
    /// add it to the log as an unvalidated block, and instruct the validator to validate it.
    // pub fn on_block_new(&mut self, block: Block, state: &mut PbftState) -> Result<(), PbftError> {
    //     info!(
    //         "{}: Got BlockNew: {} / {}",
    //         state,
    //         block.block_num,
    //         hex::encode(&block.block_id)
    //     );
    //     trace!("Block details: {:?}", block);

    //     // Only future blocks should be considered since committed blocks are final
    //     if block.block_num < state.seq_num {
    //         self.service
    //             .fail_block(block.block_id.clone())
    //             .unwrap_or_else(|err| error!("Couldn't fail block due to error: {:?}", err));
    //         return Err(PbftError::InternalError(format!(
    //             "Received block {:?} / {:?} that is older than the current sequence number: {:?}",
    //             block.block_num,
    //             hex::encode(&block.block_id),
    //             state.seq_num,
    //         )));
    //     }

    //     // Make sure the node already has the previous block, since the consensus seal can't be
    //     // verified without it
    //     let previous_block = self
    //         .msg_log
    //         .get_block_with_id(block.previous_id.as_slice())
    //         .or_else(|| {
    //             self.msg_log
    //                 .get_unvalidated_block_with_id(block.previous_id.as_slice())
    //         });
    //     if previous_block.is_none() {
    //         self.service
    //             .fail_block(block.block_id.clone())
    //             .unwrap_or_else(|err| error!("Couldn't fail block due to error: {:?}", err));
    //         return Err(PbftError::InternalError(format!(
    //             "Received block {:?} / {:?} but node does not have previous block {:?}",
    //             block.block_num,
    //             hex::encode(&block.block_id),
    //             hex::encode(&block.previous_id),
    //         )));
    //     }

    //     // Make sure that the previous block has the previous block number (enforces that blocks
    //     // are strictly monotically increasing by 1)
    //     let previous_block = previous_block.expect("Previous block's existence already checked");
    //     if previous_block.block_num != block.block_num - 1 {
    //         self.service
    //             .fail_block(block.block_id.clone())
    //             .unwrap_or_else(|err| error!("Couldn't fail block due to error: {:?}", err));
    //         return Err(PbftError::InternalError(format!(
    //             "Received block {:?} / {:?} but its previous block ({:?} / {:?}) does not have \
    //              the previous block_num",
    //             block.block_num,
    //             hex::encode(&block.block_id),
    //             block.block_num - 1,
    //             hex::encode(&block.previous_id),
    //         )));
    //     }

    //     // Add the currently unvalidated block to the log
    //     self.msg_log.add_unvalidated_block(block.clone());

    //     // Have the validator check the block
    //     self.service
    //         .check_blocks(vec![block.block_id.clone()])
    //         .map_err(|err| {
    //             PbftError::ServiceError(
    //                 format!(
    //                     "Failed to check block {:?} / {:?}",
    //                     block.block_num,
    //                     hex::encode(&block.block_id),
    //                 ),
    //                 err,
    //             )
    //         })?;

    //     Ok(())
    // }

    /// Handle a `BlockValid` update from the Validator
    ///
    /// The block has been verified by the validator, so mark it as validated in the log and
    /// attempt to handle the block.
    // pub fn on_block_valid(
    //     &mut self,
    //     block_id: BlockId,
    //     state: &mut PbftState,
    // ) -> Result<(), PbftError> {
    //     info!("Got BlockValid: {}", hex::encode(&block_id));

    //     // Mark block as validated in the log and get the block
    //     let block = self
    //         .msg_log
    //         .block_validated(block_id.clone())
    //         .ok_or_else(|| {
    //             PbftError::InvalidMessage(format!(
    //                 "Received BlockValid message for an unknown block: {}",
    //                 hex::encode(&block_id)
    //             ))
    //         })?;

    //     self.try_handling_block(block, state)
    // }

    /// Validate the block's seal and handle the block. If this is the block the node is waiting
    /// for and this node is the primary, broadcast a PrePrepare; if the node isn't the primary but
    /// it already has the PrePrepare for this block, switch to `Preparing`. If this is a future
    /// block, use it to catch up.
    // fn try_handling_block(&mut self, block: Block, state: &mut PbftState) -> Result<(), PbftError> {
    //     // If the block's number is higher than the current sequence number + 1 (i.e., it is newer
    //     // than the grandchild of the last committed block), the seal cannot be verified; this is
    //     // because the settings in a block's grandparent are needed to verify the block's seal, and
    //     // these settings are only guaranteed to be in the validator's state when the block is
    //     // committed. If this is a newer block, wait until after the grandparent is committed
    //     // before validating the seal and handling the block.
    //     if block.block_num > state.seq_num + 1 {
    //         return Ok(());
    //     }

    //     let seal = self
    //         .verify_consensus_seal_from_block(&block, state)
    //         .map_err(|err| {
    //             self.service
    //                 .fail_block(block.block_id.clone())
    //                 .unwrap_or_else(|err| error!("Couldn't fail block due to error: {:?}", err));
    //             PbftError::InvalidMessage(format!(
    //                 "Consensus seal failed verification - Error was: {}",
    //                 err
    //             ))
    //         })?;

    //     // This block's seal can be used to commit the block previous to it (i.e. catch-up) if it's
    //     // a future block and the node isn't waiting for a commit message for a previous block (if
    //     // it is waiting for a commit message, catch-up will have to be done after the message is
    //     // received)
    //     let is_waiting = matches!(state.phase, PbftPhase::Finishing(_));
    //     if block.block_num > state.seq_num && !is_waiting {
    //         self.catchup(state, &seal, true)?;
    //     } else if block.block_num == state.seq_num {
    //         if block.signer_id == state.id && state.is_primary() {
    //             // This is the next block and this node is the primary; broadcast PrePrepare
    //             // messages
    //             info!("Broadcasting PrePrepares");
    //             self.broadcast_pbft_message(
    //                 state.view,
    //                 state.seq_num,
    //                 PbftMessageType::PrePrepare,
    //                 block.block_id,
    //                 state,
    //             )?;
    //         } else {
    //             // If the node is in the PrePreparing phase and it already has a PrePrepare for
    //             // this block: switch to Preparing
    //             self.try_preparing(block.block_id, state)?;
    //         }
    //     }

    //     Ok(())
    // }

    /// Handle a `BlockInvalid` update from the Validator
    ///
    /// The block is invalid, so drop it from the log and fail it.
    // pub fn on_block_invalid(&mut self, block_id: BlockId) -> Result<(), PbftError> {
    //     info!("Got BlockInvalid: {}", hex::encode(&block_id));

    //     // Drop block from the log
    //     if !self.msg_log.block_invalidated(block_id.clone()) {
    //         return Err(PbftError::InvalidMessage(format!(
    //             "Received BlockInvalid message for an unknown block: {}",
    //             hex::encode(&block_id)
    //         )));
    //     }

    //     // Fail the block
    //     self.service
    //         .fail_block(block_id)
    //         .unwrap_or_else(|err| error!("Couldn't fail block due to error: {:?}", err));

    //     Ok(())
    // }

    /// Use the given consensus seal to verify and commit the block this node is working on
    // fn catchup(
    //     &mut self,
    //     state: &mut PbftState,
    //     seal: &PbftSeal,
    //     catchup_again: bool,
    // ) -> Result<(), PbftError> {
    //     info!(
    //         "{}: Attempting to commit block {} using catch-up",
    //         state, state.seq_num
    //     );

    //     let messages = seal
    //         .get_commit_votes()
    //         .iter()
    //         .try_fold(Vec::new(), |mut msgs, vote| {
    //             msgs.push(ParsedMessage::from_signed_vote(vote)?);
    //             Ok(msgs)
    //         })?;

    //     // Update view if necessary
    //     let view = messages[0].info().get_view();
    //     if view != state.view {
    //         info!("Updating view from {} to {}", state.view, view);
    //         state.view = view;
    //     }

    //     // Add messages to the log
    //     for message in &messages {
    //         self.msg_log.add_message(message.clone());
    //     }

    //     // Commit the block, stop the idle timeout, and skip straight to Finishing
    //     self.service
    //         .commit_block(seal.block_id.clone())
    //         .map_err(|err| {
    //             PbftError::ServiceError(
    //                 format!(
    //                     "Failed to commit block with catch-up {:?} / {:?}",
    //                     state.seq_num,
    //                     hex::encode(&seal.block_id)
    //                 ),
    //                 err,
    //             )
    //         })?;
    //     state.idle_timeout.stop();
    //     state.phase = PbftPhase::Finishing(catchup_again);

    //     Ok(())
    // }

    /// Handle a `BlockCommit` update from the Validator
    ///
    /// A block was sucessfully committed; clean up any uncommitted blocks, update state to be
    /// ready for the next block, make any necessary view and membership changes, garbage collect
    /// the logs, and start a new block if this node is the primary.
    // pub fn on_block_commit(
    //     &mut self,
    //     block_id: BlockId,
    //     state: &mut PbftState,
    // ) -> Result<(), PbftError> {
    //     info!("{}: Got BlockCommit for {}", state, hex::encode(&block_id));

    //     let is_catching_up = matches!(state.phase, PbftPhase::Finishing(true));

    //     // If there are any blocks in the log at this sequence number other than the one that was
    //     // just committed, reject them
    //     let invalid_block_ids = self
    //         .msg_log
    //         .get_blocks_with_num(state.seq_num)
    //         .iter()
    //         .filter_map(|block| {
    //             if block.block_id != block_id {
    //                 Some(block.block_id.clone())
    //             } else {
    //                 None
    //             }
    //         })
    //         .collect::<Vec<_>>();

    //     for id in invalid_block_ids {
    //         self.service.fail_block(id.clone()).unwrap_or_else(|err| {
    //             error!(
    //                 "Couldn't fail block {:?} due to error: {:?}",
    //                 &hex::encode(id),
    //                 err
    //             )
    //         });
    //     }

    //     // Increment sequence number and update state
    //     state.seq_num += 1;
    //     state.mode = PbftMode::Normal;
    //     state.phase = PbftPhase::PrePreparing;
    //     state.chain_head = block_id.clone();

    //     // If node(s) are waiting for a seal to commit the last block, send it now
    //     let requesters = self
    //         .msg_log
    //         .get_messages_of_type_seq(PbftMessageType::SealRequest, state.seq_num - 1)
    //         .iter()
    //         .map(|req| req.info().get_signer_id().to_vec())
    //         .collect::<Vec<_>>();

    //     for req in requesters {
    //         self.send_seal_response(state, &req).unwrap_or_else(|err| {
    //             error!("Failed to send seal response due to: {:?}", err);
    //         });
    //     }

    //     // Update membership if necessary
    //     self.update_membership(block_id.clone(), state);

    //     // Increment the view if a view change must be forced for fairness
    //     if state.at_forced_view_change() {
    //         state.view += 1;
    //     }

    //     // Tell the log to garbage collect if it needs to
    //     self.msg_log.garbage_collect(state.seq_num);

    //     // If the node already has grandchild(ren) of the block that was just committed, one of
    //     // them may be used to perform catch-up to commit the next block.
    //     let grandchildren = self
    //         .msg_log
    //         .get_blocks_with_num(state.seq_num + 1)
    //         .iter()
    //         .cloned()
    //         .cloned()
    //         .collect::<Vec<_>>();
    //     for block in grandchildren {
    //         if self.try_handling_block(block, state).is_ok() {
    //             return Ok(());
    //         }
    //     }

    //     // If the node is catching up but doesn't have a block with a seal to commit the next one,
    //     // it will need to request the seal to commit the last block. The node doesn't know which
    //     // block that the network decided to commit, so it can't request the seal for a specific
    //     // block (puts an empty BlockId in the message)
    //     if is_catching_up {
    //         info!(
    //             "{}: Requesting seal to finish catch-up to block {}",
    //             state, state.seq_num
    //         );
    //         return self.broadcast_pbft_message(
    //             state.view,
    //             state.seq_num,
    //             PbftMessageType::SealRequest,
    //             BlockId::new(),
    //             state,
    //         );
    //     }

    //     // Start the idle timeout for the next block
    //     state.idle_timeout.start();

    //     // If we already have a block at this sequence number with a valid PrePrepare for it, start
    //     // Preparing (there may be multiple blocks, but only one will have a valid PrePrepare)
    //     let block_ids = self
    //         .msg_log
    //         .get_blocks_with_num(state.seq_num)
    //         .iter()
    //         .map(|block| block.block_id.clone())
    //         .collect::<Vec<_>>();
    //     for id in block_ids {
    //         self.try_preparing(id, state)?;
    //     }

    //     // Initialize a new block if this node is the primary and it is not in the process of
    //     // catching up
    //     if state.is_primary() {
    //         info!(
    //             "{}: Initializing block on top of {}",
    //             state,
    //             hex::encode(&block_id)
    //         );
    //         self.service
    //             .initialize_block(Some(block_id))
    //             .map_err(|err| {
    //                 PbftError::ServiceError("Couldn't initialize block after commit".into(), err)
    //             })?;
    //     }

    //     Ok(())
    // }

    /// Check the on-chain list of members; if it has changed, update members list and return true.
    ///
    /// # Panics
    /// + If the `sawtooth.consensus.pbft.members` setting is unset or invalid
    /// + If the network this node is on does not have enough nodes to be Byzantine fault tolernant
    // fn update_membership(&mut self, block_id: BlockId, state: &mut PbftState) {
    //     // Get list of members from settings (retry until a valid result is received)
    //     trace!("Getting on-chain list of members to check for membership updates");
    //     let settings = retry_until_ok(
    //         state.exponential_retry_base,
    //         state.exponential_retry_max,
    //         || {
    //             self.service.get_settings(
    //                 block_id.clone(),
    //                 vec![String::from("sawtooth.consensus.pbft.members")],
    //             )
    //         },
    //     );
    //     let on_chain_members = get_members_from_settings(&settings);

    //     if on_chain_members != state.member_ids {
    //         info!("Updating membership: {:?}", on_chain_members);
    //         state.member_ids = on_chain_members;
    //         let f = (state.member_ids.len() - 1) / 3;
    //         if f == 0 {
    //             panic!("This network no longer contains enough nodes to be fault tolerant");
    //         }
    //         state.f = f as u64;
    //     }
    // }

    /// When the node has a block and a corresponding PrePrepare for its current sequence number,
    /// and it is in the PrePreparing phase, it can enter the Preparing phase and broadcast its
    /// Prepare
    fn try_preparing(&mut self, block_id: BlockId) -> Result<(), PbftError> {

        if let Some(block) = self.msg_log.get_block_with_id(&block_id) {
            if self.state.phase == PbftPhase::PrePreparing
                && self.msg_log.has_pre_prepare(self.state.seq_num, self.state.view, &block_id)
                // PrePrepare.seq_num == state.seq_num == block.block_num enforces the one-to-one
                // correlation between seq_num and block_num (PrePrepare n should be for block n)
                && block.block_num == self.state.seq_num
            {
                self.state.switch_phase(PbftPhase::Preparing)?;

                // Stop idle timeout, since a new block and valid PrePrepare were received in time
                self.state.idle_timeout.stop();

                // Now start the commit timeout in case the network fails to commit the block
                // within a reasonable amount of time
                self.state.commit_timeout.start();

                // The primary doesn't broadcast a Prepare; its PrePrepare counts as its "vote"
                if !self.state.is_primary() {
                    self.broadcast_pbft_message(
                        self.state.view,
                        self.state.seq_num,
                        PbftMessageType::Prepare,
                        block_id,
                    )?;
                }
            }
        }

        Ok(())
    }

    /// When the whole network is starting "fresh" from a non-genesis block, none of the nodes will
    /// have the `Commit` messages necessary to build the consensus seal for the last committed
    /// block (the chain head). To bootstrap the network in this scenario, all nodes will send a
    /// `Commit` message for their chain head whenever one of the PBFT members connects; when
    /// > 2f + 1 nodes have connected and received these `Commit` messages, the nodes will be able
    /// to build a seal using the messages.
    // fn broadcast_bootstrap_commit(
    //     &mut self,
    //     peer_id: PeerId,
    //     state: &mut PbftState,
    // ) -> Result<(), PbftError> {
    //     // The network must agree on a single view number for the Commit messages, so the view
    //     // of the chain head's predecessor is used. For block 1 this is view 0; otherwise, it's the
    //     // view of the block's consensus seal
    //     let view = if state.seq_num == 2 {
    //         0
    //     } else {
    //         self.msg_log
    //             .get_block_with_id(&state.chain_head)
    //             .ok_or_else(|| {
    //                 PbftError::InternalError(format!(
    //                     "Node does not have chain head ({:?}) in its log",
    //                     state.chain_head
    //                 ))
    //             })
    //             .and_then(|block| {
    //                 PbftSeal::parse_from_bytes(&block.payload).map_err(|err| {
    //                     PbftError::SerializationError(
    //                         "Error parsing seal from chain head".into(),
    //                         err,
    //                     )
    //                 })
    //             })?
    //             .get_info()
    //             .get_view()
    //     };

    //     // Construct the commit message for the chain head and send it to the connected peer
    //     let mut commit = PbftMessage::new();
    //     commit.set_info(PbftMessageInfo::new_from(
    //         PbftMessageType::Commit,
    //         view,
    //         state.seq_num - 1,
    //         state.id.clone(),
    //     ));
    //     commit.set_block_id(state.chain_head.clone());

    //     let bytes = commit.write_to_bytes().map_err(|err| {
    //         PbftError::SerializationError("Error writing commit to bytes".into(), err)
    //     })?;

    //     self.service
    //         .send_to(
    //             &peer_id,
    //             String::from(PbftMessageType::Commit).as_str(),
    //             bytes,
    //         )
    //         .map_err(|err| {
    //             PbftError::ServiceError(
    //                 format!("Failed to send Commit to {:?}", hex::encode(peer_id)),
    //                 err,
    //             )
    //         })
    // }

    // ---------- Methods for building & verifying proofs and signed messages from other nodes ----------

    /// Generate a `protobuf::RepeatedField` of signed votes from a list of parsed messages
    // fn signed_votes_from_messages(msgs: &[&ParsedMessage]) -> RepeatedField<PbftSignedVote> {
    //     RepeatedField::from(
    //         msgs.iter()
    //             .map(|m| {
    //                 let mut vote = PbftSignedVote::new();

    //                 vote.set_header_bytes(m.header_bytes.clone());
    //                 vote.set_header_signature(m.header_signature.clone());
    //                 vote.set_message_bytes(m.message_bytes.clone());

    //                 vote
    //             })
    //             .collect::<Vec<_>>(),
    //     )
    // }

    /// Build a consensus seal that proves the last block committed by this node
    // fn build_seal(&self, state: &PbftState) -> Result<PbftSeal, PbftError> {
    //     trace!("{}: Building seal for block {}", state, state.seq_num - 1);

    //     // The previous block may have been committed in a different view, so the node will need to
    //     // find the view that contains the required 2f Commit messages for building the seal
    //     let (block_id, view, messages) = self
    //         .msg_log
    //         .get_messages_of_type_seq(PbftMessageType::Commit, state.seq_num - 1)
    //         .iter()
    //         // Filter out this node's own messages because self-sent messages aren't signed and
    //         // therefore can't be included in the seal
    //         .filter(|msg| !msg.from_self)
    //         .cloned()
    //         // Map to ((block_id, view), msg)
    //         .map(|msg| ((msg.get_block_id(), msg.info().get_view()), msg))
    //         // Group messages together by block and view
    //         .into_group_map()
    //         .into_iter()
    //         // One and only one block/view should have the required number of messages, since only
    //         // one block at this sequence number should have been committed and in only one view
    //         .find_map(|((block_id, view), msgs)| {
    //             if msgs.len() as u64 >= 2 * state.f {
    //                 Some((block_id, view, msgs))
    //             } else {
    //                 None
    //             }
    //         })
    //         .ok_or_else(|| {
    //             PbftError::InternalError(String::from(
    //                 "Couldn't find 2f commit messages in the message log for building a seal",
    //             ))
    //         })?;

    //     let mut seal = PbftSeal::new();
    //     seal.set_info(PbftMessageInfo::new_from(
    //         PbftMessageType::Seal,
    //         view,
    //         state.seq_num - 1,
    //         state.id.clone(),
    //     ));
    //     seal.set_block_id(block_id);
    //     seal.set_commit_votes(Self::signed_votes_from_messages(messages.as_slice()));

    //     trace!("Seal created: {:?}", seal);

    //     Ok(seal)
    // }

    /// Verify that a vote matches the expected type, is properly signed, and passes the specified
    /// criteria; if it passes verification, return the signer ID to be used for further
    /// verification
    // fn verify_vote<F>(
    //     vote: &PbftSignedVote,
    //     expected_type: PbftMessageType,
    //     validation_criteria: F,
    // ) -> Result<PeerId, PbftError>
    // where
    //     F: Fn(&PbftMessage) -> Result<(), PbftError>,
    // {
    //     // Parse the message
    //     let pbft_message: PbftMessage = Message::parse_from_bytes(vote.get_message_bytes())
    //         .map_err(|err| {
    //             PbftError::SerializationError("Error parsing PbftMessage from vote".into(), err)
    //         })?;
    //     let header: ConsensusPeerMessageHeader = Message::parse_from_bytes(vote.get_header_bytes())
    //         .map_err(|err| {
    //             PbftError::SerializationError("Error parsing header from vote".into(), err)
    //         })?;

    //     trace!(
    //         "Verifying vote with PbftMessage: {:?} and header: {:?}",
    //         pbft_message,
    //         header
    //     );

    //     // Verify the header's signer matches the PbftMessage's signer
    //     if header.signer_id != pbft_message.get_info().get_signer_id() {
    //         return Err(PbftError::InvalidMessage(format!(
    //             "Received a vote where PbftMessage's signer ID ({:?}) and PeerMessage's signer ID \
    //              ({:?}) don't match",
    //             pbft_message.get_info().get_signer_id(),
    //             header.signer_id
    //         )));
    //     }

    //     // Verify the message type
    //     let msg_type = PbftMessageType::from(pbft_message.get_info().get_msg_type());
    //     if msg_type != expected_type {
    //         return Err(PbftError::InvalidMessage(format!(
    //             "Received a {:?} vote, but expected a {:?}",
    //             msg_type, expected_type
    //         )));
    //     }

    //     // Verify the signature
    //     let key = Secp256k1PublicKey::from_hex(&hex::encode(&header.signer_id)).map_err(|err| {
    //         PbftError::SigningError(format!(
    //             "Couldn't parse public key from signer ID ({:?}) due to error: {:?}",
    //             header.signer_id, err
    //         ))
    //     })?;
    //     let context = create_context("secp256k1").map_err(|err| {
    //         PbftError::SigningError(format!("Couldn't create context due to error: {}", err))
    //     })?;

    //     match context.verify(
    //         &hex::encode(vote.get_header_signature()),
    //         vote.get_header_bytes(),
    //         &key,
    //     ) {
    //         Ok(true) => {}
    //         Ok(false) => {
    //             return Err(PbftError::SigningError(format!(
    //                 "Vote ({}) failed signature verification",
    //                 vote
    //             )));
    //         }
    //         Err(err) => {
    //             return Err(PbftError::SigningError(format!(
    //                 "Error while verifying vote signature: {:?}",
    //                 err
    //             )));
    //         }
    //     }

    //     verify_sha512(vote.get_message_bytes(), header.get_content_sha512())?;

    //     // Validate against the specified criteria
    //     validation_criteria(&pbft_message)?;

    //     Ok(PeerId::from(pbft_message.get_info().get_signer_id()))
    // }

    /// Verify that a NewView messsage is valid
    fn verify_new_view(
        &mut self,
        new_view: &PbftMessage,
    ) -> Result<(), PbftError> {
        // Make sure this is for a future view (prevents re-using old NewView messages)
        if new_view.get_view() <= self.state.view {
            return Err(PbftError::InvalidMessage(format!(
                "Node is on view {}, but received NewView message for view {}",
                self.state.view,
                new_view.get_view(),
            )));
        }

        // Make sure this is from the new primary
        if new_view.get_signer_id()
            != self.state.get_primary_id_at_view(new_view.get_view())
        {
            return Err(PbftError::InvalidMessage(format!(
                "Received NewView message for view {} that is not from the primary for that view",
                new_view.get_view()
            )));
        }

        // Verify each individual vote and extract the signer ID from each ViewChange so the IDs
        // can be verified

        let vote_set:Vec<PbftMessage> = bcs::from_bytes(&new_view.get_info()).unwrap();
        // todo!()
        // let voter_ids =
        //     vote_set
        //         .iter()
        //         .try_fold(HashSet::new(), |mut ids, vote| {
        //             Self::verify_vote(vote, PbftMessageType::ViewChange, |msg| {
        //                 if msg.get_info().get_view() != new_view.get_info().get_view() {
        //                     return Err(PbftError::InvalidMessage(format!(
        //                         "ViewChange's view number ({}) doesn't match NewView's view \
        //                          number ({})",
        //                         msg.get_info().get_view(),
        //                         new_view.get_info().get_view(),
        //                     )));
        //                 }
        //                 Ok(())
        //             })
        //             .map(|id| ids.insert(id))?;
        //             Ok(ids)
        //         })?;

        // All of the votes must come from PBFT members, and the primary can't explicitly vote
        // itself, since broacasting the NewView is an implicit vote. Check that the votes received
        // are from a subset of "members - primary".
        let peer_ids: HashSet<_> = self.state
            .member_ids
            .iter()
            .cloned()
            .filter(|pid| pid != &PeerId::from(new_view.get_signer_id()))
            .collect();

        // trace!(
        //     "Comparing voter IDs ({:?}) with member IDs - primary ({:?})",
        //     voter_ids,
        //     peer_ids
        // );

        // if !voter_ids.is_subset(&peer_ids) {
        //     return Err(PbftError::InvalidMessage(format!(
        //         "NewView contains vote(s) from invalid IDs: {:?}",
        //         voter_ids.difference(&peer_ids).collect::<Vec<_>>()
        //     )));
        // }

        // Check that the NewView contains 2f votes (primary vote is implicit, so total of 2f + 1)
        // if (voter_ids.len() as u64) < 2 * self.state.f {
        //     return Err(PbftError::InvalidMessage(format!(
        //         "NewView needs {} votes, but only {} found",
        //         2 * self.state.f,
        //         voter_ids.len()
        //     )));
        // }

        Ok(())
    }

    /// Verify the consensus seal from the current block that proves the previous block and return
    /// the parsed seal
    // fn verify_consensus_seal_from_block(
    //     &mut self,
    //     block: &Block,
    //     state: &mut PbftState,
    // ) -> Result<PbftSeal, PbftError> {
    //     // Since block 0 is genesis, block 1 is the first that can be verified with a seal; this
    //     // means that the node won't see a seal until block 2
    //     if block.block_num < 2 {
    //         return Ok(PbftSeal::new());
    //     }

    //     // Parse the seal
    //     if block.payload.is_empty() {
    //         return Err(PbftError::InvalidMessage(
    //             "Block published without a seal".into(),
    //         ));
    //     }

    //     let seal: PbftSeal = Message::parse_from_bytes(&block.payload).map_err(|err| {
    //         PbftError::SerializationError("Error parsing seal for verification".into(), err)
    //     })?;

    //     trace!("Parsed seal: {}", seal);

    //     // Make sure this is the correct seal for the previous block
    //     if seal.block_id != block.previous_id[..] {
    //         return Err(PbftError::InvalidMessage(format!(
    //             "Seal's ID ({}) doesn't match block's previous ID ({})",
    //             hex::encode(&seal.block_id),
    //             hex::encode(&block.previous_id)
    //         )));
    //     }

    //     // Get the previous ID of the block this seal is supposed to prove so it can be used to
    //     // verify the seal
    //     let proven_block_previous_id = self
    //         .msg_log
    //         .get_block_with_id(seal.block_id.as_slice())
    //         .map(|proven_block| proven_block.previous_id.clone())
    //         .ok_or_else(|| {
    //             PbftError::InternalError(format!(
    //                 "Got seal for block {:?}, but block was not found in the log",
    //                 seal.block_id,
    //             ))
    //         })?;

    //     // Verify the seal itself
    //     self.verify_consensus_seal(&seal, proven_block_previous_id, state)?;

    //     Ok(seal)
    // }

    /// Verify the given consenus seal
    ///
    /// # Panics
    /// + If the `sawtooth.consensus.pbft.members` setting is unset or invalid
    // fn verify_consensus_seal(
    //     &mut self,
    //     seal: &PbftSeal,
    //     previous_id: BlockId,
    //     state: &mut PbftState,
    // ) -> Result<(), PbftError> {
    //     // Verify each individual vote and extract the signer ID from each PbftMessage so the IDs
    //     // can be verified
    //     let voter_ids =
    //         seal.get_commit_votes()
    //             .iter()
    //             .try_fold(HashSet::new(), |mut ids, vote| {
    //                 Self::verify_vote(vote, PbftMessageType::Commit, |msg| {
    //                     // Make sure all votes are for the right block
    //                     if msg.block_id != seal.block_id {
    //                         return Err(PbftError::InvalidMessage(format!(
    //                             "Commit vote's block ID ({:?}) doesn't match seal's ID ({:?})",
    //                             msg.block_id, seal.block_id
    //                         )));
    //                     }
    //                     // Make sure all votes are for the right view
    //                     if msg.get_info().get_view() != seal.get_info().get_view() {
    //                         return Err(PbftError::InvalidMessage(format!(
    //                             "Commit vote's view ({:?}) doesn't match seal's view ({:?})",
    //                             msg.get_info().get_view(),
    //                             seal.get_info().get_view()
    //                         )));
    //                     }
    //                     // Make sure all votes are for the right sequence number
    //                     if msg.get_info().get_seq_num() != seal.get_info().get_seq_num() {
    //                         return Err(PbftError::InvalidMessage(format!(
    //                             "Commit vote's seq_num ({:?}) doesn't match seal's seq_num ({:?})",
    //                             msg.get_info().get_seq_num(),
    //                             seal.get_info().get_seq_num()
    //                         )));
    //                     }
    //                     Ok(())
    //                 })
    //                 .map(|id| ids.insert(id))?;
    //                 Ok(ids)
    //             })?;

    //     // All of the votes in a seal must come from PBFT members, and the primary can't explicitly
    //     // vote itself, since building a consensus seal is an implicit vote. Check that the votes
    //     // received are from a subset of "members - seal creator". Use the list of members from the
    //     // block previous to the one this seal verifies, since that represents the state of the
    //     // network at the time this block was voted on.
    //     trace!("Getting on-chain list of members to verify seal");
    //     let settings = retry_until_ok(
    //         state.exponential_retry_base,
    //         state.exponential_retry_max,
    //         || {
    //             self.service.get_settings(
    //                 previous_id.clone(),
    //                 vec![String::from("sawtooth.consensus.pbft.members")],
    //             )
    //         },
    //     );
    //     let members = get_members_from_settings(&settings);

    //     // Verify that the seal's signer is a PBFT member
    //     if !members.contains(&seal.get_info().get_signer_id().to_vec()) {
    //         return Err(PbftError::InvalidMessage(format!(
    //             "Consensus seal is signed by an unknown peer: {:?}",
    //             seal.get_info().get_signer_id()
    //         )));
    //     }

    //     let peer_ids: HashSet<_> = members
    //         .iter()
    //         .cloned()
    //         .filter(|pid| pid.as_slice() != seal.get_info().get_signer_id())
    //         .collect();

    //     trace!(
    //         "Comparing voter IDs ({:?}) with on-chain member IDs - primary ({:?})",
    //         voter_ids,
    //         peer_ids
    //     );

    //     if !voter_ids.is_subset(&peer_ids) {
    //         return Err(PbftError::InvalidMessage(format!(
    //             "Consensus seal contains vote(s) from invalid ID(s): {:?}",
    //             voter_ids.difference(&peer_ids).collect::<Vec<_>>()
    //         )));
    //     }

    //     // Check that the seal contains 2f votes (primary vote is implicit, so total of 2f + 1)
    //     if (voter_ids.len() as u64) < 2 * state.f {
    //         return Err(PbftError::InvalidMessage(format!(
    //             "Consensus seal needs {} votes, but only {} found",
    //             2 * state.f,
    //             voter_ids.len()
    //         )));
    //     }

    //     Ok(())
    // }

    // ---------- Methods called in the main engine loop to periodically check and update state ----------

    /// At a regular interval, try to finalize a block when the primary is ready
    // pub fn try_publish(&mut self, state: &mut PbftState) -> Result<(), PbftError> {
    //     // Only the primary takes care of this, and we try publishing a block
    //     // on every engine loop, even if it's not yet ready. This isn't an error,
    //     // so just return Ok(()).
    //     if !state.is_primary() || state.phase != PbftPhase::PrePreparing {
    //         return Ok(());
    //     }

    //     trace!("{}: Attempting to summarize block", state);

    //     match self.service.summarize_block() {
    //         Ok(_) => {}
    //         Err(err) => {
    //             trace!("Couldn't summarize, so not finalizing: {}", err);
    //             return Ok(());
    //         }
    //     }

    //     // We don't publish a consensus seal at block 1, since we never receive any
    //     // votes on the genesis block. Leave payload blank for the first block.
    //     let data = if state.seq_num <= 1 {
    //         vec![]
    //     } else {
    //         self.build_seal(state)?.write_to_bytes().map_err(|err| {
    //             PbftError::SerializationError("Error writing seal to bytes".into(), err)
    //         })?
    //     };

    //     match self.service.finalize_block(data) {
    //         Ok(block_id) => {
    //             info!("{}: Publishing block {}", state, hex::encode(&block_id));
    //             Ok(())
    //         }
    //         Err(err) => Err(PbftError::ServiceError(
    //             "Couldn't finalize block".into(),
    //             err,
    //         )),
    //     }
    // }

    /// Check to see if the idle timeout has expired
    pub fn check_idle_timeout_expired(&mut self) -> bool {
        self.state.idle_timeout.check_expired()
    }

    /// Start the idle timeout
    pub fn start_idle_timeout(&mut self) {
        self.state.idle_timeout.start();
    }

    /// Check to see if the commit timeout has expired
    pub fn check_commit_timeout_expired(&mut self) -> bool {
        self.state.commit_timeout.check_expired()
    }

    /// Start the commit timeout
    pub fn start_commit_timeout(&mut self) {
        self.state.commit_timeout.start();
    }

    /// Check to see if the view change timeout has expired
    pub fn check_view_change_timeout_expired(&mut self) -> bool {
        self.state.view_change_timeout.check_expired()
    }
    // ---------- Methods for communication between nodes ----------

    /// Construct a PbftMessage message and broadcast it to all peers (including self)
    fn broadcast_pbft_message(
        &mut self,
        view: u64,
        seq_num: u64,
        msg_type: PbftMessageType,
        block_id: BlockId,
    ) -> Result<(), PbftError> {
        let mut msg = PbftMessage::new(msg_type, view, seq_num, self.state.id.clone(), block_id);
        trace!("{}: Created PBFT message: {:?}", self.state, msg);

        self.broadcast_message(msg)
    }

    /// Broadcast the specified message to all of the node's peers, including itself
    fn broadcast_message(
        &mut self,
        msg: PbftMessage,
    ) -> Result<(), PbftError> {
        // Broadcast to peers
        // self.service
        //     .broadcast(
        //         String::from(msg.info().get_msg_type()).as_str(),
        //         msg.message_bytes.clone(),
        //     )
        //     .unwrap_or_else(|err| {
        //         error!(
        //             "Couldn't broadcast message ({:?}) due to error: {}",
        //             msg, err
        //         )
        //     });

        // Send to self
        self.self_tx.send(msg);
        Ok(())
    }

    /// Build a consensus seal for the last block this node committed and send it to the node that
    /// requested the seal (the `recipient`)
    // #[allow(clippy::ptr_arg)]
    // fn send_seal_response(
    //     &mut self,
    //     state: &PbftState,
    //     recipient: &PeerId,
    // ) -> Result<(), PbftError> {
    //     let seal = self.build_seal(state).map_err(|err| {
    //         PbftError::InternalError(format!("Failed to build requested seal due to: {}", err))
    //     })?;

    //     let msg_bytes = seal.write_to_bytes().map_err(|err| {
    //         PbftError::SerializationError("Error writing seal to bytes".into(), err)
    //     })?;

    //     // Send the seal to the requester
    //     self.service
    //         .send_to(
    //             recipient,
    //             String::from(PbftMessageType::Seal).as_str(),
    //             msg_bytes,
    //         )
    //         .map_err(|err| {
    //             PbftError::ServiceError(
    //                 format!(
    //                     "Failed to send requested seal to {:?}",
    //                     hex::encode(recipient)
    //                 ),
    //                 err,
    //             )
    //         })
    // }

    // ---------- Miscellaneous methods ----------

    /// Start a view change when this node suspects that the primary is faulty
    ///
    /// Update state to reflect that the node is now in the process of this view change, start the
    /// view change timeout, and broadcast a view change message
    ///
    /// # Panics
    /// + If the view change timeout overflows
    pub fn start_view_change(&mut self, view: u64) -> Result<(), PbftError> {
        // Do not send messages again if we are already in the midst of this or a later view change
        if match self.state.mode {
            PbftMode::ViewChanging(v) => view <= v,
            _ => false,
        } {
            return Ok(());
        }

        info!("{}: Starting change to view {}", self.state, view);

        self.state.mode = PbftMode::ViewChanging(view);

        // Stop the idle and commit timeouts because they are not needed until after the view
        // change
        self.state.idle_timeout.stop();
        self.state.commit_timeout.stop();
        // Stop the view change timeout if it is already active (will be restarted when 2f + 1
        // ViewChange messages for the new view are received)
        self.state.view_change_timeout.stop();

        // Broadcast the view change message
        self.broadcast_pbft_message(
            view,
            self.state.seq_num - 1,
            PbftMessageType::ViewChange,
            BlockId::new(),
        )
    }
}

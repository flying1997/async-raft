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

use core::time;
use std::collections::HashSet;
use std::convert::From;
use std::sync::Arc;
use std::thread;

use rl_logger::{debug, error, info, trace, warn};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use types::network::network_message::Message;
use types::pbft::consensus::{Block, BlockId, PbftMessage, PbftMessageType, PeerId, is_in_view};

use crate::config::{PbftConfig};
use crate::error::PbftError;

use crate::message_log::PbftLog;
use crate::pbft_sender::PbftNetwork;
use crate::state::{PbftMode, PbftPhase, PbftState};
use crate::timing::{Timeout};

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
    pub fn start(peer_id: PeerId, uid: u64, config: &PbftConfig,
        service: Arc<N>, self_tx: UnboundedSender<PbftMessage>, events: UnboundedReceiver<PbftMessage>) -> JoinHandle<()>{
        let state = PbftState::new(peer_id, uid, 0, config);
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
        info!("Start Pbft main thread!");

        if self.state.is_primary(){
            info!("Create gensis block!!!");
            let block = Block::new(vec![0], vec![0], vec![0], 0, vec![1,2,3]);
            // init gensis block
            let one = time::Duration::from_secs(1);
            thread::sleep(one);
            self.broadcast_block(0, 0, PbftMessageType::PrePrepare, vec![0], bcs::to_bytes(&block).unwrap());
            // self.broadcast_pbft_message();
        }

        loop {

            // info!("Incoming message: {:?}", incoming_message);
            info!("state:{:?}", self.state.phase);
            
            tokio::select! {
                // if self.state.phase == PbftPhase::Finishing(true) =>{
                    // self.state.phase = PbftPhase::PrePreparing;


                    // let v = self.state.seq_num.to_string().as_bytes().to_owned();
                    // let block = Block::new(v.clone(), vec![0], self.state.id.clone(), self.state.seq_num, vec![1]);
                    // self.broadcast_block(self.state.view, self.state.seq_num, PbftMessageType::PrePrepare, v, bcs::to_bytes(&block).unwrap());

                // }
                Some(incoming_message) = self.events.recv() =>{
                    self.on_peer_message(incoming_message).await;
                }

            }
            
            // If the block publishing delay has passed, attempt to publish a block
            // block_publishing_ticker.tick(|| log_any_error(node.try_publish(state)));
    
            // If the idle timeout has expired, initiate a view change
            // if self.check_idle_timeout_expired() {
            //     warn!("Idle timeout expired; proposing view change");
            //     self.start_view_change(self.state.view + 1);
            // }
    
            // If the commit timeout has expired, initiate a view change
            // if self.check_commit_timeout_expired() {
            //     warn!("Commit timeout expired; proposing view change");
            //     self.start_view_change(self.state.view + 1);
            // }
    
            // Check the view change timeout if the node is view changing so we can start a new
            // view change if we don't get a NewView in time
            // if let PbftMode::ViewChanging(v) = self.state.mode {
            //     if self.check_view_change_timeout_expired() {
            //         warn!(
            //             "View change timeout expired; proposing view change for view {}",
            //             v + 1
            //        );
            //        self.start_view_change(v + 1);
            //     }
            // }
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
        if !self.contains_id(&msg.get_signer_id()) {
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
        info!("Recevice message:{:?}", msg);
        match msg_type {
            PbftMessageType::PrePrepare => self.handle_pre_prepare(msg)?,
            PbftMessageType::Prepare => self.handle_prepare(msg)?,
            PbftMessageType::Commit => self.handle_commit(msg)?,
            PbftMessageType::Reply => self.handle_reply(msg)?,
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
        let block: Block = bcs::from_bytes(&msg.get_info()).unwrap();
        self.msg_log.add_validated_block(block);

        // If the node is in the PrePreparing phase, this message is for the current sequence
        // number, and the node already has this block: switch to Preparing
        info!("try_preparing!!!!");
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
                >= 2 * self.state.f;
                // >= (self.state.member_ids.len()-1) as u64;
            if has_matching_pre_prepare && has_required_prepares {
                self.state.switch_phase(PbftPhase::Committing)?;

                
                let commit_msg = self.msg_log.get_messages_of_type_seq_view(PbftMessageType::Commit, self.state.seq_num, self.state.view)
                .iter().cloned().cloned().collect::<Vec<_>>();
                for i in commit_msg{
                    self.handle_commit(i);
                }

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
                // >= self.state.member_ids.len() as u64;
            if has_matching_pre_prepare && has_required_commits {
                info!("Apply to DataBase!!!!");
                // todo! commit block
                // Stop the commit timeout, since the network has agreed to commit the block
                self.state.commit_timeout.stop();

                self.state.seq_num += 1;
                self.state.mode = PbftMode::Normal;
                if(self.state.is_primary()){
                    self.state.phase = PbftPhase::Finishing(false);  

                }else{
    
                    self.state.phase = PbftPhase::PrePreparing;
                    self.send_to_leader(PbftMessageType::Reply, self.state.view, self.state.seq_num);

                    let prepre_msg = self.msg_log.get_messages_of_type_seq_view(PbftMessageType::PrePrepare, self.state.seq_num, self.state.view)
                    .iter().cloned().cloned().collect::<Vec<_>>();
                    for i in prepre_msg{
                        self.handle_pre_prepare(i);
                    }
                   
                }
                
                
               
                
                
               
                //????
                // self.state.chain_head = block_id.clone();
                // Increment the view if a view change must be forced for fairness
                // if self.state.at_forced_view_change() {
                //     self.state.view += 1;
                // }
                
                self.msg_log.garbage_collect(self.state.seq_num);
            }
        }

        Ok(())
    }


    ///Handle a 'Reply mseeage
    /// only leader to handle
    /// 
    fn handle_reply(
        &mut self,
        msg: PbftMessage,
    ) -> Result<(), PbftError>{

        // Check that the message is for the current view
        if msg.get_view() != self.state.view {
            return Err(PbftError::InvalidMessage(format!(
                "Node is on view {}, but a Commit for view {} was received",
                self.state.view,
                msg.get_view(),
            )));
        }
        self.msg_log.add_message(msg.clone());
        if msg.get_seq_num() == self.state.seq_num && self.state.phase == PbftPhase::Finishing(false) {
            let is_next = self
                .msg_log
                .get_messages_of_type_seq_view(PbftMessageType::Reply, self.state.seq_num, self.state.view)
                .len() as u64 >= self.state.f;
            if(is_next){
                    // let one = time::Duration::from_secs(1);
                    // thread::sleep(one);

                    // self.state.phase = PbftPhase::Finishing(true);

                    self.state.phase = PbftPhase::PrePreparing;

                    loop{
                        if true{
                            let v = self.state.seq_num.to_string().as_bytes().to_owned();
                            let block = Block::new(v.clone(), vec![0], self.state.id.clone(), self.state.seq_num, vec![1]);
                            self.broadcast_block(self.state.view, self.state.seq_num, PbftMessageType::PrePrepare, v, bcs::to_bytes(&block).unwrap());
                            break
                        }
                    }

                    

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



    /// When the node has a block and a corresponding PrePrepare for its current sequence number,
    /// and it is in the PrePreparing phase, it can enter the Preparing phase and broadcast its
    /// Prepare
    fn try_preparing(&mut self, block_id: BlockId) -> Result<(), PbftError> {

        if let Some(block) = self.msg_log.get_block_with_id(&block_id) {
            
            info!("state phase:{:?}, state seq:{:?}, state view: {:?}",self.state.phase, self.state.seq_num, self.state.view);
            if self.state.phase == PbftPhase::PrePreparing
                && self.msg_log.has_pre_prepare(self.state.seq_num, self.state.view, &block_id)
                // PrePrepare.seq_num == state.seq_num == block.block_num enforces the one-to-one
                // correlation between seq_num and block_num (PrePrepare n should be for block n)
                && block.block_num == self.state.seq_num
            {
                // info!("====================");
                self.state.switch_phase(PbftPhase::Preparing)?;

                // Stop idle timeout, since a new block and valid PrePrepare were received in time
                self.state.idle_timeout.stop();

                // Now start the commit timeout in case the network fails to commit the block
                // within a reasonable amount of time
                self.state.commit_timeout.start();

                // handle previous prepare 
                let vec = self.msg_log.get_messages_of_type_seq_view(PbftMessageType::Prepare, self.state.seq_num, self.state.view)
                          .iter().cloned().cloned().collect::<Vec<_>>();
                for i in vec{
                    self.handle_prepare(i);
                }
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
            .filter(|(_, pid)| pid != &PeerId::from(new_view.get_signer_id()))
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


    // ---------- Methods called in the main engine loop to periodically check and update state ----------


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
       
        let mut target = Vec::new();
        for (i,_) in self.state.member_ids.iter(){
            if *i == self.state.uid{
                continue
            }
            target.push(*i);
        } 
        info!("Broadcast {:?}, target:{:?} ", msg, target);
        self.service.broadcast(target, Message::new(bcs::to_bytes(&msg).unwrap()));

        // Send to self
        self.self_tx.send(msg);
        Ok(())
    }
    /// broadcast a new block
    pub fn broadcast_block(&mut self, view: u64,
        seq_num: u64,
        msg_type: PbftMessageType,
        block_id: BlockId, info: Vec<u8>) {
        let mut msg = PbftMessage::new(msg_type, view, seq_num, self.state.id.clone(), block_id);   
        msg.set_info(info);
        self.broadcast_message(msg);
    }


    /// send message to leader
    /// 
    fn send_to_leader(&mut self, msg_type: PbftMessageType, view: u64, seq: u64) {
        let mut msg = PbftMessage::new(msg_type, view, seq, self.state.id.clone(), vec![]);
        self.service.send_to(self.state.get_primary_uid(), Message::new(bcs::to_bytes(&msg).unwrap()));
    }
    pub fn contains_id(&self, id: &PeerId) -> bool{
        for (_, i) in self.state.member_ids.iter(){
            if i.eq(id){
                return true
            }
        }
        false
    }
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

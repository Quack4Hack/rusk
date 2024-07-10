// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, RoundUpdate};
use crate::iteration_ctx::RoundCommittees;
use crate::user::committee::Committee;
use async_trait::async_trait;
use node_data::bls::PublicKeyBytes;
use node_data::message::{Message, Status};
use node_data::StepName;
use tracing::{debug, trace};

/// Indicates whether an output value is available for current step execution
/// (Step is Ready) or needs to collect data (Step is Pending)
#[allow(clippy::large_enum_variant)]
pub enum HandleMsgOutput {
    Pending,
    Ready(Message),
}

/// MsgHandler must be implemented by any step that needs to handle an external
/// message within event_loop life-cycle.
#[async_trait]
pub trait MsgHandler {
    /// is_valid checks a new message is valid in the first place.
    ///
    /// Only if the message has correct round and step and is signed by a
    /// committee member then we delegate it to Phase::verify.
    fn is_valid(
        &self,
        msg: &Message,
        ru: &RoundUpdate,
        iteration: u8,
        step: StepName,
        committee: &Committee,
        round_committees: &RoundCommittees,
    ) -> Result<(), ConsensusError> {
        let signer = msg.get_signer().ok_or(ConsensusError::InvalidMsgType)?;
        debug!(
            event = "msg received",
            signer = signer.to_bs58(),
            topic = ?msg.topic(),
            step = msg.get_step(),
        );

        trace!(event = "msg received", msg = format!("{:#?}", msg),);

        match msg.compare(ru.round, iteration, step) {
            Status::Past => Err(ConsensusError::PastEvent),
            Status::Present => {
                let msg_tip = msg.header.prev_block_hash;
                if msg_tip != ru.hash() {
                    return Err(ConsensusError::InvalidPrevBlockHash(msg_tip));
                }

                // Ensure the message originates from a committee member.
                if !committee.is_member(signer) {
                    return Err(ConsensusError::NotCommitteeMember);
                }

                // Delegate message final verification to the phase instance.
                // It is the phase that knows what message type to expect and if
                // it is valid or not.
                self.verify(msg, iteration, round_committees)
            }
            Status::Future => Err(ConsensusError::FutureEvent),
        }
    }

    /// verify allows each Phase to fully verify the message payload.
    fn verify(
        &self,
        msg: &Message,
        iteration: u8,
        round_committees: &RoundCommittees,
    ) -> Result<(), ConsensusError>;

    /// collect allows each Phase to process a verified inbound message.
    async fn collect(
        &mut self,
        msg: Message,
        ru: &RoundUpdate,
        committee: &Committee,
        generator: Option<PublicKeyBytes>,
    ) -> Result<HandleMsgOutput, ConsensusError>;

    /// collect allows each Phase to process a verified message from a former
    /// iteration
    async fn collect_from_past(
        &mut self,
        msg: Message,
        ru: &RoundUpdate,
        committee: &Committee,
        generator: Option<PublicKeyBytes>,
    ) -> Result<HandleMsgOutput, ConsensusError>;

    /// handle_timeout allows each Phase to handle a timeout event.
    fn handle_timeout(&self) -> Result<HandleMsgOutput, ConsensusError>;
}

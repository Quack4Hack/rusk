// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

// RoundUpdate carries the data about the new Round, such as the active
// Provisioners, the BidList, the Seed and the Hash.

use std::collections::HashMap;
use std::time::Duration;

use dusk_core::signatures::bls::SecretKey as BlsSecretKey;
use node_data::bls::PublicKey;
use node_data::ledger::*;
use node_data::message::{payload, ConsensusHeader};
use node_data::StepName;

use crate::operations::Voter;

pub type TimeoutSet = HashMap<StepName, Duration>;

#[derive(Clone, Default, Debug)]
pub struct RoundUpdate {
    // Current round number of the ongoing consensus
    pub round: u64,

    // This provisioner consensus keys
    pub pubkey_bls: PublicKey,
    pub secret_key: BlsSecretKey,

    seed: Seed,
    hash: [u8; 32],
    state_root: [u8; 32],
    att: Attestation,
    att_voters: Vec<Voter>,
    timestamp: u64,

    pub base_timeouts: TimeoutSet,
}

impl RoundUpdate {
    pub fn new(
        pubkey_bls: PublicKey,
        secret_key: BlsSecretKey,
        tip_header: &Header,
        base_timeouts: TimeoutSet,
        att_voters: Vec<Voter>,
    ) -> Self {
        let round = tip_header.height + 1;
        RoundUpdate {
            round,
            pubkey_bls,
            secret_key,
            att: tip_header.att,
            hash: tip_header.hash,
            seed: tip_header.seed,
            timestamp: tip_header.timestamp,
            base_timeouts,
            att_voters,
            state_root: tip_header.state_hash,
        }
    }

    pub fn seed(&self) -> Seed {
        self.seed
    }

    pub fn hash(&self) -> [u8; 32] {
        self.hash
    }

    pub fn att(&self) -> &Attestation {
        &self.att
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn att_voters(&self) -> &Vec<Voter> {
        &self.att_voters
    }

    pub fn state_root(&self) -> [u8; 32] {
        self.state_root
    }
}

#[async_trait::async_trait]
pub trait Database: Send + Sync {
    async fn store_candidate_block(&mut self, b: Block);
    async fn store_validation_result(
        &mut self,
        ch: &ConsensusHeader,
        vr: &payload::ValidationResult,
    );
    async fn get_last_iter(&self) -> (Hash, u8);
    async fn store_last_iter(&mut self, data: (Hash, u8));
}

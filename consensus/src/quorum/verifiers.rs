// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node_data::bls::PublicKey;
use node_data::ledger::{Seed, StepVotes};
use node_data::message::payload::{self, Quorum, Vote};
use node_data::message::{ConsensusHeader, StepMessage};
use node_data::{Serializable, StepName};

use crate::commons::StepSigError;
use crate::user::cluster::Cluster;
use crate::user::committee::{Committee, CommitteeSet};
use crate::user::sortition;

use crate::config::CONSENSUS_MAX_ITER;
use dusk_bytes::Serializable as BytesSerializable;
use execution_core::{BlsAggPublicKey, BlsSignature};
use tokio::sync::RwLock;
use tracing::error;

/// Performs all three-steps verification of a quorum msg.
pub async fn verify_quorum(
    quorum: &Quorum,
    committees_set: &RwLock<CommitteeSet<'_>>,
    seed: Seed,
) -> Result<(), StepSigError> {
    // Verify validation
    verify_step_votes(
        &quorum.header,
        quorum.vote(),
        &quorum.att.validation,
        committees_set,
        seed,
        StepName::Validation,
    )
    .await
    .map_err(|e| {
        error!(
            desc = "invalid validation",
            sv = ?quorum.att.validation,
            hdr = ?quorum.header,
        );
        e
    })?;

    // Verify ratification
    verify_step_votes(
        &quorum.header,
        quorum.vote(),
        &quorum.att.ratification,
        committees_set,
        seed,
        StepName::Ratification,
    )
    .await
    .map_err(|e| {
        error!(
            desc = "invalid ratification",
            sv = ?quorum.att.ratification,
            hdr = ?quorum.header,
        );
        e
    })?;

    Ok(())
}

pub async fn verify_step_votes(
    header: &ConsensusHeader,
    vote: &Vote,
    sv: &StepVotes,
    committees_set: &RwLock<CommitteeSet<'_>>,
    seed: Seed,
    step: StepName,
) -> Result<(QuorumResult, Committee), StepSigError> {
    let round = header.round;
    let iteration = header.iteration;

    let mut exclusion_list = vec![];
    let generator = committees_set
        .read()
        .await
        .provisioners()
        .get_generator(iteration, seed, round);

    exclusion_list.push(generator);

    if iteration < CONSENSUS_MAX_ITER {
        let next_generator = committees_set
            .read()
            .await
            .provisioners()
            .get_generator(iteration + 1, seed, round);

        exclusion_list.push(next_generator);
    }

    let cfg =
        sortition::Config::new(seed, round, iteration, step, exclusion_list);

    if committees_set.read().await.get(&cfg).is_none() {
        let _ = committees_set.write().await.get_or_create(&cfg);
    }

    let set = committees_set.read().await;
    let committee = set.get(&cfg).expect("committee to be created");

    let quorum_result = verify_votes(header, step, vote, sv, committee)?;
    Ok((quorum_result, committee.clone()))
}

#[derive(Default)]
pub struct QuorumResult {
    pub total: usize,
    pub target_quorum: usize,
}

impl QuorumResult {
    pub fn quorum_reached(&self) -> bool {
        self.total >= self.target_quorum
    }
}

pub fn verify_votes(
    header: &ConsensusHeader,
    step: StepName,
    vote: &Vote,
    step_votes: &StepVotes,
    committee: &Committee,
) -> Result<QuorumResult, StepSigError> {
    let bitset = step_votes.bitset;
    let signature = step_votes.aggregate_signature().inner();
    let sub_committee = committee.intersect(bitset);

    let total = committee.total_occurrences(&sub_committee);
    let target_quorum = match vote {
        Vote::Valid(_) => committee.super_majority_quorum(),
        _ => committee.majority_quorum(),
    };

    let quorum_result = QuorumResult {
        total,
        target_quorum,
    };

    let skip_quorum = step == StepName::Validation && vote == &Vote::NoQuorum;

    if !skip_quorum && !quorum_result.quorum_reached() {
        tracing::error!(
            desc = "vote_set_too_small",
            committee = format!("{:#?}", sub_committee),
            bitset,
            target_quorum,
            total,
            ?vote
        );
        return Err(StepSigError::VoteSetTooSmall);
    }

    // If bitset=0 this means that we are checking for failed iteration
    // attestations. If a winning attestation is checked with bitset=0 it will
    // fail to pass the quorum and results in VoteSetTooSmall.
    // FIXME: Anyway this should be handled properly, maybe with a different
    // function
    if bitset > 0 {
        // aggregate public keys
        let apk = sub_committee.aggregate_pks()?;

        // verify signatures
        verify_step_signature(header, step, vote, apk, signature)?;
    }
    // Verification done
    Ok(quorum_result)
}

impl Cluster<PublicKey> {
    fn aggregate_pks(&self) -> Result<BlsAggPublicKey, StepSigError> {
        let pks: Vec<_> =
            self.iter().map(|(pubkey, _)| *pubkey.inner()).collect();

        match pks.split_first() {
            Some((first, rest)) => {
                let mut apk = BlsAggPublicKey::from(first);
                apk.aggregate(rest)?;
                Ok(apk)
            }
            None => Err(StepSigError::EmptyApk),
        }
    }
}

fn verify_step_signature(
    header: &ConsensusHeader,
    step: StepName,
    vote: &Vote,
    apk: BlsAggPublicKey,
    signature: &[u8; 48],
) -> Result<(), StepSigError> {
    // Compile message to verify
    let sign_seed = match step {
        StepName::Validation => payload::Validation::SIGN_SEED,
        StepName::Ratification => payload::Ratification::SIGN_SEED,
        StepName::Proposal => Err(StepSigError::InvalidType)?,
    };

    let sig = BlsSignature::from_bytes(signature)?;
    let mut msg = header.signable();
    msg.extend_from_slice(sign_seed);
    vote.write(&mut msg).expect("Writing to vec should succeed");
    apk.verify(&sig, &msg)?;
    Ok(())
}

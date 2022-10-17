// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::agreement::verifiers::verify_agreement;
use crate::commons::{Hash, RoundUpdate};
use crate::messages;
use crate::messages::{payload, Message, Payload};
use crate::user::committee::CommitteeSet;
use crate::user::sortition;
use hex::ToHex;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{error, info, warn, Instrument};

/// AgreementsPerStep is a mapping of StepNum to Set of Agreements,
/// where duplicated agreements per step are not allowed.
type AgreementsPerStep = HashMap<u8, (HashSet<payload::Agreement>, usize)>;

/// StorePerHash implements a mapping of a block hash to AgreementsPerStep,
/// where AgreementsPerStep is a mapping of StepNum to Set of Agreements.
type StorePerHash = HashMap<Hash, AgreementsPerStep>;

pub(crate) struct Accumulator {
    workers: Vec<JoinHandle<()>>,
    tx: async_channel::Sender<Message>,
    rx: async_channel::Receiver<Message>
}

impl Accumulator {
    pub fn new() -> Self {
        let (tx, rx) = async_channel::unbounded();
 
        Self {
            workers: vec![],
            tx,
            rx,
        }
    }

    pub fn spawn_workers_pool(&mut self,
        workers_amount: usize,
        output_chan: Sender<Message>,
        committees_set: Arc<Mutex<CommitteeSet>>,
        ru: RoundUpdate,
    )  {
        assert!(workers_amount > 0);

        let stores = Arc::new(Mutex::new(StorePerHash::default()));

        // Spawn a set of workers to process all agreement message
        // verifications and accumulate results.
        // Final result is written to output_chan.
        for _i in 0..workers_amount {
            let rx = self.rx.clone();
            let committees_set = committees_set.clone();
            let output_chan = output_chan.clone();
            let stores = stores.clone();

            self.workers.push(tokio::spawn(
                async move {
                    // Process each request for verification
                    while let Ok(msg) = rx.recv().await {
                        if msg.header.block_hash == [0; 32] {
                            // discard empty block hash
                            continue
                        }
                        
                        if let Err(e) =
                            verify_agreement(msg.clone(), committees_set.clone(), ru.seed).await
                        {
                            error!("{:#?}", e);
                            continue;
                        }

                        if let Some(msg) =
                            Self::accumulate( stores.clone(), committees_set.clone(), msg, ru.seed)
                                .await
                        {
                            output_chan.send(msg).await.unwrap_or_else(|err| {
                                error!("unable to send_msg collected_votes {:?}", err)
                            });
                            break;
                        }
                    }
                }
                .instrument(tracing::info_span!("acc_task",)),
            ));
        }
 
    }


    pub async fn process(&mut self, msg: Message) {
        self.tx
            .send(msg)
            .await
            .unwrap_or_else(|err| error!("unable to queue agreement_msg {:?}", err));
    }

    async fn accumulate(
        stores: Arc< Mutex< StorePerHash>>,
        committees_set: Arc<Mutex<CommitteeSet>>,
        msg: messages::Message,
        seed: [u8; 32],
    ) -> Option<messages::Message> {
        let hdr = msg.header;

        let cfg = sortition::Config::new(seed, hdr.round, hdr.step, 64);

        // Mutex guard used here to fetch all data needed from CommitteeSet
        let (weight, target_quorum) = {
            let mut guard = committees_set.lock().await;

            let weight = guard.votes_for(hdr.pubkey_bls, cfg)?;
            if *weight == 0 {
                warn!("Agreement was not accumulated since it is not from a committee member");
                return None;
            }

            Some((*weight, guard.quorum(cfg)))
        }?;

        if let Payload::Agreement(payload) = msg.payload {
            let mut guard = stores.lock().await;

            let (agr_set, agr_weight) = guard
                .entry(hdr.block_hash)
                .or_insert_with(AgreementsPerStep::default)
                .entry(hdr.step)
                .or_insert((HashSet::new(), 0));

            if agr_set.contains(&payload) {
                warn!("Agreement was not accumulated since it is a duplicate");
                return None;
            }

            // Save agreement to avoid duplicates
            agr_set.insert(payload);

            // Increase the cumulative weight
            *agr_weight += weight;

            if *agr_weight >= target_quorum {
                info!(
                    "event=quorum reached, hash={} msg_round={}, msg_step={}, target={}, aggr_count={} ",
                    hdr.block_hash.encode_hex::<String>(),hdr.round, hdr.step, target_quorum, agr_weight
                );

                // TODO: CollectedVotes Message
                return Some(Message::empty());
            }
        }

        None
    }
}

impl Drop for Accumulator {
    fn drop(&mut self) {
        // Abort all workers
        for handle in self.workers.iter() {
            handle.abort();
        }

        self.workers.clear();
    }
}

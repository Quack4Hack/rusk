// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # blindbid-circuits
//! ![Build Status](https://github.com/dusk-network/rusk/workflows/Continuous%20integration/badge.svg)
//! [![Repository](https://img.shields.io/badge/github-blindbid-circuits-blueviolet?logo=github)](https://github.com/dusk-network/rusk/circuits/blindbid)
//! [![Documentation](https://img.shields.io/badge/docs-blindbid--circuits-blue?logo=rust)](https://docs.rs/dusk-blindbid/)
//!
//! ## Contents
//!
//! This library provides the implementation of the [`BlindBidCircuit`] which allows the user
//! to construct and verify a Proof of BlindBid.
//! ### Example
//! ```rust,ignore
//! use rand::{Rng, thread_rng};
//! use dusk_plonk::prelude::*;
//! use dusk_plonk::jubjub::JubJubAffine;
//! use blindbid_circuits::BlindBidCircuit;
//! // Assume the following fields come from the outside:
//! let poseidon_tree_root = BlsScalar::random(&mut thread_rng());
//! let bid_hash = BlsScalar::Random(&mut thread_rng());
//! let hashed_secret = BlsScalar::Random(&mut thread_rng());
//! let prover_id = BlsScalar::Random(&mut thread_rng());
//! let score_value = BlsScalar::Random(&mut thread_rng());
//!
//! let (pk, vd) = BlindBidCircuit::default().compile();
//! // Initialize your `PublicInputValue` Vector.
//! let pi: Vec<PublicInputValue> = vec![
//!     poseidon_tree_root.into(),
//!     bid_hash.into(),
//!     JubJubAffine::identity().into(),
//!     hashed_secret.into(),
//!     prover_id.into(),
//!     score_value().into(),
//! ];
//!
//! // Assuming we got a proof from somewhere which we want to verify.
//! circuit::verify_proof(&pub_params, &vd.key(), &proof, &pi, &vd.pi_pos(), b"CorrectnessBid")
//! ```
//!
//!
//! ## Rationale & Theory
//!
//! In order to participate in the SBA consensus, Block generators have to
//! submit a bid in DUSK. As long as their bid is active - and their full-node
//! is connected with the internet and running- they are participating in the
//! consensus rounds. Essentially, every time a consensus round is run, the
//! Block Generator software generates a comprehensive zero-knowledge proof, and
//! executes various steps in order to generate a valid candidate block, and
//! compete with the other Block Generators for a chance to become the winner of
//! the consensus round.
//!
//! ![](https://user-images.githubusercontent.com/1636833/107039506-468c9e80-67be-11eb-9fb1-7ba999b3d6dc.png)
//!
//! Below we describe the three main processes that happen
//! every consensus round. Please note that 1 and 2 are run as part of the same
//! algorithm.
//!
//! ### 1: Score generation.
//! Block Generators obtain a score from a lottery by executing the Score
//! Generation Function. The score is positively influenced by the amount of
//! DUSK that the Block Generator bids. In other words, the higher the bid, the
//! better the chance to generate a high score. This is important to guarantee
//! _Sybil attack_ protection.
//!
//! Without this link a bad actor could subvert the reputation system by
//! creating multiple identities. Also note: there are _minimum_ and _maximum_
//! thresholds that determine the minimum and maximum size of the bid.
//!
//! ### 2. Proof of Blind-Bid Generation.
//!
//! In general computing science, a circuit is a computational model through
//! which input values proceed through a sequence of gates, each of which
//! computes a specific function. In our case, the circuits perform the logical
//! checks with public and private inputs to make sure that the generated Blind
//! Bid proofs are generated by the rules of the game. For explanatory reasons,
//! we define two circuits although in practice, these two are a collection of
//! gadgets added up together in order to compose the [`BlindBidCircuit`]:
//!
//! 1. Blind Bid circuit;
//! 2. Score Generation circuit.
//!
//! Below we describe the Blind Bid circuit and the score generation circuit,
//! who together form the pillars of the Proof-of-Blind Bid procedure.
//!
//! #### Blind Bid Circuit
//! ![Fig1](https://user-images.githubusercontent.com/1636833/107039495-4391ae00-67be-11eb-8c76-9314c0f3b77c.png)
//!
//! Some noteworthy proofs are:
//!
//! Opening Proof: this is generated to check where the Bid has been stored on
//! the merkle-tree (you could see this as a ledger where values are stored)
//! that contains all of the bids. This proof checks that the Bid has indeed
//! been made, and can be trusted.
//!
//! Pre-image check of the Bid: this is a consistency check that aims to make it
//! impossible to cheat during the computation of the bid. If a bad actor
//! attempts to cheat, the opening proof will not be the same and therefore not
//! consistent.
//!
//! It goes both ways. If you try to cheat on the pre-image check, the Opening
//! Proof will fail as a result. And if you try to cheat on the Opening Proof,
//! the pre-image would be impossible to compute because there are 2^256
//! different possibilities. To put that in perspective, even with all the time
//! in the universe, you would not be able to check all of them (note that a
//! consensus round also only takes ~10 seconds).
//!
//! In Fig 1. you can see that in step 3. & 4 we perform range checks to make
//! sure that the Bid is valid and eligible during the current consensus round
//! and steps. Finally, in proofs 7. & 8. we check the hash of the secret (H(k))
//! and the prover ID (i), asking for proof that the block generator - who we
//! assume has posted the bid -, indeed is the owner of the bid.
//!
//! Once the process above has been completed we move to Score Generation.
//!
//! #### Score Generation Circuit
//! ![Fig2](https://user-images.githubusercontent.com/1636833/107039501-455b7180-67be-11eb-8e69-f7a96cf98d52.png)
//!
//! Score generation needs to be understood as a continuation of the next
//! circuit instead of a different entity.
//!
//! The final step is to check if the Score in the Blind Bid is correct. This
//! step is important, as the Score determines the winner of an election round.
//!
//! The prover ID (y) is directly connected to the secret (k) and pre-image hash
//! of the Bid (H(bidi)), meaning that any changes to the score will
//! automatically result in a different prover ID, and thus a failed constraint
//! on line 1. of the Score Generation Circuit.
//!
//! ### 3. Propagation.
//! During each consensus round, the Block Generator checks
//! the score that he produced, and verifies whether it is greater than the
//! _**minimum score threshold**_. If it is indeed greater, then the Block
//! Generator generates the aforementioned proofs and propagates the score
//! obtained, the zero-knowledge proof computed and various other elements
//! alongside the Block Candidate to his peers in the network.
//! The Block Generator that computed the highest score is considered to be the
//! leader of the current iteration of the consensus.
//!
//! # Documentation
//! The best usage example of this library can actually be found in the Bid
//! contract. This is the place where this lib provides all it's
//! functionallities together with PoseidonTrees and Zero Knowledge Proofs.
//! See: <https://github.com/dusk-network/rusk/tree/master/contracts/bid for more info and detail.>
//!
//! You can also check the documentation of this crate [here](https://docs.rs/blindbid-circuits/0.1.0/).
//!
//! ## Licensing
//! This code is licensed under Mozilla Public License Version 2.0 (MPL-2.0).
//! Please see [LICENSE](https://github.com/dusk-network/dusk-blindbid/blob/master/LICENSE) for further info.
//!
//! ## About
//! Protocol & Implementation designed by the [dusk](https://dusk.network) team.
//!
//! ## Contributing
//! - If you want to contribute to this repository/project please, check [CONTRIBUTING.md](https://github.com/dusk-network/dusk-blindbid/blob/master/CONTRIBUTING.md)
//! - If you want to report a bug or request a new feature addition, please open
//!   an issue on this repository.

#![doc(
    html_logo_url = "https://lh3.googleusercontent.com/SmwswGxtgIANTbDrCOn5EKcRBnVdHjmYsHYxLq2HZNXWCQ9-fZyaea-bNgdX9eR0XGSqiMFi=w128-h128-e365",
    html_favicon_url = "https://dusk.network/lib/img/favicon-16x16.png",
    html_root_url = "https://docs.rs/blindbid-circuits/0.0.0"
)]

pub(crate) mod error;
pub(crate) mod gadgets;
pub mod proof;

pub use error::BlindBidCircuitError;
pub use proof::BlindBidCircuit;

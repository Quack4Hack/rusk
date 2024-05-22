// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types used for transactions with Dusk's transfer contract.

#![no_std]
#![deny(missing_docs)]
#![deny(clippy::pedantic)]

extern crate alloc;
use alloc::vec::Vec;

use dusk_bls12_381::BlsScalar;

use bytecheck::CheckBytes;
use phoenix_core::{Note, StealthAddress};
use rkyv::{Archive, Deserialize, Serialize};

/// Module Id
pub type ModuleId = [u8; 32];

/// A leaf of the transfer tree.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct TreeLeaf {
    /// The height of the block when the note was inserted in the tree.
    pub block_height: u64,
    /// The note inserted in the tree.
    pub note: Note,
}

/// Send value to a contract transparently.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Stct {
    /// Module to send the value to.
    pub module: ModuleId,
    /// The value to send to the contract.
    pub value: u64,
    /// Proof of the `STCT` circuit.
    pub proof: Vec<u8>,
}

/// Withdraw value from a contract transparently.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Wfct {
    ///     The value to withdraw
    pub value: u64,
    /// The note to withdraw transparently to
    pub note: Note,
    /// A proof of the `WFCT` circuit.
    pub proof: Vec<u8>,
}

/// Withdraw value from a contract transparently.
/// Note is passed in a raw form.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct WfctRaw {
    ///     The value to withdraw
    pub value: u64,
    /// The note to withdraw transparently to
    pub note: Vec<u8>,
    /// A proof of the `WFCT` circuit.
    pub proof: Vec<u8>,
}

/// Withdraw value from the calling contract to another contract.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Wfctc {
    /// The contract to transfer value to.
    pub module: ModuleId,
    /// The value to transfer.
    pub value: u64,
}

/// Mint value to a stealth address.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Mint {
    /// The address to mint to.
    pub address: StealthAddress,
    /// The value to mint to the address.
    pub value: u64,
    /// A nonce to prevent replay.
    pub nonce: BlsScalar,
}

///
/// Events

#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub enum EconomicResult {
    /// Contract's charge has been successfully applied, contract will earn
    /// fee.
    ChargeApplied,
    /// Contract's allowance has been successfully applied, contract will pay
    /// for gas.
    AllowanceApplied,
    /// Contract's charge was not sufficient as it was smaller that the actual
    /// cost of the call.
    ChargeNotSufficient,
    /// Contract's allowance was not sufficient to cover the actual cost of the
    /// call.
    AllowanceNotSufficient,
    /// Contract's balance was not sufficient to pay for the call.
    BalanceNotSufficient,
}

/// Event emitted after economic operation is performed.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct EconomicEvent {
    /// Module id which is relevant to the event.
    pub module: ModuleId,
    /// Value of the relevant operation.
    pub value: u64,
    /// Result of the relevant operation.
    pub result: EconomicResult,
}

// Copyright (C) Quantum Fusion Network, 2025.
// Copyright (C) Parity Technologies (UK) Ltd., until 2025.
// SPDX-License-Identifier: Apache-2.0

//! Primitives for SPIN.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use codec::{Codec, Decode, Encode};
use sp_runtime::ConsensusEngineId;

pub mod digests;
pub mod inherents;

pub mod sr25519 {
	mod app_sr25519 {
		use sp_application_crypto::{app_crypto, key_types::AURA, sr25519};
		app_crypto!(sr25519, AURA);
	}

	sp_application_crypto::with_pair! {
		/// A SPIN authority keypair using S/R 25519 as its crypto.
		pub type AuthorityPair = app_sr25519::Pair;
	}

	/// A SPIN authority signature using S/R 25519 as its crypto.
	pub type AuthoritySignature = app_sr25519::Signature;

	/// A SPIN authority identifier using S/R 25519 as its crypto.
	pub type AuthorityId = app_sr25519::Public;
}

pub mod ed25519 {
	mod app_ed25519 {
		use sp_application_crypto::{app_crypto, ed25519, key_types::AURA};
		app_crypto!(ed25519, AURA);
	}

	sp_application_crypto::with_pair! {
		/// A SPIN authority keypair using Ed25519 as its crypto.
		pub type AuthorityPair = app_ed25519::Pair;
	}

	/// A SPIN authority signature using Ed25519 as its crypto.
	pub type AuthoritySignature = app_ed25519::Signature;

	/// A SPIN authority identifier using Ed25519 as its crypto.
	pub type AuthorityId = app_ed25519::Public;
}

pub use sp_consensus_slots::{Slot, SlotDuration};

/// The `ConsensusEngineId` of SPIN.
pub const SPIN_ENGINE_ID: ConsensusEngineId = *b"spin";

/// The index of an authority.
pub type AuthorityIndex = u32;

/// The length of the session.
pub type SessionLength = u32;

/// Auxilary data for SPIN
pub type SpinAuxData<A> = (Vec<A>, SessionLength);

/// An consensus log item for SPIN.
#[derive(Decode, Encode)]
pub enum ConsensusLog<AuthorityId: Codec> {
	/// The authorities have changed.
	#[codec(index = 1)]
	AuthoritiesChange(Vec<AuthorityId>),
	/// Disable the authority with given index.
	#[codec(index = 2)]
	OnDisabled(AuthorityIndex),
}

sp_api::decl_runtime_apis! {
	/// API necessary for block authorship with SPIN.
	pub trait SpinApi<AuthorityId: Codec> {
		/// Returns the slot duration for SPIN.
		///
		/// Currently, only the value provided by this type at genesis will be used.
		fn slot_duration() -> SlotDuration;

		/// Return the current set of authorities.
		fn aux_data() -> SpinAuxData<AuthorityId>;
	}
}

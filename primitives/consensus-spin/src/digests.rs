// Copyright (C) Quantum Fusion Network, 2025.
// Copyright (C) Parity Technologies (UK) Ltd., until 2025.
// SPDX-License-Identifier: Apache-2.0

//! SPIN digests.
//!
//! This implements the digests for SPIN, to allow the private
//! `CompatibleDigestItem` trait to appear in public interfaces.

use crate::SPIN_ENGINE_ID;
use codec::{Codec, Encode};
use sp_consensus_slots::Slot;
use sp_runtime::generic::DigestItem;

/// A digest item which is usable with spin consensus.
pub trait CompatibleDigestItem<Signature>: Sized {
	/// Construct a digest item which contains a signature on the hash.
	fn spin_seal(signature: Signature) -> Self;

	/// If this item is an SPIN seal, return the signature.
	fn as_spin_seal(&self) -> Option<Signature>;

	/// Construct a digest item which contains the slot number
	fn spin_pre_digest(slot: Slot) -> Self;

	/// If this item is an SPIN pre-digest, return the slot number
	fn as_spin_pre_digest(&self) -> Option<Slot>;
}

impl<Signature> CompatibleDigestItem<Signature> for DigestItem
where
	Signature: Codec,
{
	fn spin_seal(signature: Signature) -> Self {
		DigestItem::Seal(SPIN_ENGINE_ID, signature.encode())
	}

	fn as_spin_seal(&self) -> Option<Signature> {
		self.seal_try_to(&SPIN_ENGINE_ID)
	}

	fn spin_pre_digest(slot: Slot) -> Self {
		DigestItem::PreRuntime(SPIN_ENGINE_ID, slot.encode())
	}

	fn as_spin_pre_digest(&self) -> Option<Slot> {
		self.pre_runtime_try_to(&SPIN_ENGINE_ID)
	}
}

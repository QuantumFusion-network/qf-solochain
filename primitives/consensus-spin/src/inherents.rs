// Copyright (C) QF Network, 2025.
// Copyright (C) Parity Technologies (UK) Ltd., until 2025.
// SPDX-License-Identifier: Apache-2.0

/// Contains the inherents for the SPIN module
use sp_inherents::{Error, InherentData, InherentIdentifier};

/// The SPIN inherent identifier.
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"spinslot";

/// The type of the SPIN inherent.
pub type InherentType = sp_consensus_slots::Slot;

/// Auxiliary trait to extract SPIN inherent data.
pub trait SpinInherentData {
	/// Get SPIN inherent data.
	fn spin_inherent_data(&self) -> Result<Option<InherentType>, Error>;
	/// Replace SPIN inherent data.
	fn spin_replace_inherent_data(&mut self, new: InherentType);
}

impl SpinInherentData for InherentData {
	fn spin_inherent_data(&self) -> Result<Option<InherentType>, Error> {
		self.get_data(&INHERENT_IDENTIFIER)
	}

	fn spin_replace_inherent_data(&mut self, new: InherentType) {
		self.replace_data(INHERENT_IDENTIFIER, &new);
	}
}

/// Provides the slot duration inherent data for `SPIN`.
// TODO: Remove in the future. https://github.com/paritytech/substrate/issues/8029
#[cfg(feature = "std")]
pub struct InherentDataProvider {
	slot: InherentType,
}

#[cfg(feature = "std")]
impl InherentDataProvider {
	/// Create a new instance with the given slot.
	pub fn new(slot: InherentType) -> Self {
		Self { slot }
	}

	/// Creates the inherent data provider by calculating the slot from the
	/// given `timestamp` and `duration`.
	pub fn from_timestamp_and_slot_duration(
		timestamp: sp_timestamp::Timestamp,
		slot_duration: sp_consensus_slots::SlotDuration,
	) -> Self {
		let slot = InherentType::from_timestamp(timestamp, slot_duration);

		Self { slot }
	}
}

#[cfg(feature = "std")]
impl core::ops::Deref for InherentDataProvider {
	type Target = InherentType;

	fn deref(&self) -> &Self::Target {
		&self.slot
	}
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for InherentDataProvider {
	async fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), Error> {
		inherent_data.put_data(INHERENT_IDENTIFIER, &self.slot)
	}

	async fn try_handle_error(
		&self,
		_: &InherentIdentifier,
		_: &[u8],
	) -> Option<Result<(), Error>> {
		// There is no error anymore
		None
	}
}

use codec::Encode;
use cumulus_primitives_core::{ParaId, relay_chain::CoreIndex};
use sp_io::hashing::{blake2_128, twox_64, twox_256};
use sp_runtime::{KeyTypeId, Vec};

/// OnDemandAssignmentProvider QueueStatus
pub const QUEUE_STATUS: &[u8] =
	&hex_literal::hex!["8f32430b49607f8d60bfd3a003ddf4b58bf29330833ea7904c7209f4ce9d917a"];

/// Configuration ActiveConfig
pub const ACTIVE_CONFIG: &[u8] =
	&hex_literal::hex!["06de3d8a54d27e44a9d5ce189618f22db4b49d95320d9021994c850f25b8e385"];

pub const PARAS_PARA_LIFECYCLES: &[u8] =
	&hex_literal::hex!["cd710b30bd2eab0352ddcc26417aa194281e0bfde17b36573208a06cb5cfba6b"];

pub const CORE_DESCRIPTORS: &[u8] =
	&hex_literal::hex!["638595eebaa445ce03a13547bece90e704e6ac775a3245623103ffec2cb2c92f"];

pub const SYSTEM_ACCOUNT: &[u8] =
	&hex_literal::hex!["26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9"];

pub const EVENTS: &[u8] =
	&hex_literal::hex!["26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7"];

pub const SESSION_KEY_OWNER: &[u8] =
	&hex_literal::hex!["cec5070d609dd3497f72bde07fc96ba0726380404683fc89e8233450c8aa1950"];

pub fn account<AccountId: Encode>(acc: AccountId) -> Vec<u8> {
	acc.using_encoded(|acc: &[u8]| {
		SYSTEM_ACCOUNT
			.iter()
			.chain(blake2_128(acc).iter())
			.chain(acc.iter())
			.cloned()
			.collect()
	})
}

pub fn session_key_owner(key_type: KeyTypeId, acc: Vec<u8>) -> Vec<u8> {
	(key_type, acc).using_encoded(|encoded: &[u8]| {
		PARAS_PARA_LIFECYCLES
			.iter()
			.chain(twox_64(encoded).iter())
			.chain(encoded.iter())
			.cloned()
			.collect()
	})
}

/// Returns the storage key for a specific core descriptor.
pub fn core_descriptor(core_index: CoreIndex) -> Vec<u8> {
	core_index.using_encoded(|core_index: &[u8]| {
		CORE_DESCRIPTORS.iter().chain(twox_256(core_index).iter()).cloned().collect()
	})
}

/// Returns the storage key for accessing the parachain lifecycle.
pub fn para_lifecycle(para_id: ParaId) -> Vec<u8> {
	para_id.using_encoded(|para_id: &[u8]| {
		PARAS_PARA_LIFECYCLES
			.iter()
			.chain(twox_64(para_id).iter())
			.chain(para_id.iter())
			.cloned()
			.collect()
	})
}

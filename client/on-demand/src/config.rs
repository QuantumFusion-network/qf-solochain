//! This file contains all the configuration related traits.

use crate::RelayBlockNumber;
use codec::Codec;
use cumulus_relay_chain_interface::RelayChainInterface;
use order_primitives::ThresholdParameterT;
use sc_client_api::UsageProvider;
use sc_service::Arc;
use sc_transaction_pool_api::MaintainedTransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_application_crypto::RuntimeAppPublic;
use sp_core::H256;
use sp_runtime::traits::{AtLeast32BitUnsigned, Block as BlockT, Debug, MaybeDisplay, Member};
use std::{error::Error, fmt::Display};

pub trait OnDemandConfig {
	/// Custom order placement criteria.
	type OrderPlacementCriteria: OrderCriteria;

	/// Author identifier.
	type AuthorPub: Member + RuntimeAppPublic + Display + Send + Codec;

	/// Block type.
	type Block: BlockT<Hash = H256>;

	/// Relay chain.
	type R: RelayChainInterface + Clone;

	/// Parachain.
	type P: ProvideRuntimeApi<Self::Block> + UsageProvider<Self::Block> + Send + Sync;

	/// Extrinsic pool.
	type ExPool: MaintainedTransactionPool<Block = Self::Block, Hash = <Self::Block as BlockT>::Hash>
		+ 'static;

	/// Balance type.
	type Balance: Codec
		+ MaybeDisplay
		+ 'static
		+ Debug
		+ Send
		+ Into<u128>
		+ AtLeast32BitUnsigned
		+ Copy
		+ From<u128>;

	/// On-demand pallet threshold parameter.
	type ThresholdParameter: ThresholdParameterT;
}

pub trait OrderCriteria {
	type Block: BlockT;
	type P: ProvideRuntimeApi<Self::Block> + UsageProvider<Self::Block>;
	type ExPool: MaintainedTransactionPool<Block = Self::Block, Hash = <Self::Block as BlockT>::Hash>
		+ 'static;

	/// Returns true or false depending on whether an order should be placed.
	fn should_place_order(
		parachain: &Self::P,
		transaction_pool: Arc<Self::ExPool>,
		height: RelayBlockNumber,
	) -> bool;
}

pub fn order_placer<Config: OnDemandConfig>(
	authorities: Vec<Config::AuthorPub>,
	relay_height: RelayBlockNumber,
	slot_width: u32,
) -> Result<Config::AuthorPub, Box<dyn Error>> {
	// Taken from: https://github.com/paritytech/polkadot-sdk/issues/1487
	let indx = current_slot(relay_height, slot_width) as u128 % authorities.len() as u128;
	let author = authorities.get(indx as usize).ok_or("Failed to get selected collator")?;

	Ok(author.clone())
}

pub fn current_slot(relay_height: RelayBlockNumber, slot_width: u32) -> u32 {
	let slot = relay_height >> slot_width;
	slot
}

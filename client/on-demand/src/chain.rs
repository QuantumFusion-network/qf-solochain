//! This file contains all the chain related interaction functions.

use crate::{
	chain::polkadot::{
		on_demand_assignment_provider::storage::types::queue_status::QueueStatus,
		runtime_types::{
			pallet_broker::coretime_interface::CoreAssignment,
			polkadot_parachain_primitives::primitives::Id,
			polkadot_runtime_parachains::assigner_coretime::CoreDescriptor,
		},
	},
	RelayBlockNumber,
};
use codec::{Codec, Decode};
use cumulus_primitives_core::{relay_chain::CoreIndex, ParaId};
use cumulus_relay_chain_interface::RelayChainInterface;
use order_primitives::well_known_keys::{
	core_descriptor, para_lifecycle, ACTIVE_CONFIG, QUEUE_STATUS,
};
use polkadot_runtime_parachains::{configuration::HostConfiguration, ParaLifecycle};
use sp_application_crypto::AppCrypto;
use sp_core::{ByteArray, H256};
use sp_keystore::KeystorePtr;
use sp_runtime::{
	traits::{IdentifyAccount, MaybeDisplay, Verify},
	MultiSignature as SpMultiSignature, SaturatedConversion,
};
use std::{error::Error, fmt::Debug};
use subxt::{
	backend::{legacy::LegacyRpcMethods, rpc::RpcClient},
	config::DefaultExtrinsicParamsBuilder as Params,
	tx::Signer,
	utils::MultiSignature,
	Config, OnlineClient, PolkadotConfig,
};

#[subxt::subxt(runtime_metadata_path = "./artifacts/metadata.scale")]
pub mod polkadot {}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Signature(pub [u8; 64]);

impl From<Signature> for MultiSignature {
	fn from(value: Signature) -> Self {
		MultiSignature::Sr25519(value.0)
	}
}

pub struct SignerKeystore<T: Config> {
	/// Account ID
	account_id: T::AccountId,
	/// Keystore of node
	keystore: KeystorePtr,
}

impl<T> SignerKeystore<T>
where
	T: Config,
	T::AccountId: From<[u8; 32]>,
{
	pub fn new(keystore: KeystorePtr) -> Self {
		let pub_key =
			keystore.sr25519_public_keys(sp_consensus_aura::sr25519::AuthorityPair::ID)[0];

		let binding = <SpMultiSignature as Verify>::Signer::from(pub_key).into_account().clone();

		let account_id = binding.as_slice();
		let mut r = [0u8; 32];
		r.copy_from_slice(account_id);
		let acc = T::AccountId::try_from(r).ok().expect("Failed to get account from keystore");
		Self { account_id: acc.clone(), keystore }
	}
}
impl<T> Signer<T> for SignerKeystore<T>
where
	T: Config,
	T::AccountId: From<[u8; 32]>,
	T::Signature: From<Signature>,
{
	fn account_id(&self) -> T::AccountId {
		self.account_id.clone()
	}

	fn address(&self) -> T::Address {
		self.account_id.clone().into()
	}

	/// Use aura's key to sign
	fn sign(&self, signer_payload: &[u8]) -> T::Signature {
		let pub_key =
			self.keystore.sr25519_public_keys(sp_consensus_aura::sr25519::AuthorityPair::ID)[0];

		let signature = self
			.keystore
			.sr25519_sign(sp_consensus_aura::sr25519::AuthorityPair::ID, &pub_key, signer_payload)
			.unwrap()
			.unwrap();

		Signature(signature.0).into()
	}
}

/// Submits order to an rpc node.
pub async fn submit_order(
	url: &str,
	para_id: ParaId,
	relay_height: RelayBlockNumber,
	slot_width: u32,
	max_amount: u128,
	keystore: KeystorePtr,
) -> Result<(), Box<dyn Error>> {
	let client = OnlineClient::<PolkadotConfig>::from_url(url).await?;
	let rpc_client = RpcClient::from_url(url).await?;

	let rpc = LegacyRpcMethods::<PolkadotConfig>::new(rpc_client.clone());

	let place_order = polkadot::tx()
		.on_demand_assignment_provider()
		.place_order_allow_death(max_amount, Id(para_id.into()));

	let signer_keystore = SignerKeystore::<PolkadotConfig>::new(keystore.clone());
	let first_block_in_slot = crate::config::current_slot(relay_height, slot_width) << slot_width;

	let block_hash = rpc
		.chain_get_block_hash(Some(first_block_in_slot.into()))
		.await?
		.ok_or("Failed to get hash of slot starting block")?;

	let block = client.blocks().at(block_hash).await?;

	let mortality = 1u32 << slot_width;

	let tx_params = Params::new().mortal(block.header(), mortality.into()).build();
	let submit_result =
		client.tx().sign_and_submit(&place_order, &signer_keystore, tx_params).await;

	log::info!("submit_result: {:?}", submit_result);
	submit_result?;

	Ok(())
}

/// Get the spot price from the relay chain.
pub async fn get_spot_price<Balance>(
	relay_chain: impl RelayChainInterface + Clone,
	hash: H256,
) -> Option<Balance>
where
	Balance: Codec + MaybeDisplay + 'static + Debug + From<u128>,
{
	let queue_status_storage = relay_chain.get_storage_by_key(hash, QUEUE_STATUS).await.ok()?;
	let queue_status = queue_status_storage
		.map(|raw| <QueueStatus>::decode(&mut &raw[..]))
		.transpose()
		.ok()?;

	let active_config_storage = relay_chain.get_storage_by_key(hash, ACTIVE_CONFIG).await.ok()?;
	let active_config = active_config_storage
		.map(|raw| <HostConfiguration<u32>>::decode(&mut &raw[..]))
		.transpose()
		.ok()?;

	if queue_status.is_some() && active_config.is_some() {
		let traffic = queue_status.expect("We ensured spot_traffic is Some above; qed").traffic;
		let active_config = active_config.expect("We ensured active_config is Some above; qed");
		let spot_price = traffic.0.saturating_mul(
			active_config.scheduler_params.on_demand_base_fee.saturated_into::<u128>(),
		);
		Some(Balance::from(spot_price))
	} else {
		None
	}
}

/// Is this a parathread?
pub async fn is_parathread(
	relay_chain: &(impl RelayChainInterface + Clone),
	r_hash: H256,
	para_id: ParaId,
) -> Result<bool, Box<dyn Error>> {
	let para_lifecycle_storage = relay_chain
		.get_storage_by_key(r_hash, para_lifecycle(para_id).as_slice())
		.await?;
	let para_lifecycle = para_lifecycle_storage
		.map(|raw| <ParaLifecycle>::decode(&mut &raw[..]))
		.transpose()?;

	let is_parathread = para_lifecycle == Some(ParaLifecycle::Parathread);
	Ok(is_parathread)
}

/// Checks if there are any cores allocated to on-demand.
pub async fn on_demand_cores_available(
	relay_chain: &(impl RelayChainInterface + Clone),
	hash: H256,
) -> Option<bool> {
	let active_config_storage = relay_chain.get_storage_by_key(hash, ACTIVE_CONFIG).await.ok()?;

	let active_config = active_config_storage
		.map(|raw| <HostConfiguration<u32>>::decode(&mut &raw[..]))
		.transpose()
		.ok()?;

	let active_config = active_config?;

	for core in 0..active_config.scheduler_params.num_cores {
		let core_descriptor_storage = relay_chain
			.get_storage_by_key(hash, &core_descriptor(CoreIndex(core)))
			.await
			.ok()?;

		let core_descriptor = core_descriptor_storage
			.map(|raw| <CoreDescriptor<u32>>::decode(&mut &raw[..]))
			.transpose()
			.ok()?;

		if let Some(descriptor) = core_descriptor {
			if let Some(work) = descriptor.current_work {
				let available_core = work
					.assignments
					.into_iter()
					.position(|assignment| match assignment {
						(CoreAssignment::Pool, _) => true,
						_ => false,
					})
					.is_some();

				return Some(available_core);
			}
		}
	}

	Some(false)
}

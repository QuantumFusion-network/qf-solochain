use crate::{EVENTS, InherentData, ON_DEMAND_INHERENT_IDENTIFIER, OrderInherentData};
use cumulus_primitives_core::{ParaId, relay_chain::BlockId};
use cumulus_relay_chain_interface::{PHash, RelayChainInterface};
use sp_core::H256;
use sp_runtime::traits::Header;

const LOG_TARGET: &str = "order-inherent";

impl OrderInherentData {
	pub async fn create_at(
		relay_chain_interface: &impl RelayChainInterface,
		relay_block_hash: Option<H256>,
		para_id: ParaId,
	) -> Option<Self> {
		let Some(hash) = relay_block_hash else {
			return Some(OrderInherentData { data: None });
		};

		let header = relay_chain_interface
			.header(BlockId::Hash(hash))
			.await
			.map_err(|e| {
				log::error!(
					target: LOG_TARGET,
					"Failed to fetch relay chain header: {}, Error: {:?}",
					hash,
					e
				);
			})
			.ok()??;

		let relay_storage_proof =
			collect_relay_storage_proof(relay_chain_interface, header.hash()).await?;

		log::info!(
			target: LOG_TARGET,
			"Submitting inherent data"
		);

		Some(OrderInherentData {
			data: Some(InherentData {
				relay_storage_proof: relay_storage_proof.clone(),
				para_id,
				relay_state_root: *header.state_root(),
				relay_height: *header.number(),
			}),
		})
	}
}

#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for OrderInherentData {
	async fn provide_inherent_data(
		&self,
		inherent_data: &mut sp_inherents::InherentData,
	) -> Result<(), sp_inherents::Error> {
		inherent_data.put_data(ON_DEMAND_INHERENT_IDENTIFIER, &self)
	}

	async fn try_handle_error(
		&self,
		_: &sp_inherents::InherentIdentifier,
		_: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		None
	}
}

async fn collect_relay_storage_proof(
	relay_chain: &impl RelayChainInterface,
	relay_parent: PHash,
) -> Option<sp_state_machine::StorageProof> {
	let mut relevant_keys = Vec::new();
	// Get storage proof for events at a specific block.
	relevant_keys.push(EVENTS.to_vec());

	let relay_storage_proof = relay_chain.prove_read(relay_parent, &relevant_keys).await;
	match relay_storage_proof {
		Ok(proof) => Some(proof),
		Err(err) => {
			log::info!("RelayChainError:{:?}", err);
			None
		},
	}
}

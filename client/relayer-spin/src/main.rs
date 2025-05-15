use clap::Parser;
use subxt::{
	OnlineClient, PolkadotConfig,
	backend::rpc::RpcClient,
	ext::{
		codec::DecodeAll,
		subxt_rpcs::client::{RpcParams, RpcSubscription},
	},
};
use subxt_signer::sr25519::dev;

// Generate an interface that we can use from the node's metadata.
#[subxt::subxt(runtime_metadata_path = "./parachain_metadata.scale")]
pub mod parachain_metadata {}

use parachain_metadata::runtime_types::pallet_spin_polkadot::pallet::AliveMessageProof;

#[derive(Parser, Debug)]
struct Cli {
	#[arg(short, long)]
	fastchain_url: String,
	#[arg(short, long)]
	parachain_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let cli = Cli::parse();

	let fastchain_rpc_client = RpcClient::from_url(&cli.fastchain_url).await?;
	let parachain_api = OnlineClient::<PolkadotConfig>::from_url(&cli.parachain_url).await?;

	let mut justifications_sub: RpcSubscription<String> = fastchain_rpc_client
		.subscribe(
			"grandpa_subscribeJustifications",
			RpcParams::default(),
			"grandpa_unsubscribeJustifications",
		)
		.await?;

	while let Some(justification) = justifications_sub.next().await {
		match justification {
			Ok(data) => {
				eprintln!("Received justification: {:?}", &data[..20]);

				let data = hex::decode(&data[2..]).unwrap();

				let res = try_submit_fastchain_finality_proof_message(&parachain_api, data).await;
				eprintln!("Submitted alive message. Result: {res:?}");
			},
			Err(err) => eprintln!("Error receiving justification: {err:?}"),
		}
	}

	Ok(())
}

async fn try_submit_fastchain_finality_proof_message(
	parachain_api: &OnlineClient<PolkadotConfig>,
	data: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
	let proof = AliveMessageProof::decode_all(&mut data.as_slice()).unwrap();

	let tx = parachain_metadata::tx().pallet_spin_polkadot().submit_fastchain_finality_proof_message(proof);

	let from = dev::alice();
	let submitted_tx = parachain_api.tx().sign_and_submit_then_watch_default(&tx, &from).await?;
	let tx_in_block = submitted_tx.wait_for_finalized().await?;

	let events = tx_in_block.fetch_events().await.unwrap();
    let all_events = events.all_events_in_block();
	eprintln!("TX events: {all_events:?}");
	let block_hash = tx_in_block.block_hash();
	eprintln!("TX block hash: {block_hash:?}");

	Ok(())
}

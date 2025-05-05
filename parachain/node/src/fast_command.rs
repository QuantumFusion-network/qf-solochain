use crate::{
	// benchmarking::{RemarkBuilder, TransferKeepAliveBuilder, inherent_benchmark_data},
	chain_spec,
	cli::{Cli, Subcommand},
	service,
};
use frame_benchmarking_cli::{BenchmarkCmd, ExtrinsicFactory, SUBSTRATE_REFERENCE_HARDWARE};
use qf_runtime::{Block, EXISTENTIAL_DEPOSIT};
use sc_cli::SubstrateCli;
use sc_service::PartialComponents;
use sp_keyring::Sr25519Keyring;

// impl SubstrateCli for Cli {
// 	fn impl_name() -> String {
// 		"Quantum Fusion QF Node".into()
// 	}

// 	fn impl_version() -> String {
// 		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
// 	}

// 	fn description() -> String {
// 		env!("CARGO_PKG_DESCRIPTION").into()
// 	}

// 	fn author() -> String {
// 		env!("CARGO_PKG_AUTHORS").into()
// 	}

// 	fn support_url() -> String {
// 		"support.anonymous.an".into()
// 	}

// 	fn copyright_start_year() -> i32 {
// 		2017
// 	}

// 	fn load_spec(&self, id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
// 		Ok(match id {
// 			"dev" => Box::new(chain_spec::development_config()?),
// 			"" | "local" => Box::new(chain_spec::local_testnet_config()?),
// 			"qf-devnet" => Box::new(chain_spec::qf_devnet_config()?),
// 			"qf-testnet" => Box::new(chain_spec::qf_testnet_config()?),
// 			path =>
// 				Box::new(chain_spec::ChainSpec::from_json_file(std::path::PathBuf::from(path))?),
// 		})
// 	}
// }

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
	// setup_logging(3).expect("failed to initialize logging.");

	log::info!("MyProgram v0.0.1 starting up!");
	let cli = Cli::from_args();
	let fargs = cli.fast_chain_args.clone();

	let runner = cli.create_runner(&cli.run.normalize())?;
	runner.run_node_until_exit(|config| async move {
		match config.network.network_backend {
			sc_network::config::NetworkBackendType::Libp2p => crate::fast_service::new_full::<
				sc_network::NetworkWorker<
					qf_runtime::opaque::Block,
					<qf_runtime::opaque::Block as sp_runtime::traits::Block>::Hash,
				>,
			>(config)
			.await.map_err(Into::into),
			sc_network::config::NetworkBackendType::Litep2p =>
				crate::fast_service::new_full::<sc_network::Litep2pNetworkBackend>(config)
					.await.map_err(Into::into),
		}
	})
		
}

use polkadot_sdk::{sp_tracing::tracing_subscriber::field::debug, *};

use cumulus_client_service::storage_proof_size::HostFunctions as ReclaimHostFunctions;
use cumulus_primitives_core::ParaId;
use frame_benchmarking_cli::{BenchmarkCmd, SUBSTRATE_REFERENCE_HARDWARE};
use log::{info, debug};
use qf_parachain_runtime::Block;
use sc_cli::{
	ChainSpec, CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams,
	NetworkParams, Result, RpcEndpoint, SharedParams, SubstrateCli, build_runtime, Signals,
	Runner
};
use sc_service::config::{BasePath, PrometheusConfig};

use crate::{
	chain_spec,
	cli::{Cli, RelayChainCli, Subcommand, FastChainCli},
	service::new_partial,
};

pub use sc_tracing::logging::LoggerBuilder;
use sc_service::Configuration;

fn load_spec(id: &str) -> std::result::Result<Box<dyn ChainSpec>, String> {
	info!("Loading parachain spec: {id}");
	Ok(match id {
		"dev" => Box::new(chain_spec::development_chain_spec()),
		"template-rococo" => Box::new(chain_spec::local_chain_spec()),
		"" | "local" => Box::new(chain_spec::local_chain_spec()),
		path => Box::new(chain_spec::ChainSpec::from_json_file(std::path::PathBuf::from(path))?),
	})
}

fn load_fast_spec(id: &str) -> std::result::Result<Box<dyn ChainSpec>, String> {
	info!("Loading fastchain spec: {id}");
	Ok(match id {
		"dev" => Box::new(chain_spec::fast_development_chain_spec()),
		"template-rococo" => Box::new(chain_spec::fast_local_chain_spec()),
		"" | "local" => Box::new(chain_spec::fast_local_chain_spec()),
		path => Box::new(chain_spec::FastChainSpec::from_json_file(std::path::PathBuf::from(path))?),
	})
}

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Parachain Collator Template".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		format!(
			"Parachain Collator Template\n\nThe command-line arguments provided first will be \
		passed to the parachain node, while the arguments provided after -- will be passed \
		to the relay chain node.\n\n\
		{} <parachain-args> -- <relay-chain-args>",
			Self::executable_name()
		)
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/paritytech/polkadot-sdk/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2020
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		load_spec(id)
	}
}

impl SubstrateCli for RelayChainCli {
	fn impl_name() -> String {
		"Parachain Collator Template".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		format!(
			"Parachain Collator Template\n\nThe command-line arguments provided first will be \
		passed to the parachain node, while the arguments provided after -- will be passed \
		to the relay chain node.\n\n\
		{} <parachain-args> -- <relay-chain-args>",
			Self::executable_name()
		)
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/paritytech/polkadot-sdk/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2020
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		polkadot_cli::Cli::from_iter([RelayChainCli::executable_name()].iter()).load_spec(id)
	}
}

impl SubstrateCli for FastChainCli {
	fn impl_name() -> String {
		"QF Fastchain".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		format!(
			"Parachain Collator \n\nThe command-line arguments provided first will be \
		passed to the parachain node, while the arguments provided after -- will be passed \
		to the relay chain node and after -- will be passed to the fastchain node.\n\n\
		{} <parachain-args> -- <relay-chain-args> -- <fastchain-args>",
			Self::executable_name()
		)
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/QuantumFusion-network/qf-solochain/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2025
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		load_fast_spec(id)
	}

	fn create_runner_with_logger_hook<
		T: sc_cli::CliConfiguration<DVC>,
		DVC: sc_cli::DefaultConfigurationValues,
		F,
	>(
		&self,
		command: &T,
		logger_hook: F,
	) -> Result<Runner<Self>>
	where
		F: FnOnce(&mut LoggerBuilder, &Configuration),
	{
		let tokio_runtime = build_runtime()?;

		let signals = tokio_runtime.block_on(async { Signals::capture() })?;

		let config = command.create_configuration(self, tokio_runtime.handle().clone())?;

		if self.with_logger.unwrap_or(false) {
			command.init(&Self::support_url(), &Self::impl_version(), |logger_builder| {
				logger_hook(logger_builder, &config)
			})?;
		}

		Runner::new(config, tokio_runtime, signals)
	}
}

macro_rules! construct_async_run {
	(|$components:ident, $cli:ident, $cmd:ident, $config:ident| $( $code:tt )* ) => {{
		let runner = $cli.create_runner($cmd)?;
		runner.async_run(|$config| {
			let $components = new_partial(&$config)?;
			let task_manager = $components.task_manager;
			{ $( $code )* }.map(|v| (v, task_manager))
		})
	}}
}

/// Parse command line arguments into service configuration.
pub fn run() -> Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		},
		Some(Subcommand::CheckBlock(cmd)) => {
			construct_async_run!(|components, cli, cmd, config| {
				Ok(cmd.run(components.client, components.import_queue))
			})
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			construct_async_run!(|components, cli, cmd, config| {
				Ok(cmd.run(components.client, config.database))
			})
		},
		Some(Subcommand::ExportState(cmd)) => {
			construct_async_run!(|components, cli, cmd, config| {
				Ok(cmd.run(components.client, config.chain_spec))
			})
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			construct_async_run!(|components, cli, cmd, config| {
				Ok(cmd.run(components.client, components.import_queue))
			})
		},
		Some(Subcommand::Revert(cmd)) => {
			construct_async_run!(|components, cli, cmd, config| {
				Ok(cmd.run(components.client, components.backend, None))
			})
		},
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.sync_run(|config| {
				let polkadot_cli = RelayChainCli::new(
					&config,
					[RelayChainCli::executable_name()].iter().chain(cli.relay_chain_args.iter()),
				);

				let polkadot_config = SubstrateCli::create_configuration(
					&polkadot_cli,
					&polkadot_cli,
					config.tokio_handle.clone(),
				)
				.map_err(|err| format!("Relay chain argument error: {}", err))?;

				cmd.run(config, polkadot_config)
			})
		},
		Some(Subcommand::ExportGenesisHead(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| {
				let partials = new_partial(&config)?;

				cmd.run(partials.client)
			})
		},
		Some(Subcommand::ExportGenesisWasm(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|_config| {
				let spec = cli.load_spec(&cmd.shared_params.chain.clone().unwrap_or_default())?;
				cmd.run(&*spec)
			})
		},
		Some(Subcommand::Benchmark(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			// Switch on the concrete benchmark sub-command-
			match cmd {
				BenchmarkCmd::Pallet(cmd) =>
					if cfg!(feature = "runtime-benchmarks") {
						runner.sync_run(|config| cmd.run_with_spec::<sp_runtime::traits::HashingFor<Block>, ReclaimHostFunctions>(Some(config.chain_spec)))
					} else {
						Err("Benchmarking wasn't enabled when building the node. \
					You can enable it with `--features runtime-benchmarks`."
							.into())
					},
				BenchmarkCmd::Block(cmd) => runner.sync_run(|config| {
					let partials = new_partial(&config)?;
					cmd.run(partials.client)
				}),
				#[cfg(not(feature = "runtime-benchmarks"))]
				BenchmarkCmd::Storage(_) => Err(sc_cli::Error::Input(
					"Compile with --features=runtime-benchmarks \
						to enable storage benchmarks."
						.into(),
				)),
				#[cfg(feature = "runtime-benchmarks")]
				BenchmarkCmd::Storage(cmd) => runner.sync_run(|config| {
					let partials = new_partial(&config)?;
					let db = partials.backend.expose_db();
					let storage = partials.backend.expose_storage();
					cmd.run(config, partials.client.clone(), db, storage)
				}),
				BenchmarkCmd::Machine(cmd) =>
					runner.sync_run(|config| cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone())),
				// NOTE: this allows the Client to leniently implement
				// new benchmark commands without requiring a companion MR.
				#[allow(unreachable_patterns)]
				_ => Err("Benchmarking sub-command unsupported".into()),
			}
		},
		None => {

			// Extract necessary arguments before spawning the thread
			let fast_chain_args = cli.fast_chain_args.clone();
			let fast_chain_present: bool = !fast_chain_args.is_empty();
			let relay_chain_present: bool = !cli.relay_chain_args.is_empty();
			let mut fast_cli = FastChainCli::from_iter(&fast_chain_args);

			if fast_chain_present && !relay_chain_present {
				fast_cli.set_logger_flag();
				let runner = fast_cli.create_runner(&fast_cli.base)?;
				info!("Starting fast chain node...");
				let _ = runner.run_node_until_exit(|config| async move {
					match config.network.network_backend {
						sc_network::config::NetworkBackendType::Libp2p => crate::fast_service::new_full::<
							sc_network::NetworkWorker<
								qf_runtime::opaque::Block,
								<qf_runtime::opaque::Block as sp_runtime::traits::Block>::Hash,
							>,
						>(config).await
						.map_err(sc_cli::Error::Service),
						sc_network::config::NetworkBackendType::Litep2p => crate::fast_service::new_full::<
							sc_network::Litep2pNetworkBackend>(config).await
							.map_err(sc_cli::Error::Service),
					}
				});
				info!("Stop fast chain node...");
				return Ok(());
			} else if fast_chain_present {
				let _fast_thread = std::thread::spawn(move || {

					// Create a runner without initializing a new logger
					info!("Starting fast chain thread...");
					let runner = fast_cli.create_runner(&fast_cli.base).expect("Can't create the runner");

					let _ = runner.run_node_until_exit(|config| async move {

						let fast_cli = FastChainCli::new(
							&config,
							[FastChainCli::executable_name()].iter().chain(fast_chain_args.iter()),
						);
						let tokio_handle = config.tokio_handle.clone();
						let fast_config = SubstrateCli::create_configuration(&fast_cli, &fast_cli, tokio_handle)
							.map_err(|err| format!("Fast chain argument error: {}", err))?;
						match fast_config.network.network_backend {
							sc_network::config::NetworkBackendType::Libp2p => crate::fast_service::new_full::<
								sc_network::NetworkWorker<
									qf_runtime::opaque::Block,
									<qf_runtime::opaque::Block as sp_runtime::traits::Block>::Hash,
								>,>
								(fast_config).await
								.map_err(sc_cli::Error::Service),
							sc_network::config::NetworkBackendType::Litep2p =>
								crate::fast_service::new_full::<sc_network::Litep2pNetworkBackend>
								(fast_config).await
								.map_err(sc_cli::Error::Service),
						}
					});
				});
			};

			let runner = cli.create_runner(&cli.run.normalize())?;
			let collator_options = cli.run.collator_options();

			runner.run_node_until_exit(|config| async move {
				let hwbench = (!cli.no_hardware_benchmarks)
					.then(|| {
						config.database.path().map(|database_path| {
							let _ = std::fs::create_dir_all(database_path);
							sc_sysinfo::gather_hwbench(
								Some(database_path),
								&SUBSTRATE_REFERENCE_HARDWARE,
							)
						})
					})
					.flatten();

				let para_id = chain_spec::Extensions::try_get(&*config.chain_spec)
					.map(|e| e.para_id)
					.ok_or("Could not find parachain ID in chain-spec.")?;
				let fargs = cli.fast_chain_args.clone();
				info!("Fast chain args: {:?}", fargs);
				let args = cli.relay_chain_args.clone();
				info!("Relay chain args: {:?}", args);
				let polkadot_cli = RelayChainCli::new(
					&config,
					[RelayChainCli::executable_name()].iter().chain(cli.relay_chain_args.iter()),
				);

				let id = ParaId::from(para_id);

				let tokio_handle = config.tokio_handle.clone();

				let polkadot_config =
					SubstrateCli::create_configuration(&polkadot_cli, &polkadot_cli, tokio_handle)
						.map_err(|err| format!("Relay chain argument error: {}", err))?;

				info!("Is collating: {}", if config.role.is_authority() { "yes" } else { "no" });

				let para_srv = crate::service::start_parachain_node(
					config,
					polkadot_config,
					collator_options,
					id,
					hwbench,
				);

				para_srv.await.map(|r| r.0).map_err(Into::into)
			})
		},
	}
}

impl DefaultConfigurationValues for RelayChainCli {
	fn p2p_listen_port() -> u16 {
		30334
	}

	fn rpc_listen_port() -> u16 {
		9945
	}

	fn prometheus_listen_port() -> u16 {
		9616
	}
}

impl CliConfiguration<Self> for RelayChainCli {
	fn shared_params(&self) -> &SharedParams {
		self.base.base.shared_params()
	}

	fn import_params(&self) -> Option<&ImportParams> {
		self.base.base.import_params()
	}

	fn network_params(&self) -> Option<&NetworkParams> {
		self.base.base.network_params()
	}

	fn keystore_params(&self) -> Option<&KeystoreParams> {
		self.base.base.keystore_params()
	}

	fn base_path(&self) -> Result<Option<BasePath>> {
		Ok(self
			.shared_params()
			.base_path()?
			.or_else(|| self.base_path.clone().map(Into::into)))
	}

	fn rpc_addr(&self, default_listen_port: u16) -> Result<Option<Vec<RpcEndpoint>>> {
		self.base.base.rpc_addr(default_listen_port)
	}

	fn prometheus_config(
		&self,
		default_listen_port: u16,
		chain_spec: &Box<dyn ChainSpec>,
	) -> Result<Option<PrometheusConfig>> {
		self.base.base.prometheus_config(default_listen_port, chain_spec)
	}

	fn init<F>(&self, _support_url: &String, _impl_version: &String, _logger_hook: F) -> Result<()>
	where
		F: FnOnce(&mut sc_cli::LoggerBuilder),
	{
		unreachable!("PolkadotCli is never initialized; qed");
	}

	fn chain_id(&self, is_dev: bool) -> Result<String> {
		let chain_id = self.base.base.chain_id(is_dev)?;

		Ok(if chain_id.is_empty() { self.chain_id.clone().unwrap_or_default() } else { chain_id })
	}

	fn role(&self, is_dev: bool) -> Result<sc_service::Role> {
		self.base.base.role(is_dev)
	}

	fn transaction_pool(&self, is_dev: bool) -> Result<sc_service::config::TransactionPoolOptions> {
		self.base.base.transaction_pool(is_dev)
	}

	fn trie_cache_maximum_size(&self) -> Result<Option<usize>> {
		self.base.base.trie_cache_maximum_size()
	}

	fn rpc_methods(&self) -> Result<sc_service::config::RpcMethods> {
		self.base.base.rpc_methods()
	}

	fn rpc_max_connections(&self) -> Result<u32> {
		self.base.base.rpc_max_connections()
	}

	fn rpc_cors(&self, is_dev: bool) -> Result<Option<Vec<String>>> {
		self.base.base.rpc_cors(is_dev)
	}

	fn default_heap_pages(&self) -> Result<Option<u64>> {
		self.base.base.default_heap_pages()
	}

	fn force_authoring(&self) -> Result<bool> {
		self.base.base.force_authoring()
	}

	fn disable_grandpa(&self) -> Result<bool> {
		self.base.base.disable_grandpa()
	}

	fn max_runtime_instances(&self) -> Result<Option<usize>> {
		self.base.base.max_runtime_instances()
	}

	fn announce_block(&self) -> Result<bool> {
		self.base.base.announce_block()
	}

	fn telemetry_endpoints(
		&self,
		chain_spec: &Box<dyn ChainSpec>,
	) -> Result<Option<sc_telemetry::TelemetryEndpoints>> {
		self.base.base.telemetry_endpoints(chain_spec)
	}

	fn node_name(&self) -> Result<String> {
		self.base.base.node_name()
	}
}

impl DefaultConfigurationValues for FastChainCli {
	fn p2p_listen_port() -> u16 {
		30333
	}

	fn rpc_listen_port() -> u16 {
		9944
	}

	fn prometheus_listen_port() -> u16 {
		9615
	}
}

impl CliConfiguration<Self> for FastChainCli {
	fn shared_params(&self) -> &SharedParams {
		self.base.shared_params()
	}

	fn import_params(&self) -> Option<&ImportParams> {
		self.base.import_params()
	}

	fn network_params(&self) -> Option<&NetworkParams> {
		self.base.network_params()
	}

	fn keystore_params(&self) -> Option<&KeystoreParams> {
		self.base.keystore_params()
	}

	fn base_path(&self) -> Result<Option<BasePath>> {
		Ok(self
			.shared_params()
			.base_path()?
			.or_else(|| self.fast_base_path.clone().map(Into::into)))
	}

	fn rpc_addr(&self, default_listen_port: u16) -> Result<Option<Vec<RpcEndpoint>>> {
		self.base.rpc_addr(default_listen_port)
	}

	fn prometheus_config(
		&self,
		default_listen_port: u16,
		chain_spec: &Box<dyn ChainSpec>,
	) -> Result<Option<PrometheusConfig>> {
		self.base.prometheus_config(default_listen_port, chain_spec)
	}

	fn init<F>(&self, _support_url: &String, _impl_version: &String, _logger_hook: F) -> Result<()>
	where
		F: FnOnce(&mut sc_cli::LoggerBuilder),
	{
		unreachable!("PolkadotCli is never initialized; qed");
	}

	// fn chain_id(&self, is_dev: bool) -> Result<String> {
	// 	let chain_id = self.base.base.chain_id(is_dev)?;

	// 	Ok(if chain_id.is_empty() { self.chain_id.clone().unwrap_or_default() } else { chain_id })
	// }

	fn role(&self, is_dev: bool) -> Result<sc_service::Role> {
		self.base.role(is_dev)
	}

	fn transaction_pool(&self, is_dev: bool) -> Result<sc_service::config::TransactionPoolOptions> {
		self.base.transaction_pool(is_dev)
	}

	fn trie_cache_maximum_size(&self) -> Result<Option<usize>> {
		self.base.trie_cache_maximum_size()
	}

	fn rpc_methods(&self) -> Result<sc_service::config::RpcMethods> {
		self.base.rpc_methods()
	}

	fn rpc_max_connections(&self) -> Result<u32> {
		self.base.rpc_max_connections()
	}

	fn rpc_cors(&self, is_dev: bool) -> Result<Option<Vec<String>>> {
		self.base.rpc_cors(is_dev)
	}

	fn default_heap_pages(&self) -> Result<Option<u64>> {
		self.base.default_heap_pages()
	}

	fn force_authoring(&self) -> Result<bool> {
		self.base.force_authoring()
	}

	fn disable_grandpa(&self) -> Result<bool> {
		self.base.disable_grandpa()
	}

	fn max_runtime_instances(&self) -> Result<Option<usize>> {
		self.base.max_runtime_instances()
	}

	fn announce_block(&self) -> Result<bool> {
		self.base.announce_block()
	}

	fn telemetry_endpoints(
		&self,
		chain_spec: &Box<dyn ChainSpec>,
	) -> Result<Option<sc_telemetry::TelemetryEndpoints>> {
		self.base.telemetry_endpoints(chain_spec)
	}

	fn node_name(&self) -> Result<String> {
		self.base.node_name()
	}
}

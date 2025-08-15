use sc_cli::{CliConfiguration, RunCmd};
use sc_service::{BlocksPruning, PruningMode};

/// This value overrides Substrate's default state pruning value of 256, which provides at least 25
/// minutes of historical data for a chain with a 6-second block time. For our case with 100ms
/// block time this would provide only 25 seconds of history which is inconvenient. See also https://github.com/paritytech/polkadot-sdk/blob/598feddb893f5ad3923a62e41a2f179b6e10c30c/substrate/client/state-db/src/lib.rs#L65.
const CUSTOM_DEFAULT_MAX_BLOCK_CONSTRAINT: u32 = 32_768;

#[derive(Debug, Clone, clap::Parser)]
pub struct CustomRunCmd {
	#[clap(flatten)]
	pub base: RunCmd,
}

// Delegate all methods to the base RunCmd except pruning-related ones.
impl CliConfiguration for CustomRunCmd {
	fn shared_params(&self) -> &sc_cli::SharedParams {
		self.base.shared_params()
	}

	fn import_params(&self) -> Option<&sc_cli::ImportParams> {
		self.base.import_params()
	}

	fn network_params(&self) -> Option<&sc_cli::NetworkParams> {
		self.base.network_params()
	}

	fn keystore_params(&self) -> Option<&sc_cli::KeystoreParams> {
		self.base.keystore_params()
	}

	fn offchain_worker_params(&self) -> Option<&sc_cli::OffchainWorkerParams> {
		self.base.offchain_worker_params()
	}

	fn node_name(&self) -> sc_cli::Result<String> {
		self.base.node_name()
	}

	fn dev_key_seed(&self, is_dev: bool) -> sc_cli::Result<Option<String>> {
		self.base.dev_key_seed(is_dev)
	}

	fn telemetry_endpoints(
		&self,
		chain_spec: &Box<dyn sc_service::ChainSpec>,
	) -> sc_cli::Result<Option<sc_telemetry::TelemetryEndpoints>> {
		self.base.telemetry_endpoints(chain_spec)
	}

	fn role(&self, is_dev: bool) -> sc_cli::Result<sc_service::Role> {
		self.base.role(is_dev)
	}

	fn transaction_pool(
		&self,
		is_dev: bool,
	) -> sc_cli::Result<sc_service::config::TransactionPoolOptions> {
		self.base.transaction_pool(is_dev)
	}

	fn rpc_addr(
		&self,
		default_listen_port: u16,
	) -> sc_cli::Result<Option<Vec<sc_cli::RpcEndpoint>>> {
		self.base.rpc_addr(default_listen_port)
	}

	fn rpc_methods(&self) -> sc_cli::Result<sc_service::config::RpcMethods> {
		self.base.rpc_methods()
	}

	fn rpc_max_connections(&self) -> sc_cli::Result<u32> {
		self.base.rpc_max_connections()
	}

	fn rpc_cors(&self, is_dev: bool) -> sc_cli::Result<Option<Vec<String>>> {
		self.base.rpc_cors(is_dev)
	}

	fn rpc_max_request_size(&self) -> sc_cli::Result<u32> {
		self.base.rpc_max_request_size()
	}

	fn rpc_max_response_size(&self) -> sc_cli::Result<u32> {
		self.base.rpc_max_response_size()
	}

	fn rpc_max_subscriptions_per_connection(&self) -> sc_cli::Result<u32> {
		self.base.rpc_max_subscriptions_per_connection()
	}

	fn rpc_buffer_capacity_per_connection(&self) -> sc_cli::Result<u32> {
		self.base.rpc_buffer_capacity_per_connection()
	}

	fn rpc_batch_config(&self) -> sc_cli::Result<sc_service::config::RpcBatchRequestConfig> {
		self.base.rpc_batch_config()
	}

	fn rpc_rate_limit_whitelisted_ips(&self) -> sc_cli::Result<Vec<sc_service::config::IpNetwork>> {
		self.base.rpc_rate_limit_whitelisted_ips()
	}

	fn rpc_rate_limit_trust_proxy_headers(&self) -> sc_cli::Result<bool> {
		self.base.rpc_rate_limit_trust_proxy_headers()
	}

	fn prometheus_config(
		&self,
		default_listen_port: u16,
		chain_spec: &Box<dyn sc_service::ChainSpec>,
	) -> sc_cli::Result<Option<sc_service::config::PrometheusConfig>> {
		self.base.prometheus_config(default_listen_port, chain_spec)
	}

	fn init<F>(
		&self,
		support_url: &String,
		impl_version: &String,
		logger_hook: F,
	) -> sc_cli::Result<()>
	where
		F: FnOnce(&mut sc_cli::LoggerBuilder),
	{
		self.base.init(support_url, impl_version, logger_hook)
	}

	fn log_filters(&self) -> sc_cli::Result<String> {
		self.base.log_filters()
	}

	fn detailed_log_output(&self) -> sc_cli::Result<bool> {
		self.base.detailed_log_output()
	}

	fn enable_log_reloading(&self) -> sc_cli::Result<bool> {
		self.base.enable_log_reloading()
	}

	fn disable_log_color(&self) -> sc_cli::Result<bool> {
		self.base.disable_log_color()
	}

	fn tracing_receiver(&self) -> sc_cli::Result<sc_service::TracingReceiver> {
		self.base.tracing_receiver()
	}

	fn tracing_targets(&self) -> sc_cli::Result<Option<String>> {
		self.base.tracing_targets()
	}

	fn network_config(
		&self,
		chain_spec: &Box<dyn sc_service::ChainSpec>,
		is_dev: bool,
		is_validator: bool,
		net_config_dir: std::path::PathBuf,
		client_id: &str,
		node_name: &str,
		node_key: sc_network::config::NodeKeyConfig,
		default_listen_port: u16,
	) -> sc_cli::Result<sc_network::config::NetworkConfiguration> {
		self.base.network_config(
			chain_spec,
			is_dev,
			is_validator,
			net_config_dir,
			client_id,
			node_name,
			node_key,
			default_listen_port,
		)
	}

	fn keystore_config(
		&self,
		config_dir: &std::path::PathBuf,
	) -> sc_cli::Result<sc_service::config::KeystoreConfig> {
		self.base.keystore_config(config_dir)
	}

	fn database_params(&self) -> Option<&sc_cli::DatabaseParams> {
		self.base.database_params()
	}

	fn trie_cache_maximum_size(&self) -> sc_cli::Result<Option<usize>> {
		self.base.trie_cache_maximum_size()
	}

	// Override with custom state pruning defaults.
	fn state_pruning(&self) -> sc_cli::Result<Option<PruningMode>> {
		let base_result = self.base.state_pruning()?;

		match base_result {
			None => Ok(Some(PruningMode::blocks_pruning(CUSTOM_DEFAULT_MAX_BLOCK_CONSTRAINT))),
			Some(_) => Ok(base_result),
		}
	}

	fn blocks_pruning(&self) -> sc_cli::Result<BlocksPruning> {
		self.base.blocks_pruning()
	}

	fn wasm_method(&self) -> sc_cli::Result<sc_service::config::WasmExecutionMethod> {
		self.base.wasm_method()
	}

	fn wasm_runtime_overrides(&self) -> Option<std::path::PathBuf> {
		self.base.wasm_runtime_overrides()
	}

	fn runtime_cache_size(&self) -> sc_cli::Result<u8> {
		self.base.runtime_cache_size()
	}
}

#[derive(Debug, clap::Parser)]
pub struct Cli {
	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[clap(flatten)]
	pub run: CustomRunCmd,
}

#[derive(Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Subcommand {
	/// Key management cli utilities
	#[command(subcommand)]
	Key(sc_cli::KeySubcommand),

	/// Build a chain specification.
	BuildSpec(sc_cli::BuildSpecCmd),

	/// Validate blocks.
	CheckBlock(sc_cli::CheckBlockCmd),

	/// Export blocks.
	ExportBlocks(sc_cli::ExportBlocksCmd),

	/// Export the state of a given block into a chain spec.
	ExportState(sc_cli::ExportStateCmd),

	/// Import blocks.
	ImportBlocks(sc_cli::ImportBlocksCmd),

	/// Remove the whole chain.
	PurgeChain(sc_cli::PurgeChainCmd),

	/// Revert the chain to a previous state.
	Revert(sc_cli::RevertCmd),

	/// Sub-commands concerned with benchmarking.
	#[command(subcommand)]
	Benchmark(frame_benchmarking_cli::BenchmarkCmd),

	/// Db meta columns information.
	ChainInfo(sc_cli::ChainInfoCmd),
}

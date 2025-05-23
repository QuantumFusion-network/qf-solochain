use polkadot_sdk::*;

use qf_parachain_runtime as runtime;
use qf_runtime;
use qf_runtime::{AccountId, Signature};
use sp_runtime::impl_opaque_keys;
use qf_runtime::{Spin, Grandpa};
use qf_parachain_runtime::Aura;
use qfp_consensus_spin::sr25519::AuthorityId as SpinId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{Pair, Public, sr25519};
use sp_runtime::traits::{IdentifyAccount, Verify};

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<Extensions>;
pub type FastChainSpec = sc_service::GenericChainSpec;
/// The relay chain that you want to configure this parachain to connect to.
pub const RELAY_CHAIN: &str = "rococo-local";

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate an authority keys (stash, account, spin, grandpa, aura).
pub fn authority_keys_from_seed(s: &str) -> (AccountId, AccountId, SpinId, GrandpaId, AuraId) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{s}//stash")),
		get_account_id_from_seed::<sr25519::Public>(s),
		get_from_seed::<SpinId>(s),
		get_from_seed::<GrandpaId>(s),
		get_from_seed::<AuraId>(s),
	)
}

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
pub struct Extensions {
	/// The relay chain of the Parachain.
	#[serde(alias = "relayChain", alias = "RelayChain")]
	pub relay_chain: String,
	/// The id of the Parachain.
	#[serde(alias = "paraId", alias = "ParaId")]
	pub para_id: u32,
}

impl Extensions {
	/// Try to get the extension from the given `ChainSpec`.
	pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
		sc_chain_spec::get_extension(chain_spec.extensions())
	}
}

pub fn development_chain_spec() -> ChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "QF".into());
	properties.insert("tokenDecimals".into(), 18.into());
	properties.insert("ss58Format".into(), 42.into());

	ChainSpec::builder(
		runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions { relay_chain: RELAY_CHAIN.into(), para_id: runtime::PARACHAIN_ID },
	)
	.with_name("Development")
	.with_id("dev")
	.with_chain_type(ChainType::Development)
	.with_genesis_config_preset_name(sp_genesis_builder::DEV_RUNTIME_PRESET)
	.with_properties(properties)
	.build()
}

pub fn local_chain_spec() -> ChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "QF".into());
	properties.insert("tokenDecimals".into(), 18.into());
	properties.insert("ss58Format".into(), 42.into());

	#[allow(deprecated)]
	ChainSpec::builder(
		runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions { relay_chain: RELAY_CHAIN.into(), para_id: runtime::PARACHAIN_ID },
	)
	.with_name("Local Testnet")
	.with_id("local_testnet")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_preset_name(sc_chain_spec::LOCAL_TESTNET_RUNTIME_PRESET)
	.with_protocol_id("template-local")
	.with_properties(properties)
	.build()
}


pub fn fast_development_chain_spec() -> FastChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "QF".into());
	properties.insert("tokenDecimals".into(), 18.into());
	properties.insert("ss58Format".into(), 42.into());

	FastChainSpec::builder(
		qf_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		None,
		// Extensions { relay_chain: RELAY_CHAIN.into(), para_id: runtime::PARACHAIN_ID },
	)
	.with_name("Development")
	.with_id("dev")
	.with_chain_type(ChainType::Development)
	.with_genesis_config_patch(testnet_genesis(
		// Initial PoA authorities
		vec![authority_keys_from_seed("Alice")],
		// Sudo account
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		// Pre-funded accounts
		vec![
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
			get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
		],
		true,
	))
	.with_properties(properties)
	.build()
}

pub fn fast_local_chain_spec() -> FastChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "QF".into());
	properties.insert("tokenDecimals".into(), 18.into());
	properties.insert("ss58Format".into(), 42.into());

	#[allow(deprecated)]
	FastChainSpec::builder(
		qf_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		None,
		// Extensions { relay_chain: RELAY_CHAIN.into(), para_id: runtime::PARACHAIN_ID },
	)
	.with_name("Local Testnet")
	.with_id("local_testnet")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_patch(testnet_genesis(
		// Initial PoA authorities
		vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
		// Sudo account
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		// Pre-funded accounts
		vec![
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_account_id_from_seed::<sr25519::Public>("Charlie"),
			get_account_id_from_seed::<sr25519::Public>("Dave"),
			get_account_id_from_seed::<sr25519::Public>("Eve"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie"),
			get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
			get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
			get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
			get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
			get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
		],
		true,
	))
	// .with_genesis_config_preset_name(sc_chain_spec::LOCAL_TESTNET_RUNTIME_PRESET)
	.with_protocol_id("template-local")
	.with_properties(properties)
	.build()
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	initial_authorities: Vec<(AccountId, AccountId, SpinId, GrandpaId, AuraId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	_enable_println: bool,
) -> serde_json::Value {
	// Configure endowed accounts with initial balance of 10^6 UNIT.
	const ENDOWMENT: u128 = 10u128.pow(6) * qf_runtime::UNIT;
	// Configure stash accounts with initial balance of 10^5 UNIT.
	const STASH: u128 = 10u128.pow(5) * qf_runtime::UNIT;

	serde_json::json!({
		"balances": {
			"balances": endowed_accounts.iter().cloned().map(|k| (k, ENDOWMENT)).collect::<Vec<_>>(),
		},
		"session": {
			"keys": initial_authorities
				.iter()
				.map(|x| {
					(
						x.1.clone(),
						x.1.clone(),
						qf_runtime::SessionKeys {
							spin: x.2.clone(),
							grandpa: x.3.clone(),
						},
					)
				})
				.collect::<Vec<_>>(),
		},
		"staking": {
			"minimumValidatorCount": 1,
			"validatorCount": initial_authorities.len(),
			"stakers": initial_authorities
				.iter()
				.map(|x| (x.1.clone(), x.1.clone(), STASH, pallet_staking::StakerStatus::<AccountId>::Validator))
				.collect::<Vec<_>>(),
			"invulnerables": initial_authorities.iter().map(|x| x.1.clone()).collect::<Vec<_>>(),
			"forceEra": pallet_staking::Forcing::NotForcing,
			"slashRewardFraction": sp_runtime::Perbill::from_percent(10),
		},
		"sudo": {
			// Assign network admin rights.
			"key": Some(root_key),
		},
	})
}

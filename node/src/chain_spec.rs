use qf_runtime::{
	configs::ClaimPalletAccountId, AccountId, Balance, BlockNumber, SessionKeys, Signature,
	GENESIS_NEXT_ASSET_ID, UNIT, WASM_BINARY,
};
use qfp_consensus_spin::sr25519::AuthorityId as SpinId;
use sc_service::ChainType;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{AccountIdConversion, IdentifyAccount, Verify};

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate
/// ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec;

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

/// Generate an authority keys (stash, account, spin, grandpa).
pub fn authority_keys_from_seed(s: &str) -> (AccountId, AccountId, SpinId, GrandpaId) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{s}//stash")),
		get_account_id_from_seed::<sr25519::Public>(s),
		get_from_seed::<SpinId>(s),
		get_from_seed::<GrandpaId>(s),
	)
}

pub fn development_config() -> Result<ChainSpec, String> {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "QF".into());
	properties.insert("tokenDecimals".into(), 18.into());
	properties.insert("ss58Format".into(), 42.into());

	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
		None,
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
	.build())
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "QF".into());
	properties.insert("tokenDecimals".into(), 18.into());
	properties.insert("ss58Format".into(), 42.into());

	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
		None,
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
	.with_properties(properties)
	.build())
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	initial_authorities: Vec<(AccountId, AccountId, SpinId, GrandpaId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	_enable_println: bool,
) -> serde_json::Value {
	// Configure endowed accounts with initial balance of 10^6 UNIT.
	const ENDOWMENT: u128 = 10u128.pow(6) * UNIT;
	// Configure stash accounts with initial balance of 10^5 UNIT.
	const STASH: u128 = 10u128.pow(5) * UNIT;

	serde_json::json!({
		"assets": {
			"nextAssetId": GENESIS_NEXT_ASSET_ID,
		},
		"balances": {
			"balances": endowed_accounts.iter().cloned().map(|k| (k, ENDOWMENT)).collect::<Vec<_>>().push((ClaimPalletAccountId::get().into_account_truncating(), 0)),
		},
		"session": {
			"keys": initial_authorities
				.iter()
				.map(|x| {
					(
						x.1.clone(),
						x.1.clone(),
						SessionKeys {
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
		"vesting": {
			"vesting": Vec::<(AccountId, BlockNumber, BlockNumber, Balance)>::new(),
		},
	})
}

pub fn qf_devnet_config() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("./res/qf-devnet.raw.json")[..])
}

pub fn qf_testnet_config() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("./res/qf-testnet.raw.json")[..])
}

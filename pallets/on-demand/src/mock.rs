use crate::FixedReward;
use cumulus_pallet_parachain_system::RelayChainStateProof;
use cumulus_primitives_core::ParaId;
use frame_support::{
	pallet_prelude::*,
	parameter_types,
	traits::Everything,
	weights::{ConstantMultiplier, IdentityFee},
};
use frame_system::{EnsureRoot, pallet_prelude::*};
use pallet_transaction_payment::FungibleAdapter;
use sp_core::{ConstBool, ConstU8, ConstU64, H256};
use sp_keyring::Sr25519Keyring;
use sp_runtime::{
	AccountId32, BuildStorage, RuntimeAppPublic,
	testing::UintAuthorityId,
	traits::{BlakeTwo256, Convert, IdentityLookup, OpaqueKeys},
};

type Block = frame_system::mocking::MockBlock<Test>;
type AccountId = AccountId32;
type Balance = u64;
type BlockNumber = u32;

pub fn alice() -> AccountId {
	Sr25519Keyring::Alice.to_account_id()
}

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		TransactionPayment: pallet_transaction_payment,
		Balances: pallet_balances,
		Timestamp: pallet_timestamp,
		Aura: pallet_aura,
		Session: pallet_session,
		OnDemand: crate::{Pallet, Call, Storage, Event<T>}
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeTask = RuntimeTask;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
	type ExtensionsWeightInfo = ();
}

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ConstU64<1>;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type DoneSlashHandler = ();
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<1>;
	type WeightInfo = ();
}

impl pallet_aura::Config for Test {
	type AuthorityId = sp_consensus_aura::sr25519::AuthorityId;
	type MaxAuthorities = ConstU32<100_000>;
	type DisabledValidators = ();
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
	type SlotDuration = pallet_aura::MinimumPeriodTimesTwo<Self>;
}

sp_runtime::impl_opaque_keys! {
	pub struct MockSessionKeys {
		// a key for aura authoring
		pub aura: UintAuthorityId,
	}
}

impl From<UintAuthorityId> for MockSessionKeys {
	fn from(aura: sp_runtime::testing::UintAuthorityId) -> Self {
		Self { aura }
	}
}

parameter_types! {
	pub static SessionHandlerCollators: Vec<AccountId> = Vec::new();
	pub static SessionChangeBlock: u64 = 0;
}

pub struct TestSessionHandler;
impl pallet_session::SessionHandler<AccountId> for TestSessionHandler {
	const KEY_TYPE_IDS: &'static [sp_runtime::KeyTypeId] = &[UintAuthorityId::ID];
	fn on_genesis_session<Ks: OpaqueKeys>(keys: &[(AccountId, Ks)]) {
		SessionHandlerCollators::set(keys.iter().map(|(a, _)| a.clone()).collect::<Vec<_>>())
	}
	fn on_new_session<Ks: OpaqueKeys>(_: bool, keys: &[(AccountId, Ks)], _: &[(AccountId, Ks)]) {
		SessionChangeBlock::set(System::block_number());
		dbg!(keys.len());
		SessionHandlerCollators::set(keys.iter().map(|(a, _)| a.clone()).collect::<Vec<_>>())
	}
	fn on_before_session_ending() {}
	fn on_disabled(_: u32) {}
}

parameter_types! {
	pub const Offset: u64 = 0;
	pub const Period: u64 = 10;
}

impl pallet_session::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	// we don't have stash and controller, thus we don't need the convert as well.
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = ();
	type SessionHandler = TestSessionHandler;
	type Keys = MockSessionKeys;
	type WeightInfo = ();
}

pub struct MockTxPaymentWeights;
impl pallet_transaction_payment::WeightInfo for MockTxPaymentWeights {
	fn charge_transaction_payment() -> Weight {
		Weight::from_parts(10, 0)
	}
}

parameter_types! {
	pub const TransactionByteFee: Balance = 0;
}

impl pallet_transaction_payment::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = FungibleAdapter<Balances, ()>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = IdentityFee<Balance>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = ();
	type WeightInfo = MockTxPaymentWeights;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct BenchHelper;
#[cfg(feature = "runtime-benchmarks")]
impl crate::BenchmarkHelper<Balance> for BenchHelper {
	fn mock_threshold_parameter() -> Balance {
		1_000u32.into()
	}
}

pub struct ToAccountIdImpl;
impl Convert<AccountId, AccountId32> for ToAccountIdImpl {
	fn convert(v: AccountId) -> AccountId {
		v
	}
}

parameter_types! {
	pub const Reward: u64 = 1_000_000;
	pub const BaseWeight: Weight =
		Weight::from_parts(125_000, 0);
}

pub struct OrderPlacementChecker;
impl crate::OrdersPlaced<Balance, AccountId> for OrderPlacementChecker {
	fn orders_placed(
		_relay_state_proof: RelayChainStateProof,
		_expected_para_id: ParaId,
	) -> Vec<(Balance, AccountId)> {
		// Dummy implementation.
		Default::default()
	}
}

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RelayChainBalance = Balance;
	type AdminOrigin = EnsureRoot<AccountId>;
	type BlockNumber = BlockNumber;
	type ThresholdParameter = Balance; // Represents fee threshold.
	type Currency = Balances;
	type OnReward = OnDemand;
	type RewardSize = FixedReward<Balance, Reward>;
	type ToAccountId = ToAccountIdImpl;
	type OrderPlacementCriteria = crate::FeeBasedCriteria<Test, BaseWeight>;
	type OrdersPlaced = OrderPlacementChecker;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = BenchHelper;
	type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

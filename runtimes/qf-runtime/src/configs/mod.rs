// This is free and unencumbered software released into the public domain.
//
// Anyone is free to copy, modify, publish, use, compile, sell, or
// distribute this software, either in source code form or as a compiled
// binary, for any purpose, commercial or non-commercial, and by any
// means.
//
// In jurisdictions that recognize copyright laws, the author or authors
// of this software dedicate any and all copyright interest in the
// software to the public domain. We make this dedication for the benefit
// of the public at large and to the detriment of our heirs and
// successors. We intend this dedication to be an overt act of
// relinquishment in perpetuity of all present and future rights to this
// software under copyright law.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY CLAIM, DAMAGES OR
// OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
// ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
// OTHER DEALINGS IN THE SOFTWARE.
//
// For more information, please refer to <http://unlicense.org>

// Substrate and Polkadot dependencies
use frame_election_provider_support::{bounds::ElectionBoundsBuilder, onchain, SequentialPhragmen};
use frame_support::{
	derive_impl, parameter_types,
	traits::{ConstBool, ConstU128, ConstU32, ConstU64, ConstU8, Nothing, VariantCountOf},
	weights::{
		constants::{RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND},
		IdentityFee, Weight,
	},
};
use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot,
};
use pallet_transaction_payment::{ConstFeeMultiplier, FungibleAdapter, Multiplier};
use qfp_consensus_spin::sr25519::AuthorityId as SpinId;
use sp_runtime::{
	curve::PiecewiseLinear,
	traits::{One, OpaqueKeys},
	Perbill,
};
use sp_version::RuntimeVersion;

use crate::{MILLI_UNIT, SESSION_LENGTH};

// Local module imports
use super::{
	AccountId, Balance, Balances, Block, BlockNumber, Hash, Nonce, PalletInfo, Runtime,
	RuntimeCall, RuntimeEvent, RuntimeFreezeReason, RuntimeHoldReason, RuntimeOrigin, RuntimeTask,
	Session, SessionKeys, Spin, Staking, System, Timestamp, EXISTENTIAL_DEPOSIT, SLOT_DURATION,
	VERSION,
};

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400 * SESSION_LENGTH / 10;
	pub const Version: RuntimeVersion = VERSION;

	/// We allow for 50 ms of compute with a 100 ms average block time.
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::with_sensible_defaults(
		Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND / 20, u64::MAX),
		NORMAL_DISPATCH_RATIO,
	);
	pub RuntimeBlockLength: BlockLength = BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub const SS58Prefix: u8 = 42;
}

/// The default types are being injected by
/// [`derive_impl`](`frame_support::derive_impl`) from
/// [`SoloChainDefaultConfig`](`struct@
/// frame_system::config_preludes::SolochainDefaultConfig`), but overridden as
/// needed.
#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig)]
impl frame_system::Config for Runtime {
	/// The block type for the runtime.
	type Block = Block;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = RuntimeBlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = RuntimeBlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// Maximum number of block number to block hash mappings to keep (oldest
	/// pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// This is used as an identifier of the chain. 42 is the generic substrate
	/// prefix.
	type SS58Prefix = SS58Prefix;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_spin::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AuthorityId = SpinId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<32>;
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
	type SlotDuration = pallet_spin::MinimumPeriodTimesTwo<Runtime>;
	type DefaultSessionLength = ConstU32<SESSION_LENGTH>;
}

parameter_types! {
	/// Session period - 30 leadership tenures
	pub const Period: BlockNumber = 30 * SESSION_LENGTH;
	/// Offset – 0 blocks
	pub const Offset: BlockNumber = 0;
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Self>;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type DisablingStrategy = pallet_session::disabling::UpToLimitWithReEnablingDisablingStrategy;

	type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
	type SessionManager = Staking;
}

parameter_types! {
	pub const MaxElectingVoters: u32 = 22_500;
	/// We take the top 22500 nominators as electing voters and all of the validators as electable
	/// targets. Whilst this is the case, we cannot and shall not increase the size of the
	/// validator intentions.
	pub ElectionBounds: frame_election_provider_support::bounds::ElectionBounds =
		ElectionBoundsBuilder::default().voters_count(MaxElectingVoters::get().into()).build();
	// Maximum winners that can be chosen as active validators
	pub const MaxActiveValidators: u32 = 1000;

	// TODO(khssnv): uncomment the block at `stable2406` or later?
	// One page only, fill the whole page with the `MaxActiveValidators`.
	// pub const MaxWinnersPerPage: u32 = MaxActiveValidators::get();
	// Unbonded, thus the max backers per winner maps to the max electing voters limit.
	// pub const MaxBackersPerWinner: u32 = MaxElectingVoters::get();
}

pub type OnChainAccuracy = sp_runtime::Perbill;

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	// type Sort = ConstBool<true>; // TODO(khssnv): uncomment at `stable2406` or later?
	type System = Runtime;
	type Solver = SequentialPhragmen<AccountId, OnChainAccuracy>;
	type DataProvider = Staking;
	type WeightInfo = frame_election_provider_support::weights::SubstrateWeight<Runtime>;
	type Bounds = ElectionBounds;

	// TODO(khssnv): uncomment at `stable2406` or later?
	// type MaxBackersPerWinner = MaxBackersPerWinner;
	// type MaxWinnersPerPage = MaxWinnersPerPage;
	type MaxWinners = MaxActiveValidators; // TODO(khssnv): remove at `stable2406` or later?
}

pallet_staking_reward_curve::build! {
	const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_025_000,
		max_inflation: 0_100_000,
		ideal_stake: 0_500_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}

parameter_types! {
	/// Number of sessions per era
	pub const SessionsPerEra: sp_staking::SessionIndex = 3;
	/// Вefines the bonding (locking) period for staking funds (measured in eras)
	pub const BondingDuration: sp_staking::EraIndex = 3;
	/// Delay before a slash (penalty) becomes effective
	pub const SlashDeferDuration: sp_staking::EraIndex = 24 * 7;
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
}

/// Upper limit on the number of NPOS nominations.
const MAX_QUOTA_NOMINATIONS: u32 = 16;

pub struct StakingBenchmarkingConfig;
impl pallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
	type MaxNominators = ConstU32<5000>;
	type MaxValidators = ConstU32<1000>;
}

impl pallet_staking::Config for Runtime {
	type OldCurrency = Balances;
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type UnixTime = Timestamp;
	type CurrencyToVote = sp_staking::currency_to_vote::U128CurrencyToVote;
	type RewardRemainder = ();
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Slash = ();
	type Reward = (); // rewards are minted from the void
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	type AdminOrigin = EnsureRoot<AccountId>;
	type SessionInterface = ();
	/// Defines how the total inflation per era is computed
	/// and split between validators and the system
	type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
	type NextNewSession = Session;
	type MaxExposurePageSize = ConstU32<64>;
	/// Maximum number of active validators allowed
	// type MaxValidatorSet = ConstU32<100>; // TODO(khssnv): uncomment at `stable2406` or later?
	/// Provides the on‐chain election logic
	type ElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type VoterList = pallet_staking::UseNominatorsAndValidatorsMap<Self>;
	type NominationsQuota = pallet_staking::FixedNominationsQuota<MAX_QUOTA_NOMINATIONS>;
	type TargetList = pallet_staking::UseValidatorsMap<Self>;
	type MaxUnlockingChunks = ConstU32<32>;
	/// Maximum number of unbonding chunks a staker's ledger may contain.
	/// Limits how many eras of unbonding can exist in flight
	type MaxControllersInDeprecationBatch = ConstU32<5900>;
	/// Number of eras to keep in on‐chain history (for rewards, points, exposures, etc.)
	type HistoryDepth = ConstU32<32>;
	type EventListeners = ();
	type WeightInfo = pallet_staking::weights::SubstrateWeight<Runtime>;
	type BenchmarkingConfig = StakingBenchmarkingConfig;
	/// Maximum number of invulnerable validators
	// type MaxInvulnerables = ConstU32<20>; // TODO(khssnv): uncomment at `stable2406` or later?
	/// Maximum number of validators that can be marked disabled at once,
	/// limiting how many can be chilled or forced out in a batch
	// type MaxDisabledValidators = ConstU32<100>; // TODO(khssnv): uncomment at `stable2406`?
	type Filter = Nothing;
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;

	type WeightInfo = ();
	/// Max authorities in use
	type MaxAuthorities = ConstU32<32>;
	/// The maximum number of nominators for each validator
	type MaxNominators = ConstU32<0>;
	type MaxSetIdSessionEntries = ConstU64<0>;

	type KeyOwnerProof = sp_core::Void;
	type EquivocationReportSystem = ();
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Spin;
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	type WeightInfo = ();
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeHoldReason;
	type DoneSlashHandler = ();
}

parameter_types! {
	pub FeeMultiplier: Multiplier = Multiplier::one();
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = FungibleAdapter<Balances, ()>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = IdentityFee<Balance>;
	type LengthToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
	type WeightInfo = pallet_transaction_payment::weights::SubstrateWeight<Runtime>;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const PolkaVmMaxCodeLen: u32 = 131072;
	pub const PolkaVmMaxCodeVersion: u64 = u64::MAX;
	pub const PolkaVmMaxUserDataLen: u32 = 2048;
	pub const PolkaVmMaxGasLimit: u64 = 2097152;
	pub const PolkaVmMaxStorageKeySize: u32 = 256;
	pub const PolkaVmMaxStorageSlots: u32 = 4;
	pub const PolkaVmMaxLogLen: u32 = 1024;
	pub const PolkaVmMinGasPrice: u64 = 1;
	pub const PolkaVmMinStorageDepositLimit: u64 = 0;
	pub const PolkaVmStorageSize: u32 = 2048;
	pub const PolkaVmStorageSlotPrice: u128 = 1 * MILLI_UNIT;
}

impl pallet_qf_polkavm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MaxCodeLen = PolkaVmMaxCodeLen;
	type MaxCodeVersion = PolkaVmMaxCodeVersion;
	type MaxUserDataLen = PolkaVmMaxUserDataLen;
	type MaxGasLimit = PolkaVmMaxGasLimit;
	type MaxStorageKeySize = PolkaVmMaxStorageKeySize;
	type MaxStorageSlots = PolkaVmMaxStorageSlots;
	type MaxLogLen = PolkaVmMaxLogLen;
	type MinGasPrice = PolkaVmMinGasPrice;
	type MinStorageDepositLimit = PolkaVmMinStorageDepositLimit;
	type StorageSize = PolkaVmStorageSize;
	type StorageSlotPrice = PolkaVmStorageSlotPrice;
	type Currency = Balances;
	type WeightInfo = pallet_qf_polkavm::weights::SubstrateWeight<Runtime>;
}

impl pallet_qf_polkavm_dev::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MaxCodeLen = PolkaVmMaxCodeLen;
	type WeightInfo = pallet_qf_polkavm_dev::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const FaucetAmount: u64 = 2;
	pub const LockPeriod: u32 = 3600; // ~3h
}

impl pallet_faucet::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type FaucetAmount = FaucetAmount;
	type LockPeriod = LockPeriod;
	type WeightInfo = pallet_faucet::weights::SubstrateWeight<Runtime>;
}

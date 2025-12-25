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
	traits::{
		fungible::{Balanced, Credit, Mutate},
		tokens::{Fortitude, Precision, Preservation},
		AsEnsureOriginWithArg, ConstBool, ConstU32, ConstU64, ConstU8, DefensiveSaturating, Get,
		Nothing, OnUnbalanced, VariantCountOf, WithdrawReasons,
	},
	weights::{
		constants::{RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND},
		IdentityFee, Weight,
	},
	PalletId,
};
use frame_system::{
	limits::{BlockLength, BlockWeights},
	pallet_prelude::BlockNumberFor,
	EnsureRoot, EnsureSigned,
};
use pallet_claims::CompensateTrait;
use pallet_transaction_payment::{ConstFeeMultiplier, FungibleAdapter, Multiplier};
use qfp_consensus_spin::sr25519::AuthorityId as SpinId;
use sp_runtime::{
	curve::PiecewiseLinear,
	traits::{AccountIdConversion, ConvertInto, One, OpaqueKeys},
	Perbill,
};
use sp_version::RuntimeVersion;

use crate::{deposit, Vesting, SESSION_LENGTH};

#[cfg(feature = "runtime-benchmarks")]
use crate::GENESIS_NEXT_ASSET_ID;

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

parameter_types! {
	pub const ApprovalDeposit: Balance = EXISTENTIAL_DEPOSIT;
	pub const AssetAccountDeposit: Balance = deposit(1, 16);
	pub const AssetDeposit: Balance = deposit(1, 190);
	pub const AssetsStringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = deposit(1, 68);
	pub const MetadataDepositPerByte: Balance = deposit(0, 1);
	pub const RemoveItemsLimit: u32 = 1000;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct AssetsBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pallet_assets::BenchmarkHelper<codec::Compact<u32>> for AssetsBenchmarkHelper {
	fn create_asset_id_parameter(id: u32) -> codec::Compact<u32> {
		codec::Compact(GENESIS_NEXT_ASSET_ID.unwrap_or_default() + id)
	}
}

impl pallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = u32;
	type AssetIdParameter = codec::Compact<u32>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Freezer = ();
	type Holder = ();
	type Extra = ();
	type WeightInfo = crate::weights::pallet_assets::WeightInfo<Runtime>;
	type CallbackHandle = pallet_assets::AutoIncAssetId<Runtime>;
	type AssetAccountDeposit = AssetAccountDeposit;
	type RemoveItemsLimit = RemoveItemsLimit;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = AssetsBenchmarkHelper;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Spin>;
	type EventHandler = (); // TODO(khssnv): Staking, ImOnline?
}

impl pallet_spin::Config for Runtime {
	type AuthorityId = SpinId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<32>;
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
	type SlotDuration = pallet_spin::MinimumPeriodTimesTwo<Runtime>;
	type DefaultSessionLength = ConstU64<SESSION_LENGTH>;
}

pub const LEADER_TENURES_PER_SESSION: u32 = 30;

/// Provides dynamic session length to reflect changes in leader's tenure duration
pub struct SessionPeriodLength<T>(core::marker::PhantomData<T>);

impl<T: pallet_spin::Config> Get<BlockNumberFor<T>> for SessionPeriodLength<T> {
	fn get() -> BlockNumberFor<T> {
		pallet_spin::SessionLength::<T>::get()
			.defensive_saturating_mul(LEADER_TENURES_PER_SESSION.into())
	}
}

parameter_types! {
	/// Offset – 0 blocks
	pub const Offset: BlockNumber = 0;
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = sp_runtime::traits::ConvertInto;
	type ShouldEndSession = pallet_session::PeriodicSessions<SessionPeriodLength<Self>, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<SessionPeriodLength<Self>, Offset>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type DisablingStrategy = pallet_session::disabling::UpToLimitWithReEnablingDisablingStrategy;
	type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
	type SessionManager = Staking;
	type Currency = Balances;
	type KeyDeposit = ();
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

	// One page only, fill the whole page with the `MaxActiveValidators`.
	pub const MaxWinnersPerPage: u32 = MaxActiveValidators::get();
	// Unbonded, thus the max backers per winner maps to the max electing voters limit.
	pub const MaxBackersPerWinner: u32 = MaxElectingVoters::get();
}

pub type OnChainAccuracy = sp_runtime::Perbill;

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type Sort = ConstBool<true>;
	type System = Runtime;
	type Solver = SequentialPhragmen<AccountId, OnChainAccuracy>;
	type DataProvider = Staking;
	type WeightInfo = frame_election_provider_support::weights::SubstrateWeight<Runtime>;
	type Bounds = ElectionBounds;
	type MaxBackersPerWinner = MaxBackersPerWinner;
	type MaxWinnersPerPage = MaxWinnersPerPage;
}

pallet_staking_reward_curve::build! {
	const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_010_000,
		max_inflation: 0_100_000,
		ideal_stake: 0_300_000,
		falloff: 0_100_000,
		max_piece_count: 100,
		test_precision: 0_001_000,
	);
}

parameter_types! {
	/// Number of sessions per era
	pub const SessionsPerEra: sp_staking::SessionIndex = 3;
	/// Вefines the bonding (locking) period for staking funds (measured in eras)
	pub const BondingDuration: sp_staking::EraIndex = 3;
	/// Delay before a slash (penalty) becomes effective
	pub const SlashDeferDuration: sp_staking::EraIndex = 2;
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
	type MaxValidatorSet = ConstU32<100>;
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
	type HistoryDepth = ConstU32<128>; // 30 minutes per session * 3 sessions per era * 128 eras = 8 days
	type EventListeners = ();
	type WeightInfo = pallet_staking::weights::SubstrateWeight<Runtime>;
	type BenchmarkingConfig = StakingBenchmarkingConfig;
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

parameter_types! {
	pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
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
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type DoneSlashHandler = ();
}

parameter_types! {
	pub const Prefix: &'static [u8] = b"Pay QF token to the QF Network account:";
	pub const ClaimPalletAccountId: PalletId = PalletId(*b"claim/pt");
}

pub struct Compensate;

impl CompensateTrait<Balance> for Compensate {
	fn burn_from(amount: Balance) -> sp_runtime::DispatchResult {
		let who = ClaimPalletAccountId::get().into_account_truncating();

		Balances::burn_from(
			&who,
			amount,
			Preservation::Expendable,
			Precision::Exact,
			Fortitude::Force,
		)?;
		Ok(())
	}
}

impl pallet_claims::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type VestingSchedule = Vesting;
	type Prefix = Prefix;
	type MoveClaimOrigin = EnsureRoot<AccountId>;
	type Compensate = Compensate;
	type WeightInfo = crate::weights::pallet_claims::WeightInfo<Runtime>;
}

parameter_types! {
	pub const DepositBase: Balance = deposit(1, 88);
	pub const DepositFactor: Balance = deposit(0, 32);
	pub const MaxSignatories: u32 = 100;
}

impl pallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
	type BlockNumberProvider = frame_system::Pallet<Runtime>;
}

parameter_types! {
	pub FeeMultiplier: Multiplier = Multiplier::one();
}

/// Logic for the author to get a portion of fees.
pub struct ToAuthor<R>(core::marker::PhantomData<R>);

impl<R> OnUnbalanced<Credit<R::AccountId, pallet_balances::Pallet<R>>> for ToAuthor<R>
where
	R: pallet_balances::Config + pallet_authorship::Config,
{
	fn on_nonzero_unbalanced(
		amount: Credit<<R as frame_system::Config>::AccountId, pallet_balances::Pallet<R>>,
	) {
		if let Some(author) = <pallet_authorship::Pallet<R>>::author() {
			let _ = <pallet_balances::Pallet<R>>::resolve(&author, amount);
		}
	}
}

pub struct BurnFeesAndTipToAuthor<R>(core::marker::PhantomData<R>);

impl<R> OnUnbalanced<Credit<R::AccountId, pallet_balances::Pallet<R>>> for BurnFeesAndTipToAuthor<R>
where
	R: pallet_balances::Config + pallet_authorship::Config,
{
	fn on_unbalanceds(
		mut fees_then_tips: impl Iterator<Item = Credit<R::AccountId, pallet_balances::Pallet<R>>>,
	) {
		if let Some(fees) = fees_then_tips.next() {
			// for fees, 100% to void
			drop(fees);

			if let Some(tips) = fees_then_tips.next() {
				// for tips, if any, 100% to author
				<ToAuthor<R> as OnUnbalanced<_>>::on_unbalanced(tips);
			}
		}
	}
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = FungibleAdapter<Balances, BurnFeesAndTipToAuthor<Runtime>>;
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

// TODO(khssnv): revisit.
parameter_types! {
	pub const DepositPerItem: Balance = deposit(1, 0);
	pub const DepositPerByte: Balance = deposit(0, 1);
	pub CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(30);
	pub const RuntimeMemory: u32 = 128 * 1024 * 1024; // 128 MiB
	pub const PVFMemory: u32 = 512 * 1024 * 1024; // 512 MiB
	pub const ChainId: u64 = 42;
	pub const NativeToEthRatio: u32 = 1;
}

impl pallet_revive::Config for Runtime {
	type Time = Timestamp;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type DepositPerItem = DepositPerItem;
	type DepositPerByte = DepositPerByte;
	type WeightPrice = pallet_transaction_payment::Pallet<Self>;
	type WeightInfo = pallet_revive::weights::SubstrateWeight<Self>;
	type AddressMapper = pallet_revive::AccountId32Mapper<Self>;
	type RuntimeMemory = RuntimeMemory;
	type PVFMemory = PVFMemory;
	type UnsafeUnstableInterface = ConstBool<true>; // TODO(khssnv): try with `false` at `stable2506`.
	type UploadOrigin = EnsureSigned<Self::AccountId>;
	type InstantiateOrigin = EnsureSigned<Self::AccountId>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
	type ChainId = ChainId;
	type NativeToEthRatio = NativeToEthRatio;
	type EthGasEncoder = ();
	type FindAuthor = <Runtime as pallet_authorship::Config>::FindAuthor;
	type Precompiles = ();
	type AllowEVMBytecode = ConstBool<false>;
}

impl TryFrom<RuntimeCall> for pallet_revive::Call<Runtime> {
	type Error = ();

	fn try_from(value: RuntimeCall) -> Result<Self, Self::Error> {
		match value {
			RuntimeCall::Revive(call) => Ok(call),
			_ => Err(()),
		}
	}
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = super::OriginCaller;
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

impl pallet_spin_anchoring::Config for Runtime {}

parameter_types! {
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pallet_vesting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BlockNumberToBalance = ConvertInto;
	type MinVestedTransfer = ExistentialDeposit;
	type WeightInfo = pallet_vesting::weights::SubstrateWeight<Runtime>;
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type BlockNumberProvider = System;
	// `VestingInfo` encode length is 36bytes. 28 schedules gets encoded as 1009 bytes, which is the
	// highest number of schedules that encodes less than 2^10.
	const MAX_VESTING_SCHEDULES: u32 = 28;
}

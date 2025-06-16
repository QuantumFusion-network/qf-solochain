use crate as pallet;

use frame_support::{derive_impl, pallet_prelude::inject_runtime_type, parameter_types};
use sp_runtime::{BuildStorage, traits::IdentityLookup};

type Block = frame_system::mocking::MockBlock<Test>;

pub type AccountId = u128;
pub type Balance = u128;

pub const MILLI_UNIT: Balance = 1_000_000_000_000_000;

#[frame_support::runtime]
mod runtime {
	// The main runtime
	#[runtime::runtime]
	// Runtime Types to be generated
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask
	)]
	pub struct Test;

	#[runtime::pallet_index(0)]
	pub type System = frame_system::Pallet<Test>;

	#[runtime::pallet_index(1)]
	pub type Balances = pallet_balances::Pallet<Test>;

	#[runtime::pallet_index(2)]
	pub type QfPolkaVM = pallet::Pallet<Test>;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = pallet_balances::AccountData<u64>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
}

parameter_types! {
	pub const PolkaVmMaxCodeLen: u32 = 131072;
	pub const PolkaVmMaxCodeSlot: u64 = u64::MAX;
	pub const PolkaVmMaxUserDataLen: u32 = 2048;
	pub const PolkaVmMaxGasLimit: u32 = 2097152;
	pub const PolkaVmMaxStorageKeySize: u32 = 256;
	pub const PolkaVmMaxStorageSlots: u32 = 4;
	pub const PolkaVmMaxLogLen: u32 = 1024;
	pub const PolkaVmMinGasPrice: u64 = 1;
	pub const PolkaVmMinStorageDepositLimit: u64 = 0;
	pub const PolkaVmStorageDeposit: u64 = 1_000_000_000_000_000;
	pub const PolkaVmStorageSize: u32 = 2048;
	pub const PolkaVmStorageSlotPrice: u128 = 1 * MILLI_UNIT;
}

impl pallet::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MaxCodeLen = PolkaVmMaxCodeLen;
	type MaxCodeSlot = PolkaVmMaxCodeSlot;
	type MaxUserDataLen = PolkaVmMaxUserDataLen;
	type MaxGasLimit = PolkaVmMaxGasLimit;
	type MaxStorageKeySize = PolkaVmMaxStorageKeySize;
	type MaxStorageSlots = PolkaVmMaxStorageSlots;
	type MaxLogLen = PolkaVmMaxLogLen;
	type MinGasPrice = PolkaVmMinGasPrice;
	type MinStorageDepositLimit = PolkaVmMinStorageDepositLimit;
	type StorageDeposit = PolkaVmStorageDeposit;
	type StorageSize = PolkaVmStorageSize;
	type StorageSlotPrice = PolkaVmStorageSlotPrice;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	frame_system::GenesisConfig::<Test>::default()
		.build_storage()
		.expect("Genesis storage can be builded; qed")
		.into()
}

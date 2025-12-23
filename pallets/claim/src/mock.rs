// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! Mocking utilities for testing in claims pallet.

#[cfg(test)]
use super::*;
use secp_utils::*;

// The testing primitives are very useful for avoiding having to work with signatures
// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
use crate as claims;
use frame_support::{
	derive_impl, ord_parameter_types, parameter_types,
	traits::{
		fungible::Mutate,
		tokens::{Fortitude, Precision, Preservation},
		WithdrawReasons,
	},
};
use pallet_balances;
use sp_runtime::{traits::Identity, BuildStorage};

type Block = frame_system::mocking::MockBlock<Test>;
const CHARLIE: u64 = 0;

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Vesting: pallet_vesting,
		Claims: claims,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type AccountData = pallet_balances::AccountData<u64>;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
}

parameter_types! {
	pub const MinVestedTransfer: u64 = 1;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pallet_vesting::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BlockNumberToBalance = Identity;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = ();
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type BlockNumberProvider = System;
	const MAX_VESTING_SCHEDULES: u32 = 28;
}

parameter_types! {
	pub Prefix: &'static [u8] = b"Pay RUSTs to the TEST account:";
}
ord_parameter_types! {
	pub const Six: u64 = 6;
	pub const Seven: u64 = 7;
}

pub struct Compensate;
impl CompensateTrait<u64> for Compensate {
	fn burn_from(amount: u64) -> sp_runtime::DispatchResult {
		Balances::burn_from(
			&CHARLIE,
			amount,
			Preservation::Expendable,
			Precision::Exact,
			Fortitude::Force,
		)?;
		Ok(())
	}
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type VestingSchedule = Vesting;
	type Prefix = Prefix;
	type MoveClaimOrigin = frame_system::EnsureSignedBy<Six, u64>;
	type MintClaimOrigin = frame_system::EnsureSignedBy<Seven, u64>;
	type Compensate = Compensate;
	type WeightInfo = TestWeightInfo;
}

pub fn alice() -> libsecp256k1::SecretKey {
	libsecp256k1::SecretKey::parse(&keccak_256(b"Alice")).unwrap()
}
pub fn bob() -> libsecp256k1::SecretKey {
	libsecp256k1::SecretKey::parse(&keccak_256(b"Bob")).unwrap()
}
pub fn dave() -> libsecp256k1::SecretKey {
	libsecp256k1::SecretKey::parse(&keccak_256(b"Dave")).unwrap()
}
pub fn eve() -> libsecp256k1::SecretKey {
	libsecp256k1::SecretKey::parse(&keccak_256(b"Eve")).unwrap()
}
pub fn frank() -> libsecp256k1::SecretKey {
	libsecp256k1::SecretKey::parse(&keccak_256(b"Frank")).unwrap()
}
pub fn charlie() -> libsecp256k1::SecretKey {
	libsecp256k1::SecretKey::parse(&keccak_256(b"Charlie")).unwrap()
}
pub fn gave() -> libsecp256k1::SecretKey {
	libsecp256k1::SecretKey::parse(&keccak_256(b"Gave")).unwrap()
}
pub fn mark() -> libsecp256k1::SecretKey {
	libsecp256k1::SecretKey::parse(&keccak_256(b"Mark")).unwrap()
}

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	// We use default for brevity, but you can configure as desired if needed.
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![(CHARLIE, 566u64)],
		dev_accounts: None,
	}
	.assimilate_storage(&mut t)
	.unwrap();
	claims::GenesisConfig::<Test> {
		claims: vec![
			(eth(&alice()), 100, None, None),
			(eth(&dave()), 200, None, Some(StatementKind::Regular)),
			(eth(&eve()), 300, Some(42), Some(StatementKind::Saft)),
			(eth(&frank()), 400, Some(43), None),
			(eth(&charlie()), 600, None, None),
			(eth(&gave()), 66, None, None),
			(eth(&mark()), 566, None, None),
		],
		vesting: vec![(eth(&alice()), (50, 10, 1)), (eth(&gave()), (30, 10, 1))],
	}
	.assimilate_storage(&mut t)
	.unwrap();
	t.into()
}

pub fn total_claims() -> u64 {
	100 + 66 + 200 + 300 + 400 + 600 + 566
}

// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Test utilities

#![cfg(test)]

use crate as pallet_spin;
use frame_support::{
    derive_impl, parameter_types,
    traits::{ConstU32, ConstU64, DisabledValidators},
};
use qfp_consensus_spin::{AuthorityIndex, ed25519::AuthorityId};
use sp_runtime::{BuildStorage, testing::UintAuthorityId};

type Block = frame_system::mocking::MockBlock<Test>;

const SLOT_DURATION: u64 = 2;

frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Spin: pallet_spin,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = Spin;
    type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
    type WeightInfo = ();
}

parameter_types! {
    static DisabledValidatorTestValue: Vec<AuthorityIndex> = Default::default();
    pub static AllowMultipleBlocksPerSlot: bool = false;
}

pub struct MockDisabledValidators;

impl MockDisabledValidators {
    pub fn disable_validator(index: AuthorityIndex) {
        DisabledValidatorTestValue::mutate(|v| {
            if let Err(i) = v.binary_search(&index) {
                v.insert(i, index);
            }
        })
    }
}

impl DisabledValidators for MockDisabledValidators {
    fn is_disabled(index: AuthorityIndex) -> bool {
        DisabledValidatorTestValue::get()
            .binary_search(&index)
            .is_ok()
    }

    fn disabled_validators() -> Vec<u32> {
        DisabledValidatorTestValue::get()
    }
}

pub(super) const DEFAULT_SESSION_LENGTH: u32 = 4;
impl pallet_spin::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type AuthorityId = AuthorityId;
    type DisabledValidators = MockDisabledValidators;
    type MaxAuthorities = ConstU32<10>;
    type AllowMultipleBlocksPerSlot = AllowMultipleBlocksPerSlot;
    type SlotDuration = ConstU64<SLOT_DURATION>;
    type DefaultSessionLength = ConstU32<DEFAULT_SESSION_LENGTH>;
}

fn build_ext(authorities: Vec<u64>) -> sp_io::TestExternalities {
    let mut storage = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    pallet_spin::GenesisConfig::<Test> {
        authorities: authorities
            .into_iter()
            .map(|a| UintAuthorityId(a).to_public_key())
            .collect(),
    }
    .assimilate_storage(&mut storage)
    .unwrap();
    storage.into()
}

pub fn build_ext_and_execute_test(authorities: Vec<u64>, test: impl FnOnce() -> ()) {
    let mut ext = build_ext(authorities);
    ext.execute_with(|| {
        test();
        Spin::do_try_state().expect("Storage invariants should hold")
    });
}

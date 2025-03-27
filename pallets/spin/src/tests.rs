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

//! Tests for the module.

#![cfg(test)]

use super::pallet;
use crate::mock::{
    DEFAULT_SESSION_LENGTH, MockDisabledValidators, RuntimeOrigin, Spin, System, Test,
    build_ext_and_execute_test,
};
use codec::Encode;
use frame_support::{assert_err, assert_ok, traits::OnInitialize};
use qfp_consensus_spin::{SPIN_ENGINE_ID, Slot};
use sp_runtime::{Digest, DigestItem, traits::BadOrigin};

#[test]
fn initial_values() {
    build_ext_and_execute_test(vec![0, 1, 2, 3], || {
        assert_eq!(pallet::CurrentSlot::<Test>::get(), 0u64);
        assert_eq!(
            pallet::Authorities::<Test>::get().len(),
            Spin::authorities_len()
        );
        assert_eq!(Spin::authorities_len(), 4);
    });
}

#[test]
fn session_length_works() {
    build_ext_and_execute_test(vec![0, 1, 2, 3], || {
        assert_eq!(pallet::SessionLength::<Test>::get(), DEFAULT_SESSION_LENGTH);

        assert_err!(
            Spin::set_session_length(RuntimeOrigin::signed(1), 10000),
            BadOrigin,
        );

        let new_session_length = 1000;
        assert_ok!(Spin::set_session_length(
            RuntimeOrigin::root(),
            new_session_length
        ));
        assert_eq!(pallet::SessionLength::<Test>::get(), new_session_length);
    });
}

#[test]
#[should_panic(
    expected = "Validator with index 1 is disabled and should not be attempting to author blocks."
)]
fn disabled_validators_cannot_author_blocks() {
    build_ext_and_execute_test(vec![0, 1, 2, 3], || {
        // slot 1 should be authored by validator at index 1
        let slot = Slot::from(1);
        let pre_digest = Digest {
            logs: vec![DigestItem::PreRuntime(SPIN_ENGINE_ID, slot.encode())],
        };

        System::reset_events();
        System::initialize(&42, &System::parent_hash(), &pre_digest);

        // let's disable the validator
        MockDisabledValidators::disable_validator(1);

        // and we should not be able to initialize the block
        Spin::on_initialize(42);
    });
}

#[test]
#[should_panic(expected = "Slot must increase")]
fn pallet_requires_slot_to_increase_unless_allowed() {
    build_ext_and_execute_test(vec![0, 1, 2, 3], || {
        crate::mock::AllowMultipleBlocksPerSlot::set(false);

        let slot = Slot::from(1);
        let pre_digest = Digest {
            logs: vec![DigestItem::PreRuntime(SPIN_ENGINE_ID, slot.encode())],
        };

        System::reset_events();
        System::initialize(&42, &System::parent_hash(), &pre_digest);

        // and we should not be able to initialize the block with the same slot a second time.
        Spin::on_initialize(42);
        Spin::on_initialize(42);
    });
}

#[test]
fn pallet_can_allow_unchanged_slot() {
    build_ext_and_execute_test(vec![0, 1, 2, 3], || {
        let slot = Slot::from(1);
        let pre_digest = Digest {
            logs: vec![DigestItem::PreRuntime(SPIN_ENGINE_ID, slot.encode())],
        };

        System::reset_events();
        System::initialize(&42, &System::parent_hash(), &pre_digest);

        crate::mock::AllowMultipleBlocksPerSlot::set(true);

        // and we should be able to initialize the block with the same slot a second time.
        Spin::on_initialize(42);
        Spin::on_initialize(42);
    });
}

#[test]
#[should_panic(expected = "Slot must not decrease")]
fn pallet_always_rejects_decreasing_slot() {
    build_ext_and_execute_test(vec![0, 1, 2, 3], || {
        let slot = Slot::from(2);
        let pre_digest = Digest {
            logs: vec![DigestItem::PreRuntime(SPIN_ENGINE_ID, slot.encode())],
        };

        System::reset_events();
        System::initialize(&42, &System::parent_hash(), &pre_digest);

        crate::mock::AllowMultipleBlocksPerSlot::set(true);

        Spin::on_initialize(42);
        System::finalize();

        let earlier_slot = Slot::from(1);
        let pre_digest = Digest {
            logs: vec![DigestItem::PreRuntime(
                SPIN_ENGINE_ID,
                earlier_slot.encode(),
            )],
        };
        System::initialize(&43, &System::parent_hash(), &pre_digest);
        Spin::on_initialize(43);
    });
}

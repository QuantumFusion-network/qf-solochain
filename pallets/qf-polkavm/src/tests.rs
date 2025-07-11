// TODO:
//
// 1. Add tests to check host functions:
// - balance
// - balance_of
// - transfer
//
// 2. Add tests to catch errors:
// - PolkaVMNotEnoughGas
// - GasLimitIsTooHigh
// - GasPriceIsTooLow
// - StorageDepositLimitIsTooLow

use crate::{
	mock::*, BlobMetadata, CodeAddress, CodeMetadata, CodeStorage, CodeStorageSlot, CodeVersion,
	Config, Error, Event, ExecResult, ExecutionResult, StorageKey,
};
use frame_support::{assert_noop, assert_ok, BoundedVec};

const ALICE: AccountId = 1;
const BOB: AccountId = 2;
const CONTRACT_ADDRESS: AccountId = 52079882031220287051226575722413486460;
const VERSION: CodeVersion = 1;

#[test]
fn upload_invalid_blob_should_not_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_noop!(
			QfPolkaVM::upload(RuntimeOrigin::signed(ALICE), vec![]),
			Error::<Test>::ProgramBlobParsingFailed
		);
		assert_noop!(
			QfPolkaVM::upload(RuntimeOrigin::signed(ALICE), vec![1, 2, 3]),
			Error::<Test>::ProgramBlobParsingFailed
		);
	})
}

#[test]
fn upload_very_big_blob_should_not_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let max_code_len: usize = <Test as Config>::MaxCodeLen::get()
			.try_into()
			.expect("u32 can be converted to usize; qed");
		let very_big_blob = vec![0; max_code_len + 1];
		assert_noop!(
			QfPolkaVM::upload(RuntimeOrigin::signed(ALICE), very_big_blob),
			Error::<Test>::ProgramBlobIsTooLarge
		);
	})
}

#[test]
fn upload_valid_blob_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_eq!(CodeAddress::<Test>::get((ALICE, VERSION)), None);
		assert_eq!(CodeMetadata::<Test>::get(ALICE), None);
		upload();
		assert_eq!(CodeAddress::<Test>::get((ALICE, VERSION)), Some(CONTRACT_ADDRESS));
		assert_eq!(
			CodeMetadata::<Test>::get(ALICE),
			Some(BlobMetadata { owner: ALICE, version: VERSION })
		);
		System::assert_last_event(
			Event::ProgramBlobUploaded {
				who: ALICE,
				contract_address: CONTRACT_ADDRESS,
				version: VERSION,
				exports: vec!["main".bytes().collect()],
			}
			.into(),
		);
	})
}

#[test]
fn block_number_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(43);
		upload();

		assert_ok!(QfPolkaVM::execute(
			RuntimeOrigin::signed(BOB),
			CONTRACT_ADDRESS,
			[3].to_vec(),
			2000.into(),
			1,
			1
		));
		assert_eq!(
			ExecutionResult::<Test>::get((CONTRACT_ADDRESS, VERSION, BOB)),
			Some(ExecResult {
				result: Some(43),
				not_enough_gas: false,
				trap: false,
				gas_before: 2000,
				gas_after: 354,
			}),
		);
		System::assert_last_event(
			Event::ExecutionResult {
				who: BOB,
				contract_address: CONTRACT_ADDRESS,
				version: VERSION,
				result: Some(43),
				not_enough_gas: false,
				trap: false,
				gas_before: 2000,
				gas_after: 354,
			}
			.into(),
		);
	})
}

#[test]
fn inc_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		upload();

		assert_eq!(CodeStorage::<Test>::get((CONTRACT_ADDRESS, VERSION, key::<Test>())), None);

		assert_ok!(QfPolkaVM::execute(
			RuntimeOrigin::signed(BOB),
			CONTRACT_ADDRESS,
			[5].to_vec(),
			20000.into(),
			1,
			1
		));
		assert_eq!(
			ExecutionResult::<Test>::get((CONTRACT_ADDRESS, VERSION, BOB)),
			Some(ExecResult {
				result: Some(0),
				not_enough_gas: false,
				trap: false,
				gas_before: 20000,
				gas_after: 2093,
			}),
		);
		assert_eq!(
			CodeStorage::<Test>::get((CONTRACT_ADDRESS, VERSION, key::<Test>())),
			Some(value::<Test>(1)),
		);
		System::assert_last_event(
			Event::ExecutionResult {
				who: BOB,
				contract_address: CONTRACT_ADDRESS,
				version: VERSION,
				result: Some(0),
				not_enough_gas: false,
				trap: false,
				gas_before: 20000,
				gas_after: 2093,
			}
			.into(),
		);

		assert_ok!(QfPolkaVM::execute(
			RuntimeOrigin::signed(BOB),
			CONTRACT_ADDRESS,
			[5].to_vec(),
			20000.into(),
			1,
			1
		));
		assert_eq!(
			ExecutionResult::<Test>::get((CONTRACT_ADDRESS, VERSION, BOB)),
			Some(ExecResult {
				result: Some(0),
				not_enough_gas: false,
				trap: false,
				gas_before: 20000,
				gas_after: 16354,
			}),
		);
		assert_eq!(
			CodeStorage::<Test>::get((CONTRACT_ADDRESS, VERSION, key::<Test>())),
			Some(value::<Test>(2)),
		);
		System::assert_last_event(
			Event::ExecutionResult {
				who: BOB,
				contract_address: CONTRACT_ADDRESS,
				version: VERSION,
				result: Some(0),
				not_enough_gas: false,
				trap: false,
				gas_before: 20000,
				gas_after: 16354,
			}
			.into(),
		);
	})
}

#[test]
fn delete_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		upload();

		assert_eq!(CodeStorage::<Test>::get((CONTRACT_ADDRESS, VERSION, key::<Test>())), None);

		assert_ok!(QfPolkaVM::execute(
			RuntimeOrigin::signed(BOB),
			CONTRACT_ADDRESS,
			[5].to_vec(),
			20000.into(),
			1,
			1
		));
		assert_eq!(
			ExecutionResult::<Test>::get((CONTRACT_ADDRESS, VERSION, BOB)),
			Some(ExecResult {
				result: Some(0),
				not_enough_gas: false,
				trap: false,
				gas_before: 20000,
				gas_after: 2093,
			}),
		);
		assert_eq!(
			CodeStorage::<Test>::get((CONTRACT_ADDRESS, VERSION, key::<Test>())),
			Some(value::<Test>(1)),
		);
		System::assert_last_event(
			Event::ExecutionResult {
				who: BOB,
				contract_address: CONTRACT_ADDRESS,
				version: VERSION,
				result: Some(0),
				not_enough_gas: false,
				trap: false,
				gas_before: 20000,
				gas_after: 2093,
			}
			.into(),
		);

		assert_ok!(QfPolkaVM::execute(
			RuntimeOrigin::signed(BOB),
			CONTRACT_ADDRESS,
			[6].to_vec(),
			20000.into(),
			1,
			1
		));
		assert_eq!(
			ExecutionResult::<Test>::get((CONTRACT_ADDRESS, VERSION, BOB)),
			Some(ExecResult {
				result: Some(0),
				not_enough_gas: false,
				trap: false,
				gas_before: 20000,
				gas_after: 18135,
			}),
		);
		assert_eq!(CodeStorage::<Test>::get((CONTRACT_ADDRESS, VERSION, key::<Test>())), None);
		System::assert_last_event(
			Event::ExecutionResult {
				who: BOB,
				contract_address: CONTRACT_ADDRESS,
				version: VERSION,
				result: Some(0),
				not_enough_gas: false,
				trap: false,
				gas_before: 20000,
				gas_after: 18135,
			}
			.into(),
		);
	})
}

fn upload() {
	assert_ok!(QfPolkaVM::upload(
		RuntimeOrigin::signed(ALICE),
		include_bytes!("seeds/hello-qf-polkavm.polkavm").to_vec()
	));
}

fn key<T: Config>() -> StorageKey<T> {
	let max_storage_key_size = <Test as Config>::MaxStorageKeySize::get()
		.try_into()
		.expect("u32 can be converted to usize; qed");
	let mut buffer = BoundedVec::with_bounded_capacity(max_storage_key_size);
	let mut raw_key = Vec::with_capacity(max_storage_key_size);
	let space = 32;
	let mut foo: Vec<_> = "foo".bytes().collect();
	let mut first_bytes: Vec<_> = vec![space; max_storage_key_size - foo.len()];
	raw_key.append(&mut first_bytes);
	raw_key.append(&mut foo);
	buffer
		.try_append(&mut raw_key)
		.expect("raw_key size is same as buffer size; qed");

	buffer
}

fn value<T: Config>(first_byte: u8) -> CodeStorageSlot<T> {
	let max_storage_size = <Test as Config>::StorageSize::get()
		.try_into()
		.expect("u32 can be converted to usize; qed");
	let mut buffer = BoundedVec::with_bounded_capacity(max_storage_size);
	let mut raw_value = Vec::with_capacity(max_storage_size);
	let mut last_bytes = vec![0; max_storage_size - 1];
	raw_value.push(first_byte);
	raw_value.append(&mut last_bytes);
	buffer
		.try_append(&mut raw_value)
		.expect("raw_value size is same as buffer size; qed");

	buffer
}

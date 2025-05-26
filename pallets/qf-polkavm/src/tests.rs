use crate::{BlobMetadata, CodeAddress, CodeMetadata, ExecResult, ExecutionResult, Error, Event, mock::*};
use frame_support::{assert_noop, assert_ok};

const ALICE: AccountId = 1;
const BOB: AccountId = 2;
const CONTRACT_ADDRESS: AccountId = 52079882031220287051226575722413486460;
const VERSION: u64 = 1;

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
fn upload_big_blob_should_not_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_noop!(
			QfPolkaVM::upload(RuntimeOrigin::signed(ALICE), [0; 131073].to_vec()),
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
		assert_eq!(CodeMetadata::<Test>::get(ALICE), Some(BlobMetadata { owner: ALICE, version: 1}));
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
		System::set_block_number(1);
		upload();

		assert_ok!(QfPolkaVM::execute(
			RuntimeOrigin::signed(BOB),
			CONTRACT_ADDRESS,
			BOB,
			1,
			[0, 3].to_vec(),
			2000,
			1
		));
		assert_eq!(
			ExecutionResult::<Test>::get((CONTRACT_ADDRESS, VERSION, BOB)),
			Some(ExecResult {
				result: Some(1),
				not_enough_gas: false,
				trap: false,
				gas_before: 2000,
				gas_after: 352,
			}),
		);
		System::assert_last_event(
			Event::ExecutionResult{
				who: BOB,
				contract_address: CONTRACT_ADDRESS,
				version: VERSION,
				result: Some(1),
				not_enough_gas: false,
				trap: false,
				gas_before: 2000,
				gas_after: 352,
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
use crate::{mock::*, Error, Event, CodeAddress};
use frame_support::{assert_noop, assert_ok};

const ALICE: AccountId = 1;
const CONTRACT_ADDRESS: AccountId = 52079882031220287051226575722413486460;
const VERSION: u64 = 1;

#[test]
fn upload_invalid_blob_should_not_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_noop!(QfPolkaVM::upload(RuntimeOrigin::signed(ALICE), vec![]), Error::<Test>::ProgramBlobParsingFailed);
		assert_noop!(QfPolkaVM::upload(RuntimeOrigin::signed(ALICE), vec![1,2,3]), Error::<Test>::ProgramBlobParsingFailed);
	})
}

#[test]
fn upload_big_blob_should_not_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_noop!(QfPolkaVM::upload(RuntimeOrigin::signed(ALICE), [0; 131073].to_vec()), Error::<Test>::ProgramBlobIsTooLarge);
	})
}

#[test]
fn upload_valid_blob_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(QfPolkaVM::upload(RuntimeOrigin::signed(ALICE), include_bytes!("seeds/hello-qf-polkavm.polkavm").to_vec()));
		assert_eq!(CodeAddress::<Test>::get((ALICE, VERSION)), Some(CONTRACT_ADDRESS));
		System::assert_last_event(Event::ProgramBlobUploaded {
			who: ALICE,
			contract_address: CONTRACT_ADDRESS,
			version: VERSION,
			exports: vec!["main".bytes().collect()],
		}.into());
	})
}

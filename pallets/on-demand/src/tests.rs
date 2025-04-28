use crate::{
	BulkMode, Event, SlotWidth, ThresholdParameter,
	mock::{OnDemand, RuntimeOrigin, System, Test, alice, new_test_ext},
};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::BadOrigin;

#[test]
fn set_slot_width_works() {
	new_test_ext().execute_with(|| {
		assert_eq!(SlotWidth::<Test>::get(), 0);

		// Failure: Bad origin
		assert_noop!(OnDemand::set_slot_width(RuntimeOrigin::signed(alice()), 1), BadOrigin);

		// Should be working fine
		assert_ok!(OnDemand::set_slot_width(RuntimeOrigin::root(), 1));

		// Check the storage item
		assert_eq!(SlotWidth::<Test>::get(), 1);

		// Check the emitted events
		System::assert_last_event(Event::SlotWidthSet { width: 1 }.into());
	})
}

#[test]
fn set_threshold_parameter_works() {
	new_test_ext().execute_with(|| {
		assert_eq!(ThresholdParameter::<Test>::get(), 0);

		// Failure: Bad origin
		assert_noop!(
			OnDemand::set_threshold_parameter(RuntimeOrigin::signed(alice()), 1_000),
			BadOrigin
		);

		// Should be working fine
		assert_ok!(OnDemand::set_threshold_parameter(RuntimeOrigin::root(), 1_000));

		// Check the storage item
		assert_eq!(ThresholdParameter::<Test>::get(), 1_000);

		// Check the emitted events
		System::assert_last_event(Event::ThresholdParameterSet { parameter: 1_000 }.into());
	})
}

#[test]
fn set_bulk_mode_works() {
	new_test_ext().execute_with(|| {
		assert_eq!(ThresholdParameter::<Test>::get(), 0);

		// Failure: Bad origin
		assert_noop!(OnDemand::set_bulk_mode(RuntimeOrigin::signed(alice()), true), BadOrigin);

		// Should be working fine
		assert_ok!(OnDemand::set_bulk_mode(RuntimeOrigin::root(), true));

		// Check the storage item
		assert_eq!(BulkMode::<Test>::get(), Some(()));

		// Check the emitted events
		System::assert_last_event(Event::BulkModeSet { bulk_mode: true }.into());
	})
}

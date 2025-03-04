pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::BlockNumberFor;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type SessionLength: Get<BlockNumberFor<Self>>;
        type RuntimeEvent: From<Event> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::storage]
    pub type SessionIndex<T: Config> = StorageValue<_, spin_primitives::SessionIndex, ValueQuery>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            if n % T::SessionLength::get() == Zero::zero() {
                // increment session index
                SessionIndex::<T>::mutate(|idx| *idx += 1);

                Self::deposit_event(Event::NewSession(SessionIndex::<T>::get()));

                return T::DbWeight::get().reads_writes(1, 1);
            }

            Weight::zero()
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event {
        /// New session
        NewSession(spin_primitives::SessionIndex),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame_support::{derive_impl, pallet_prelude::*, parameter_types};
    use sp_core::ConstU64;
    use sp_runtime::BuildStorage;

    type Block = frame_system::mocking::MockBlock<Test>;

    frame_support::construct_runtime!(
        pub struct Test {
            System: frame_system,
            Timestamp: pallet_timestamp,
            AuraSession: pallet,
        }
    );

    #[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
    impl frame_system::Config for Test {
        type Block = Block;
    }

    const SLOT_DURATION: u64 = 2000;

    impl pallet_timestamp::Config for Test {
        type Moment = u64;
        type OnTimestampSet = ();
        type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
        type WeightInfo = ();
    }

    parameter_types! {
        pub const SessionLength: u64 = 3;
    }
    impl pallet::Config for Test {
        type SessionLength = SessionLength;
        type RuntimeEvent = RuntimeEvent;
    }

    fn build_ext() -> sp_io::TestExternalities {
        let storage = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap();
        storage.into()
    }

    #[test]
    fn session_increments() {
        build_ext().execute_with(|| {
            System::set_block_number(1);

            assert_eq!(SessionIndex::<Test>::get(), 0);
            Pallet::<Test>::on_initialize(1);
            assert_eq!(SessionIndex::<Test>::get(), 0);
            Pallet::<Test>::on_initialize(2);
            assert_eq!(SessionIndex::<Test>::get(), 0);
            Pallet::<Test>::on_initialize(3);
            assert_eq!(SessionIndex::<Test>::get(), 1);

            System::assert_has_event(RuntimeEvent::AuraSession(Event::NewSession(1)));

            for i in 0..SessionLength::get() {
                Pallet::<Test>::on_initialize(i as u64);
            }
            assert_eq!(SessionIndex::<Test>::get(), 2);
        });
    }
}

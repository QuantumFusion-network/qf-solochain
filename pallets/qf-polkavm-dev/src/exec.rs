use crate::{BalanceOf, Config, Origin};

use core::marker::PhantomData;
use frame_support::pallet_prelude::DispatchResult;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub struct Stack<T: Config, E> {
    origin: Origin<T>,
    _phantom: PhantomData<E>,
}

trait Ext {
    type T: Config;

    /// Transfer some amount of funds into the specified account.
    fn transfer(&mut self, to: &AccountIdOf<Self::T>, value: BalanceOf<Self::T>) -> DispatchResult;
}
use crate::{BalanceOf, Config, Error, Origin};

use core::marker::PhantomData;
use frame_support::{pallet_prelude::{DispatchResult, Zero}, traits::tokens::Preservation};
use frame_support::traits::fungible::Mutate;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub struct Stack<T: Config, E> {
    origin: Origin<T>,
    caller: AccountIdOf<T>,
    _phantom: PhantomData<E>,
}

impl<T, E> Stack<T, E>
where   
    T: Config,
{
    fn new(
        origin: Origin<T>,
        caller: AccountIdOf<T>,
    ) -> Self {
        Self {
            origin,
            caller,
            _phantom: Default::default(),
        }
    }

    /// Transfer some funds from `from` to `to`.
    fn transfer(
        preservation: Preservation,
        from: &T::AccountId,
        to: &T::AccountId,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        if !value.is_zero() && from != to {
            T::Currency::transfer(from, to, value, preservation)
                .map_err(|_| Error::<T>::TransferFailed)?;
        }
        Ok(())
    }
}

trait Ext {
    type T: Config;

    /// Transfer some amount of funds into the specified account.
    fn transfer(&mut self, to: &AccountIdOf<Self::T>, value: BalanceOf<Self::T>) -> DispatchResult;
}

impl<T, E> Ext for Stack<T, E>
where T: Config,
{
    type T = T;

    fn transfer(&mut self, to: &T::AccountId, value: BalanceOf<T>) -> DispatchResult {
        Self::transfer(Preservation::Preserve, &self.caller, to, value)
    }    
}


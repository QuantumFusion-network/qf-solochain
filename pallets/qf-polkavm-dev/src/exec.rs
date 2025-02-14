use crate::{BalanceOf, Config, Error, Origin};
use sp_runtime::Vec;

use core::marker::PhantomData;
use frame_support::{
    pallet_prelude::{DispatchResult, Zero},
    traits::{fungible::Mutate, tokens::Preservation},
};

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub struct Stack<T: Config> {
    caller: AccountIdOf<T>,
}

impl<T> Stack<T>
where
    T: Config,
{
    pub fn new(caller: AccountIdOf<T>) -> Self {
        Self { caller }
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

pub trait Ext {
    type T: Config;

    /// Transfer some amount of funds into the specified account.
    fn transfer(&self, to: &AccountIdOf<Self::T>, value: BalanceOf<Self::T>) -> DispatchResult;
}

impl<T> Ext for Stack<T>
where
    T: Config,
{
    type T = T;

    fn transfer(&self, to: &T::AccountId, value: BalanceOf<T>) -> DispatchResult {
        Self::transfer(Preservation::Preserve, &self.caller, to, value)
    }
}

#[allow(unused_imports)]
use super::*;
use xcq_extension::XcqApiExt;
use xcq_extension::Vec;
use scale_info::prelude::{format, string::String};
use codec::alloc::borrow::ToOwned;
use frame::deps::sp_core::{sr25519, Pair};
use frame::deps::sp_runtime::{MultiAddress, print};
use frame_support::pallet_prelude::{Encode, Decode};
// use crate::Runtime;

pub type XcqResponse = Vec<u8>;
pub type XcqError = String;
pub type XcqResult = Result<XcqResponse, XcqError>;

pub use xcq_extension::{impl_extensions, ExtensionsExecutor, Guest, Input, InvokeSource, Method};
pub use xcq_primitives::metadata::Metadata;
pub use sp_api::decl_runtime_apis;
use crate::{RuntimeOrigin, AccountId, Balance, Balances};

// decl_runtime_apis! {
// 	#[api_version(1)]
//     pub trait XcqApi {
//         fn execute_query(query: Vec<u8>, input: Vec<u8>) -> XcqResult where Self: Sized;
//         fn metadata() -> Metadata where Self: Sized;
//     }
// }

// extension_core impls
pub struct ExtensionImpl;

#[derive(Encode, Decode)]
pub struct SCTransferData {
    asset: u64,
    source: [u8; 32],
    dest: [u8; 32],
}

impl xcq_extension_core::Config for ExtensionImpl {
    type ExtensionId = u64;
}

// extension_fungibles impls
impl xcq_extension_fungibles::Config for ExtensionImpl {
    type AssetId = u64;
    type AccountId = [u8; 32];
    type Balance = u64;
}

impl_extensions! {
    impl xcq_extension_core::ExtensionCore for ExtensionImpl
    {
        type Config = ExtensionImpl;
        fn has_extension(id: <Self::Config as xcq_extension_core::Config>::ExtensionId) -> bool {
            matches!(id, xcq_extension_core::EXTENSION_ID | xcq_extension_fungibles::EXTENSION_ID)
            // matches!(id, 0 | 1)
        }
    }

    impl xcq_extension_fungibles::ExtensionFungibles for ExtensionImpl
    {
        type Config = ExtensionImpl;
        #[allow(unused_variables)]
        fn balance(
            asset: <Self::Config as xcq_extension_fungibles::Config>::AssetId,
            who: <Self::Config as xcq_extension_fungibles::Config>::AccountId,
        ) -> <Self::Config as xcq_extension_fungibles::Config>::Balance {
            // let alice = sr25519::Pair::from_string("//Alice", None)
            //     .expect("static values are valid; qed");
            // let bob = sr25519::Pair::from_string("//Bob", None)
            //     .expect("static values are valid; qed");
            // let dest = AccountId::from(bob.public());
            // let from = AccountId::from(alice.public());

            // let value: u64 = 1_000_000;
            // let from = AccountId::from(alice.public());
            // // log::info!("transfer");
            // let res = crate::Assets::transfer(RuntimeOrigin::signed(from), asset as u32, MultiAddress::Id(dest), value as u128);
            // if let Ok(_) = res {
            //     print("SUCCESS: Transfer event emitted.");
            // }

            // let dest = AccountId::from(bob.public());
            // let from_free_balance = crate::Assets::balance(asset as u32, dest);
            // print("FREE BALANCE");
            // print(from_free_balance as u64);
            // from_free_balance as u64
            // let amount: u64 = 2;
            // let _ = Balances::deposit_creating(&who, amount);
            let mut s = frame_support::storage::StorageValue<Prefix = "transfer"; Value = frame_support::Blake2_128Concat>::;
            
            100
        }

        #[allow(unused_variables)]
        fn transfer(
            from_asset: <Self::Config as xcq_extension_fungibles::Config>::AssetId,
            from_who: <Self::Config as xcq_extension_fungibles::Config>::AccountId,
            to_asset: <Self::Config as xcq_extension_fungibles::Config>::AssetId,
            to_who: <Self::Config as xcq_extension_fungibles::Config>::AccountId,
        ) -> <Self::Config as xcq_extension_fungibles::Config>::Balance {
            // let origin = frame_system::RawOrigin::Root.into();
            // // use frame_support::traits::Currency;
            // let transfer_value: Balance = 1;
            // // let balance = pallet_balances::Pallet::<Runtime>::total_balance(&AccountId::from(from_who));
            // // let res = Runtime::
            // // 0
            // let _ = Balances::force_transfer(
            //     origin,
            //     from_who.into(),
            //     to_who,
            //     transfer_value.into()
            // );
            // pallet_balances::Pallet::<Runtime>::free_balance(from_who, transfer_value);
            // crate::pallet::CurrencyCall<Self::Config>::balance(from_asset, from_who).into()
            300
        }

        #[allow(unused_variables)]
        fn total_supply(asset: <Self::Config as xcq_extension_fungibles::Config>::AssetId) -> <Self::Config as xcq_extension_fungibles::Config>::Balance {
           200
        }
    }
}

// guest impls
pub struct GuestImpl {
    pub program: Vec<u8>,
}

impl Guest for GuestImpl {
    fn program(&self) -> &[u8] {
        &self.program
    }
}

pub struct InputImpl {
    pub method: Method,
    pub args: Vec<u8>,
}

impl Input for InputImpl {
    fn method(&self) -> Method {
        self.method.clone()
    }
    fn args(&self) -> &[u8] {
        &self.args
    }
}

use log;
pub struct XcqQueryExec;

impl XcqApiExt for XcqQueryExec {
    fn execute_query(query: Vec<u8>, input: Vec<u8>) -> XcqResult {
        execute_query(query, input)
    }
    fn metadata() -> Metadata {
        metadata()
    }
}

pub fn execute_query(query: Vec<u8>, input: Vec<u8>) -> XcqResult {
    log::info!("execute_query from runtime");
    let mut executor = ExtensionsExecutor::<Extensions, ()>::new(InvokeSource::RuntimeAPI);
    log::info!("executor created");
    let guest = GuestImpl { program: query.to_vec() };
    log::info!("guest created");
    let input = InputImpl {
        method: "main".to_owned(),
        args: input,
    };
    log::info!("input created");
    log::debug!("Run executor");
    executor.execute_method(guest, input)
}

pub fn metadata() -> Metadata {
    ExtensionImpl::runtime_metadata().into()
}

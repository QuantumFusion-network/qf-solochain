#[allow(unused_imports)]
use xcq_extension::Vec;
use scale_info::prelude::string::String;
use codec::alloc::borrow::ToOwned;

pub type XcqResponse = Vec<u8>;
pub type XcqError = String;
pub type XcqResult = Result<XcqResponse, XcqError>;

use xcq_extension::{impl_extensions, ExtensionsExecutor, Guest, Input, InvokeSource, Method};
use xcq_primitives::metadata::Metadata;


pub trait XcqApi {
    fn execute_query(query: Vec<u8>, input: Vec<u8>) -> XcqResult;
    fn metadata() -> Vec<u8>;
}

// extension_core impls
pub struct ExtensionImpl;

impl xcq_extension_core::Config for ExtensionImpl {
    type ExtensionId = u64;
}

// extension_fungibles impls
impl xcq_extension_fungibles::Config for ExtensionImpl {
    type AssetId = u32;
    type AccountId = [u8; 32];
    type Balance = u64;
}
impl_extensions! {
    impl xcq_extension_core::ExtensionCore for ExtensionImpl {
        type Config = ExtensionImpl;
        fn has_extension(id: <Self::Config as xcq_extension_core::Config>::ExtensionId) -> bool {
            // matches!(id, xcq_extension_core::EXTENSION_ID | xcq_extension_fungibles::EXTENSION_ID)
            matches!(id, 0 | 1)
        }
    }

    impl xcq_extension_fungibles::ExtensionFungibles for ExtensionImpl {
        type Config = ExtensionImpl;
        #[allow(unused_variables)]
        fn balance(
            asset: <Self::Config as xcq_extension_fungibles::Config>::AssetId,
            who: <Self::Config as xcq_extension_fungibles::Config>::AccountId,
        ) -> <Self::Config as xcq_extension_fungibles::Config>::Balance {
            100
        }
        #[allow(unused_variables)]
        fn transfer(
            from_asset: <Self::Config as xcq_extension_fungibles::Config>::AssetId,
            from_who: <Self::Config as xcq_extension_fungibles::Config>::AccountId,
            to_asset: <Self::Config as xcq_extension_fungibles::Config>::AssetId,
            to_who: <Self::Config as xcq_extension_fungibles::Config>::AccountId,
        ) -> <Self::Config as xcq_extension_fungibles::Config>::Balance {
            100
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
pub fn execute_query(query: Vec<u8>, input: Vec<u8>) -> XcqResult {
    let mut executor = ExtensionsExecutor::<Extensions, ()>::new(InvokeSource::RuntimeAPI);
    let guest = GuestImpl { program: query };
    let input = InputImpl {
        method: "main".to_owned(),
        args: input,
    };
    executor.execute_method(guest, input)
}

pub fn metadata() -> Metadata {
    ExtensionImpl::runtime_metadata().into()
}


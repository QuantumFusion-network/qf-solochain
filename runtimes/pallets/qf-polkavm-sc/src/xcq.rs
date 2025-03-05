#[allow(unused_imports)]
use super::*;
use xcq_extension::Vec;
use scale_info::prelude::string::String;
use codec::alloc::borrow::ToOwned;
// use crate::Runtime;

pub type XcqResponse = Vec<u8>;
pub type XcqError = String;
pub type XcqResult = Result<XcqResponse, XcqError>;

use xcq_extension::{impl_extensions, ExtensionsExecutor, Guest, Input, InvokeSource, Method};
pub use xcq_primitives::metadata::Metadata;
pub use xcq_primitives::metadata_ir::{RuntimeMetadata, ExtensionMetadataIR, MethodMetadataIR, MethodParamMetadataIR};
use xcq_types::MetaType;

pub trait XcqApi {
    fn execute_query(query: Vec<u8>, input: Vec<u8>) -> XcqResult;
    fn metadata() -> Metadata;
}

// pub trait ExtensionImpl {
//     fn runtime_metadata() -> Metadata;
// }

pub struct ExtensionImpl;

// impl RuntimeMetadata for ExtensionImpl {
//     fn runtime_metadata() -> xcq_primitives::metadata_ir::MetadataIR {
//         Metadata::from(ExtensionMetadataIR {
//             name: "xcq",
//             methods: vec![MethodMetadataIR {
//                 name: "main",
//                 inputs: vec![MethodParamMetadataIR {
//                     name: "input",
//                     ty: MetaType::Bytes,
//                 }],
//                 output: MetaType::Bytes,
//             }],
//         })
//     }
// }

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

// pub fn execute_query(query: Vec<u8>, input: Vec<u8>) -> XcqResult {
//     let mut executor = ExtensionsExecutor::<Extensions, ()>::new(InvokeSource::RuntimeAPI);
//     let guest = GuestImpl { program: query };
//     let input = InputImpl {
//         method: "main".to_owned(),
//         args: input,
//     };
//     executor.execute_method(guest, input)
// }

// pub fn metadata() -> Metadata {
//     ExtensionImpl::runtime_metadata().into()
// }


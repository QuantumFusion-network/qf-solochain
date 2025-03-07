use core::marker::PhantomData;
use frame_support::weights::Weight;

pub trait WeightInfo {
    fn drip() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn drip() -> Weight {
        Weight::from_parts(10_000_000, 0)
    }
}

impl WeightInfo for () {
    fn drip() -> Weight {
        Weight::from_parts(10_000_000, 0)
    }
}

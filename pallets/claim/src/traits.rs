use sp_runtime::DispatchResult;

pub trait CompensateTrait<Imbalance> {
	fn on_unbalanced(amount: Imbalance) -> DispatchResult;
}

impl<Imbalance> CompensateTrait<Imbalance> for () {
	fn on_unbalanced(_amount: Imbalance) -> DispatchResult {
		Ok(())
	}
}
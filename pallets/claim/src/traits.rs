use sp_runtime::DispatchResult;

pub trait TokenImbalanceTrait<Imbalance> {
	fn on_unbalanced(amount: Imbalance) -> DispatchResult;
}

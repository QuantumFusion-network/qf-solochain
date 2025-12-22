use sp_runtime::DispatchResult;

pub trait CompensateTrait<Balance> {
	fn burn_from(amount: Balance) -> DispatchResult;
}

impl<Balance> CompensateTrait<Balance> for () {
	fn burn_from(_amount: Balance) -> DispatchResult {
		Ok(())
	}
}

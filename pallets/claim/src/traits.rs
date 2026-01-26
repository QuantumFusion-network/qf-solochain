use sp_runtime::DispatchResult;

pub trait CompensateTrait<AccountId, Balance> {
	fn burn_from(amount: Balance) -> DispatchResult;

	#[cfg(feature = "runtime-benchmarks")]
	fn mint_tokens_for_further_burn(account: &AccountId, amount: Balance);
}

impl<AccountId, Balance> CompensateTrait<AccountId, Balance> for () {
	fn burn_from(_amount: Balance) -> DispatchResult {
		Ok(())
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn mint_tokens_for_further_burn(_account: &AccountId, _amount: Balance) {}
}

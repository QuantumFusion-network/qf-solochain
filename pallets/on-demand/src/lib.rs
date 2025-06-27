//! Pallet for managing the on-demand configuration.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use cumulus_pallet_parachain_system::RelayChainStateProof;
use cumulus_primitives_core::ParaId;
use frame_support::{
	pallet_prelude::*,
	traits::{fungible::Inspect, tokens::Balance as BalanceT},
};
use frame_system::pallet_prelude::*;
use pallet_transaction_payment::OnChargeTransaction;
use sp_runtime::{FixedPointNumber, SaturatedConversion, Saturating};

pub use pallet::*;

const LOG_TARGET: &str = "pallet-on-demand";

type BalanceOf<T> =
	<<T as pallet::Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
pub use benchmarking::BenchmarkHelper;

pub trait OnReward<AccountId, Balance, R: RewardSize<Balance>> {
	/// Reward logic.
	fn reward(rewardee: AccountId);
}

pub trait RewardSize<Balance> {
	/// Determines the reward size the order placer will receive.
	fn reward_size() -> Balance;
}

pub trait RuntimeOrderCriteria {
	/// Returns true or false depending on whether an order should be placed.
	fn should_place_order() -> bool;
}

pub trait OrdersPlaced<Balance, Account> {
	/// Returns the spot price and the order placer if an `OnDemandOrderPlaced` event is found.
	///
	/// Given that there can be multiple orders in a single block, the function returns a vector.
	///
	/// Arguments:
	/// - `relay_state_proof`: state proof from which the events are read.
	fn orders_placed(
		relay_state_proof: RelayChainStateProof,
		expected_para_id: ParaId,
	) -> Vec<(Balance, Account)>;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::weights::WeightInfo;
	use codec::MaxEncodedLen;
	use cumulus_primitives_core::relay_chain;
	use frame_support::{
		DefaultNoBound,
		traits::{
			fungible::{Inspect, Mutate},
			tokens::Balance,
		},
	};
	use order_primitives::OrderInherentData;
	use sp_runtime::{
		AccountId32, RuntimeAppPublic,
		traits::{AtLeast32BitUnsigned, Convert},
	};

	/// The module configuration trait.
	#[pallet::config]
	pub trait Config:
		frame_system::Config<AccountId = AccountId32> + pallet_aura::Config + pallet_session::Config
	{
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Relay chain balance type.
		type RelayChainBalance: Balance;

		/// The admin origin for managing the on-demand configuration.
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Block number type.
		type BlockNumber: Parameter
			+ Member
			+ Default
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen
			+ AtLeast32BitUnsigned;

		/// Given that we want to keep this pallet as generic as possible, we don't assume the type
		/// of the threshold.
		///
		/// We are adding this for implementations that have some kind of threshold and want it to
		/// be stored within the runtime.
		///
		/// For example, this threshold could represent the total weight of all the ready
		/// transactions from the pool, or their total fees.
		///
		/// NOTE: If there isn't a threshold parameter, this can simply be set to `()`.
		type ThresholdParameter: Member
			+ Parameter
			+ Default
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;

		/// The currency for rewarding order placers.
		type Currency: Mutate<Self::AccountId>;

		/// Type which implements the logic for rewarding order placers.
		type OnReward: OnReward<
				Self::AccountId,
				<Self::Currency as Inspect<Self::AccountId>>::Balance,
				Self::RewardSize,
			>;

		/// Reward size for order placers.
		type RewardSize: RewardSize<BalanceOf<Self>>;

		/// Type converting `Self::ValidatorId` to `Self::AccountId`.
		type ToAccountId: Convert<Self::ValidatorId, Self::AccountId>;

		/// Implements logic to check whether an order should have been placed.
		type OrderPlacementCriteria: RuntimeOrderCriteria;

		/// Type implementing the logic to check if an order was placed and extracting data from it.
		type OrdersPlaced: OrdersPlaced<BalanceOf<Self>, Self::AccountId>;

		#[cfg(feature = "runtime-benchmarks")]
		type BenchmarkHelper: crate::BenchmarkHelper<Self::ThresholdParameter>;

		/// Weight Info
		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// The authorities for the current on-demand slot.
	#[pallet::storage]
	#[pallet::getter(fn authorities)]
	pub type Authorities<T: Config> =
		StorageValue<_, BoundedVec<T::AuthorityId, T::MaxAuthorities>, ValueQuery>;

	/// Defines how often a new on-demand order is created, based on the number of slots.
	///
	/// This will limit the block production rate. However, if set to a low value, collators
	/// will struggle to coordinate effectively, leading to unnecessary multiple orders being
	/// placed.
	#[pallet::storage]
	#[pallet::getter(fn slot_width)]
	pub type SlotWidth<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// The threshold parameter stored in the runtime state.
	///
	/// This will determine whether an on-demand order should be placed by a collator.
	#[pallet::storage]
	#[pallet::getter(fn threshold_parameter)]
	pub type ThresholdParameter<T: Config> = StorageValue<_, T::ThresholdParameter, ValueQuery>;

	/// When in bulk mode we skip the `on_initialize` logic of this pallet.
	#[pallet::storage]
	#[pallet::getter(fn bulk_mode)]
	pub type BulkMode<T: Config> = StorageValue<_, (), OptionQuery>;

	/// Previous slot in which an order was placed.
	///
	/// This is tracked to prevent rewarding a collator multiple times for the same slot.
	#[pallet::storage]
	#[pallet::getter(fn previous_slot)]
	pub type PreviousSlot<T: Config> = StorageValue<_, u128, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Configuration of the coretime chain was set.
		SlotWidthSet { width: u32 },
		/// Threshold parameter set.
		ThresholdParameterSet { parameter: T::ThresholdParameter },
		/// We rewarded the order placer.
		OrderPlacerRewarded { order_placer: T::AccountId, reward: BalanceOf<T> },
		/// Bulk mode set.
		BulkModeSet { bulk_mode: bool },
	}

	#[pallet::error]
	#[derive(PartialEq)]
	pub enum Error<T> {
		/// Invalid proof provided for system events key
		InvalidProof,
		/// Failed to read the relay chain proof.
		FailedProofReading,
		/// Failed to get the order placer account based on the authority id.
		FailedToGetOrderPlacerAccount,
		/// We failed to decode inherent data.
		FailedToDecodeInherentData,
	}

	#[pallet::genesis_config]
	#[derive(DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		/// Initial threshold parameter.
		pub threshold_parameter: T::ThresholdParameter,
		/// Initial mode.
		pub bulk_mode: bool,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			ThresholdParameter::<T>::set(self.threshold_parameter.clone());
			if self.bulk_mode {
				BulkMode::<T>::set(Some(()));
			}
		}
	}

	// NOTE: always place pallet-on-demand after the aura pallet.
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_: BlockNumberFor<T>) {
			// Update to the latest AuRa authorities.
			//
			// By updating the authorities on finalize we will always have the previous set
			// from the previous block used within this pallet.
			Authorities::<T>::put(pallet_aura::Authorities::<T>::get());
		}
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = MakeFatalError<Error<T>>;

		const INHERENT_IDENTIFIER: InherentIdentifier =
			order_primitives::ON_DEMAND_INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			log::info!(
				target: LOG_TARGET,
				"Create inherent"
			);

			let data: OrderInherentData = data
				.get_data(&Self::INHERENT_IDENTIFIER)
				.ok()
				.flatten()
				.expect("there is not data to be posted; qed");

			Some(Call::create_order { data })
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::create_order { .. })
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight((0, DispatchClass::Mandatory))]
		pub fn create_order(
			origin: OriginFor<T>,
			data: OrderInherentData,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			let Some(data) = data.data else {
				return Ok(().into());
			};

			if BulkMode::<T>::get().is_some() {
				return Ok(().into());
			}
			if Authorities::<T>::get().len().is_zero() {
				return Ok(().into());
			}

			if !T::OrderPlacementCriteria::should_place_order() {
				// Not supposed to place an order.
				//
				// Short-circuit: the order placer doesn't get rewarded.
				return Ok(().into());
			}

			let slot = Self::slot_at(data.relay_height);
			if slot <= PreviousSlot::<T>::get() {
				// The order placer doesn't get rewarded multiple times for blocks produced in the
				// same slot.
				return Ok(().into())
			}

			let relay_state_proof = RelayChainStateProof::new(
				data.para_id,
				data.relay_state_root,
				data.relay_storage_proof,
			)
			.expect("Invalid relay chain state proof");

			let result = T::OrdersPlaced::orders_placed(relay_state_proof, data.para_id);

			let Some(order_placer) = Self::order_placer_at(data.relay_height) else {
				return Ok(().into());
			};

			let order_placer_acc = pallet_session::KeyOwner::<T>::get((
				sp_application_crypto::key_types::AURA,
				order_placer.to_raw_vec(),
			))
			.ok_or(Error::<T>::FailedToGetOrderPlacerAccount)?;

			if !result.into_iter().any(|(_, ordered_by)| {
				// In most implementations the validator id is same as account id.
				<T as pallet_session::Config>::ValidatorIdOf::convert(ordered_by.clone()) ==
					Some(order_placer_acc.clone())
			}) {
				return Ok(().into());
			};

			T::OnReward::reward(T::ToAccountId::convert(order_placer_acc));
			PreviousSlot::<T>::set(slot);

			Ok(().into())
		}
		/// Set the slot width for on-demand blocks.
		///
		/// Parameters:
		/// - `origin`: Must be Root or pass `AdminOrigin`.
		/// - `width`: The slot width in relay chain blocks.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_slot_width())]
		pub fn set_slot_width(origin: OriginFor<T>, width: u32) -> DispatchResult {
			T::AdminOrigin::ensure_origin_or_root(origin)?;

			SlotWidth::<T>::set(width.clone());
			Self::deposit_event(Event::SlotWidthSet { width });

			Ok(())
		}

		/// Set the threshold parameter.
		///
		/// Parameters:
		/// - `origin`: Must be Root or pass `AdminOrigin`.
		/// - `parameter`: The threshold parameter.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_threshold_parameter())]
		pub fn set_threshold_parameter(
			origin: OriginFor<T>,
			parameter: T::ThresholdParameter,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin_or_root(origin)?;

			ThresholdParameter::<T>::set(parameter.clone());
			Self::deposit_event(Event::ThresholdParameterSet { parameter });

			Ok(())
		}

		/// Set the bulk mode.
		///
		/// Parameters:
		/// - `origin`: Must be Root or pass `AdminOrigin`.
		/// - `bulk_mode`: Defines whether we want to switch to bulk mode.
		#[pallet::call_index(3)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_bulk_mode())]
		pub fn set_bulk_mode(origin: OriginFor<T>, bulk_mode: bool) -> DispatchResult {
			T::AdminOrigin::ensure_origin_or_root(origin)?;

			if bulk_mode {
				BulkMode::<T>::set(Some(()));
			} else {
				BulkMode::<T>::kill();
			}

			Self::deposit_event(Event::BulkModeSet { bulk_mode });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn order_placer_at(relay_height: relay_chain::BlockNumber) -> Option<T::AuthorityId> {
			let authorities = Authorities::<T>::get();

			let slot = Self::slot_at(relay_height);
			let indx = slot % authorities.len() as u128;

			let authority_id = authorities.get(indx as usize).cloned();

			authority_id
		}

		fn slot_at(relay_height: relay_chain::BlockNumber) -> u128 {
			let slot_width = SlotWidth::<T>::get();
			let slot: u128 = (relay_height >> slot_width).saturated_into();
			slot
		}
	}

	impl<T: Config>
		OnReward<T::AccountId, <T::Currency as Inspect<T::AccountId>>::Balance, T::RewardSize>
		for Pallet<T>
	{
		fn reward(rewardee: T::AccountId) {
			let reward_size = T::RewardSize::reward_size();
			if T::Currency::mint_into(&rewardee, reward_size).is_err() {
				return;
			};
			Self::deposit_event(Event::OrderPlacerRewarded {
				order_placer: rewardee,
				reward: reward_size,
			});
		}
	}
}

pub struct FixedReward<Balance: BalanceT, Amount: Get<Balance>>(PhantomData<(Balance, Amount)>);
impl<Balance: BalanceT, Amount: Get<Balance>> RewardSize<Balance> for FixedReward<Balance, Amount> {
	fn reward_size() -> Balance {
		Balance::default().saturating_add(Amount::get())
	}
}

pub struct FeeBasedCriteria<T, BaseWeight>(PhantomData<(T, BaseWeight)>)
where
	T: Config<ThresholdParameter = BalanceOf<T>> + pallet_transaction_payment::Config,
	BaseWeight: Get<Weight>,
	T::OnChargeTransaction: OnChargeTransaction<T, Balance = BalanceOf<T>>;

impl<T, BaseWeight> RuntimeOrderCriteria for FeeBasedCriteria<T, BaseWeight>
where
	T: Config<ThresholdParameter = BalanceOf<T>> + pallet_transaction_payment::Config,
	BaseWeight: Get<Weight>,
	T::OnChargeTransaction: OnChargeTransaction<T, Balance = BalanceOf<T>>,
{
	fn should_place_order() -> bool {
		let weight = frame_system::Pallet::<T>::block_weight();
		let total_weight = weight.total();

		let unadjusted_weight_fee =
			pallet_transaction_payment::Pallet::<T>::weight_to_fee(total_weight);
		let multiplier = pallet_transaction_payment::NextFeeMultiplier::<T>::get();
		// final adjusted weight fee.
		let adjusted_weight_fee = multiplier.saturating_mul_int(unadjusted_weight_fee);

		let all_xt_len = frame_system::AllExtrinsicsLen::<T>::get().unwrap_or_default();
		let len_fee = pallet_transaction_payment::Pallet::<T>::length_to_fee(all_xt_len);

		let base_weight = pallet_transaction_payment::Pallet::<T>::weight_to_fee(BaseWeight::get());

		let total_fees = base_weight.saturating_add(adjusted_weight_fee).saturating_add(len_fee);

		total_fees >= ThresholdParameter::<T>::get()
	}
}

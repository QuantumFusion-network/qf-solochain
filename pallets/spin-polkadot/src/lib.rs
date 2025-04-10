//! # SPIN pallet for obtaining secure finality from Polkadot to SPIN
//!
//! This pallet defines the structure of alive messages and the state machine
//! for the slow chain. It automatically triggers state transitions based on
//! received messages and passed blocks. Extrinsic allow handling of alive
//! messages.
//!
//! ## State Machine
//!
//! The pallet implements a state machine with two states:
//!
//! * `Operational`: The normal operating mode. In this state:
//!   - Fastchain processes blocks and accepts alive messages
//!   - Each received alive message updates the `last_alive_message_block_number`
//!   - If no alive message is received for `TimeoutBlocks` blocks, the system transitions to
//!     `CoolDown`
//!
//! * `CoolDown`: A recovery period after a timeout. In this state:
//!   - Validators provide last known notarizations
//!   - Alive messages are not accepted until the cool down period ends
//!   - After `CoolDownPeriodBlocks` blocks, the system automatically transitions back to
//!     `Operational`

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame::pallet]
pub mod pallet {
	use frame::{prelude::*, runtime::types_common::BlockNumber, traits::Header};
	use polkadot_parachain_primitives::primitives::HeadData;
	use sp_consensus_grandpa::Commit;

	/// The validation data provides information about how to create the inputs
	/// for validation of a candidate.
	///
	/// See the [original reference](https://github.com/paritytech/polkadot-sdk/blob/polkadot-stable2412-2/polkadot/primitives/src/v8/mod.rs#L663)
	#[derive(CloneNoBound, Encode, Decode, PartialEqNoBound, RuntimeDebugNoBound, TypeInfo)]
	pub struct PersistedValidationData<
		H: Clone + Debug + PartialEq = H256,
		N: Clone + Debug + PartialEq = BlockNumber,
	> {
		/// The parent head-data.
		pub parent_head: HeadData,
		/// The relay-chain block number this is in the context of.
		pub relay_parent_number: N,
		/// The relay-chain block storage root this is in the context of.
		pub relay_parent_storage_root: H,
		/// The maximum legal size of a POV block, in bytes.
		pub max_pov_size: u32,
	}

	/// The inherent data that is passed by the fastchain validator to the
	/// parachain runtime.
	///
	///  See the [original reference](https://github.com/paritytech/polkadot-sdk/blob/polkadot-stable2412-2/cumulus/primitives/parachain-inherent/src/lib.rs#L46)
	#[derive(CloneNoBound, Encode, Decode, PartialEqNoBound, RuntimeDebugNoBound, TypeInfo)]
	pub struct FastchainInherentData {
		pub validation_data: PersistedValidationData,
		/// A storage proof of a predefined set of keys from the relay-chain.
		///
		/// Specifically this witness contains the data for:
		///
		/// - the current slot number at the given relay parent
		/// - active host configuration as per the relay parent,
		/// - the relay dispatch queue sizes
		/// - the list of egress HRMP channels (in the list of recipients form)
		/// - the metadata for the egress HRMP channels
		pub relay_chain_state: sp_trie::StorageProof,
		// /// Downward messages in the order they were sent.
		// pub downward_messages: Vec<InboundDownwardMessage>,
		// /// HRMP messages grouped by channels. The messages in the inner vec must be in order
		// they /// were sent. In combination with the rule of no more than one message in a
		// channel per block, /// this means `sent_at` is **strictly** greater than the previous
		// one (if any). pub horizontal_messages: BTreeMap<ParaId, Vec<InboundHrmpMessage>>,
	}

	/// A GRANDPA justification for block finality, it includes a commit message
	/// and an ancestry proof including all headers routing all precommit
	/// target blocks to the commit target block. Due to the current voting
	/// strategy the precommit targets should be the same as the commit target,
	/// since honest voters don't vote past authority set change blocks.
	///
	/// See the [original reference](https://github.com/paritytech/polkadot-sdk/blob/polkadot-stable2412-2/substrate/primitives/consensus/grandpa/src/lib.rs#L133)
	#[derive(CloneNoBound, Encode, Decode, PartialEqNoBound, RuntimeDebugNoBound, TypeInfo)]
	#[scale_info(skip_type_params(MaxVotesAncestries))]
	pub struct GrandpaJustification<MaxVotesAncestries: Get<u32>, H: Header> {
		pub round: u64,
		// TODO: replace with bounded size data structure
		pub commit: Commit<H>,
		pub votes_ancestries: BoundedVec<H, MaxVotesAncestries>,
	}

	/// Alive message proof combining `FastchainInherentData` and
	/// `GrandpaJustification`.
	///
	/// Should be sent from a fastchain node to the parachain SPIN pallet via an
	/// extrinsic call.
	#[derive(CloneNoBound, Encode, Decode, PartialEqNoBound, RuntimeDebugNoBound, TypeInfo)]
	#[scale_info(skip_type_params(MaxVotesAncestries))]
	pub struct AliveMessageProof<MaxVotesAncestries: Get<u32>, H: Header> {
		pub fastchain_inherent_data: FastchainInherentData,
		pub grandpa_justification: GrandpaJustification<MaxVotesAncestries, H>,
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		#[pallet::constant]
		type TimeoutBlocks: Get<BlockNumberFor<Self>>;

		#[pallet::constant]
		type CoolDownPeriodBlocks: Get<BlockNumberFor<Self>>;

		#[pallet::constant]
		type MaxVotesAncestries: Get<u32>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq)]
	pub enum SlowchainState<BlockNumber: Clone + PartialEq> {
		Operational { last_alive_message_block_number: BlockNumber },
		CoolDown { start_block_number: BlockNumber },
	}

	/// State of the slowchain: Operational, CoolDown or None
	#[pallet::storage]
	pub type State<T: Config> = StorageValue<_, SlowchainState<BlockNumberFor<T>>>;

	/// Set of fastchain validators used to verify alive messages and elect a
	/// leader
	#[pallet::storage]
	pub type ValidatorSet<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, (), ValueQuery>;

	/// Last seen alive message
	#[pallet::storage]
	pub type LastAliveMessage<T: Config> =
		StorageValue<_, AliveMessageProof<T::MaxVotesAncestries, HeaderFor<T>>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new alive message has been received.
		HeartbeatReceived { block_number: BlockNumberFor<T>, who: T::AccountId },
		/// `CoolDown` mode has been triggered.
		StartedCoolDown {
			block_number: BlockNumberFor<T>,
			last_alive_message_block_number: BlockNumberFor<T>,
		},
		/// `CoolDown` mode has ended.
		FinishedCoolDown { block_number: BlockNumberFor<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		IntegerOverflow,
		CoolDownPeriod,
		BlockNumberDecreased,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(current_block_number: BlockNumberFor<T>) -> Weight {
			let mut weight = Weight::zero();
			weight += T::DbWeight::get().reads(1);

			match <State<T>>::get() {
				Some(SlowchainState::Operational { last_alive_message_block_number }) => {
					let timeout_blocks = T::TimeoutBlocks::get();

					let deadline_block_number =
						match last_alive_message_block_number.checked_add(&timeout_blocks) {
							Some(deadline_block_number) => deadline_block_number,
							None => return weight,
						};

					if current_block_number > deadline_block_number {
						<State<T>>::put(SlowchainState::CoolDown {
							start_block_number: current_block_number,
						});
						weight += T::DbWeight::get().writes(1);
						Self::deposit_event(Event::StartedCoolDown {
							block_number: current_block_number,
							last_alive_message_block_number,
						});
					}
				},
				Some(SlowchainState::CoolDown { start_block_number }) => {
					let cool_down_period_blocks = T::CoolDownPeriodBlocks::get();
					let cool_down_period_deadline =
						match start_block_number.checked_add(&cool_down_period_blocks) {
							Some(cool_down_period_deadline) => cool_down_period_deadline,
							None => return weight,
						};
					if current_block_number > cool_down_period_deadline {
						<State<T>>::put(SlowchainState::Operational {
							last_alive_message_block_number: current_block_number,
						});
						weight += T::DbWeight::get().writes(1);
						Self::deposit_event(Event::FinishedCoolDown {
							block_number: current_block_number,
						});
					}
				},
				None => {
					// Start as `Operational` for first block
					<State<T>>::put(SlowchainState::Operational {
						last_alive_message_block_number: current_block_number,
					});
					weight += T::DbWeight::get().writes(1);
				},
			}

			weight
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submit an alive message and postpone the transition to `CoolDown`
		/// mode.
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
		pub fn submit_alive_message(
			origin: OriginFor<T>,
			proof: AliveMessageProof<T::MaxVotesAncestries, HeaderFor<T>>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			// TODO: validate GRANDPA justification proof on client side
			// proof.validate();

			let current_block_number = frame_system::Pallet::<T>::block_number();

			match <State<T>>::get() {
				Some(SlowchainState::Operational { last_alive_message_block_number }) => {
					ensure!(
						current_block_number > last_alive_message_block_number,
						Error::<T>::BlockNumberDecreased
					);
					<State<T>>::put(SlowchainState::Operational {
						last_alive_message_block_number: current_block_number,
					});
					Self::deposit_event(Event::HeartbeatReceived {
						block_number: current_block_number,
						who,
					});
				},
				Some(SlowchainState::CoolDown { start_block_number }) => {
					let cool_down_period_blocks = T::CoolDownPeriodBlocks::get();
					let cool_down_period_deadline_block_number: BlockNumberFor<T> =
						start_block_number
							.checked_add(&cool_down_period_blocks)
							.ok_or(Error::<T>::IntegerOverflow)?;

					if current_block_number > cool_down_period_deadline_block_number {
						<State<T>>::put(SlowchainState::Operational {
							last_alive_message_block_number: current_block_number,
						});
						Self::deposit_event(Event::FinishedCoolDown {
							block_number: current_block_number,
						});
					} else {
						return Err(Error::<T>::CoolDownPeriod.into());
					}
				},
				None => {
					// Start as `Operational` for first heartbeat
					<State<T>>::put(SlowchainState::Operational {
						last_alive_message_block_number: current_block_number,
					});
					Self::deposit_event(Event::HeartbeatReceived {
						block_number: current_block_number,
						who,
					});
				},
			}

			<LastAliveMessage<T>>::put(proof);

			Ok(().into())
		}
	}
}

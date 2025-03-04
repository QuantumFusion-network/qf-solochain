//! # SPIN Pallet
//!
//! A pallet with minimal functionality to help developers understand the essential components of
//! writing a FRAME pallet. It is typically used in beginner tutorials or in Polkadot SDK template
//! as a starting point for creating a new pallet and **not meant to be used in production**.
//!
//! ## Overview
//!
//! This template pallet contains basic examples of:
//! - declaring a storage item that stores a single block-number
//! - declaring and using events
//! - declaring and using errors
//! - a dispatchable function that allows a user to set a new value to storage and emits an event
//!   upon success
//! - another dispatchable function that causes a custom error to be thrown
//!
//! Each pallet section is annotated with an attribute using the `#[pallet::...]` procedural macro.
//! This macro generates the necessary code for a pallet to be aggregated into a FRAME runtime.
//!
//! To get started with pallet development, consider using this tutorial:
//!
//! <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html>
//!
//! And reading the main documentation of the `frame` crate:
//!
//! <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/polkadot_sdk/frame_runtime/index.html>
//!
//! And looking at the frame [`kitchen-sink`](https://paritytech.github.io/polkadot-sdk/master/pallet_example_kitchensink/index.html)
//! pallet, a showcase of all pallet macros.
//!
//! ### Pallet Sections
//!
//! The pallet sections in this template are:
//!
//! - A **configuration trait** that defines the types and parameters which the pallet depends on
//!   (denoted by the `#[pallet::config]` attribute). See: [`Config`].
//! - A **means to store pallet-specific data** (denoted by the `#[pallet::storage]` attribute).
//!   See: [`storage_types`].
//! - A **declaration of the events** this pallet emits (denoted by the `#[pallet::event]`
//!   attribute). See: [`Event`].
//! - A **declaration of the errors** that this pallet can throw (denoted by the `#[pallet::error]`
//!   attribute). See: [`Error`].
//! - A **set of dispatchable functions** that define the pallet's functionality (denoted by the
//!   `#[pallet::call]` attribute). See: [`dispatchables`].
//!
//! Run `cargo doc --package pallet-template --open` to view this pallet's documentation.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

const LOG_TARGET: &str = "runtime::template";

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

// <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/polkadot_sdk/frame_runtime/index.html>
// <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html>
//
// To see a full list of `pallet` macros and their use cases, see:
// <https://paritytech.github.io/polkadot-sdk/master/pallet_example_kitchensink/index.html>
// <https://paritytech.github.io/polkadot-sdk/master/frame_support/pallet_macros/index.html>
#[frame::pallet]
pub mod pallet {
    use frame::{prelude::*, runtime::types_common::BlockNumber, traits::Header};
    use polkadot_parachain_primitives::primitives::HeadData;
    use sp_runtime::Vec;
    // use sp_consensus_grandpa::GrandpaJustification;
    use finality_grandpa::{voter_set::VoterSet, Error as GrandpaError};
    use sp_consensus_grandpa::{AuthorityId, Commit};
    use sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor};

    #[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug)]
    pub struct PersistedValidationData<H = H256, N = BlockNumber> {
        /// The parent head-data.
        pub parent_head: HeadData,
        /// The relay-chain block number this is in the context of.
        pub relay_parent_number: N,
        /// The relay-chain block storage root this is in the context of.
        pub relay_parent_storage_root: H,
        /// The maximum legal size of a POV block, in bytes.
        pub max_pov_size: u32,
    }

    /// The inherent data that is passed by the collator to the parachain runtime.
    #[derive(Encode, Decode, RuntimeDebug, Clone, PartialEq, TypeInfo)]
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

    // substrate/primitives/consensus/grandpa/src/lib.rs
    #[derive(Clone, Encode, Decode, RuntimeDebug, PartialEq, Eq, TypeInfo)]
    pub struct GrandpaJustification<H: Header> {
        pub round: u64,
        pub commit: Commit<H>,
        pub votes_ancestries: Vec<H>,
    }

    #[derive(Encode, Decode, RuntimeDebug, Clone, PartialEq, TypeInfo)]
    pub struct AliveMessageProof<H: Header> {
        pub fastchain_inherent_data: FastchainInherentData,
        pub grandpa_justification: GrandpaJustification<H>,
    }

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// A type representing the weights required by the dispatchables of this pallet.
        type WeightInfo: crate::weights::WeightInfo;

        #[pallet::constant]
        type TimeoutBlocks: Get<BlockNumber>;

        #[pallet::constant]
        type CoolDownPeriodBlocks: Get<BlockNumber>;
    }

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[derive(
        Encode, Decode, MaxEncodedLen, TypeInfo, CloneNoBound, PartialEqNoBound, DefaultNoBound,
    )]
    #[scale_info(skip_type_params(T))]
    pub enum SlowchainState<T: Config> {
        #[default]
        Operational {
            last_alive_message_block_number: BlockNumberFor<T>,
        },
        CoolDown {
            start_block_number: BlockNumberFor<T>,
        },
        SlowMode {
            start_block_number: BlockNumberFor<T>,
        },
    }

    #[pallet::storage]
    pub type State<T: Config> = StorageValue<_, SlowchainState<T>, ValueQuery>;

    // TODO: reuse pallet-staking
    #[pallet::storage]
    pub type ValidatorSet<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, (), ValueQuery>;

    #[pallet::storage]
    pub type AuthorityList<T: Config> =
        StorageValue<_, sp_consensus_grandpa::AuthorityList, ValueQuery>;

    /// Pallets use events to inform users when important changes are made.
    /// <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#event-and-error>
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Heartbeat {
            block_number: BlockNumberFor<T>,
            who: T::AccountId,
        },
        StartedCoolDown {
            block_number: BlockNumberFor<T>,
            last_alive_message_block_number: BlockNumberFor<T>,
        },
        FinishedCoolDown {
            block_number: BlockNumberFor<T>,
        },
    }

    /// Errors inform users that something went wrong.
    /// <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#event-and-error>
    #[pallet::error]
    pub enum Error<T> {
        IntegerOverflow,
        BlockTimeout,
        CoolDownPeriod,
        Unimplemented,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(current_block_number: BlockNumberFor<T>) -> Weight {
            let weight = Weight::zero();

            match <State<T>>::get() {
                SlowchainState::Operational {
                    last_alive_message_block_number,
                } => {
                    let timeout_blocks: BlockNumberFor<T> = T::TimeoutBlocks::get().into();

                    let deadline_block_number: Option<BlockNumberFor<T>> =
                        last_alive_message_block_number.checked_add(&timeout_blocks);
                    let deadline_block_number = match deadline_block_number {
                        Some(deadline_block_number) => deadline_block_number,
                        None => return weight,
                    };

                    if current_block_number > deadline_block_number {
                        log::info!(
                            target: crate::LOG_TARGET,
                            "on_initialize: alive message deadline exceeded. Starting cool down"
                        );
                        <State<T>>::put(SlowchainState::CoolDown {
                            start_block_number: current_block_number,
                        });
                        Self::deposit_event(Event::StartedCoolDown {
                            block_number: current_block_number,
                            last_alive_message_block_number,
                        });
                    }
                }
                SlowchainState::CoolDown { start_block_number } => {
                    let cool_down_period_blocks: BlockNumberFor<T> =
                        T::CoolDownPeriodBlocks::get().into();
                    let cool_down_period_deadline: Option<BlockNumberFor<T>> =
                        start_block_number.checked_add(&cool_down_period_blocks);
                    let cool_down_period_deadline = match cool_down_period_deadline {
                        Some(cool_down_period_deadline) => cool_down_period_deadline,
                        None => return weight,
                    };
                    if current_block_number > cool_down_period_deadline {
                        <State<T>>::put(SlowchainState::Operational {
                            last_alive_message_block_number: current_block_number,
                        });
                        Self::deposit_event(Event::FinishedCoolDown {
                            block_number: current_block_number,
                        });
                    }
                }
                _ => (),
            }

            weight // TODO
        }
    }

    /// Dispatchable functions allows users to interact with the pallet and invoke state changes.
    /// These functions materialize as "extrinsics", which are often compared to transactions.
    /// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
    /// <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#dispatchables>
    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        NumberFor<<T as frame_system::Config>::Block>: finality_grandpa::BlockNumberOps,
    {
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
        pub fn handle_alive_message(
            origin: OriginFor<T>,
            proof: AliveMessageProof<HeaderFor<T>>,
            set_id: u64,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            // TODO: validate GRANDPA justification proof on client side
            // proof.validate();

            let current_block_number = frame_system::Pallet::<T>::block_number();

            match <State<T>>::get() {
                SlowchainState::Operational {
                    last_alive_message_block_number,
                } => {
                    // i'm online pallet
                    ensure!(
                        current_block_number > last_alive_message_block_number,
                        "BlockNumberDecreased"
                    ); // '>=' ?
                    <State<T>>::put(SlowchainState::Operational {
                        last_alive_message_block_number: current_block_number,
                    });
                    Self::deposit_event(Event::Heartbeat {
                        block_number: current_block_number,
                        who,
                    });
                }
                SlowchainState::CoolDown { start_block_number } => {
                    let cool_down_period_blocks: BlockNumberFor<T> =
                        T::CoolDownPeriodBlocks::get().into();
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
                }
                SlowchainState::SlowMode {
                    start_block_number: _,
                } => {
                    return Err(Error::<T>::Unimplemented.into());
                }
            }

            Ok(().into())
        }
    }
}

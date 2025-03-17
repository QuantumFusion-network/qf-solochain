use crate::{Config, Error};

use bp_header_chain::{AuthoritySet, ChainWithGrandpa};
use codec::{Decode, Encode, MaxEncodedLen};
use frame::prelude::*;
// use frame_support::{traits::Get, BoundedVec, CloneNoBound, RuntimeDebugNoBound};
use scale_info::TypeInfo;
use sp_consensus_grandpa::{AuthorityId, AuthorityList, AuthorityWeight, SetId};
use sp_std::marker::PhantomData;

/// A bounded list of Grandpa authorities with associated weights.
pub type StoredAuthorityList<MaxBridgedAuthorities> =
    BoundedVec<(AuthorityId, AuthorityWeight), MaxBridgedAuthorities>;

/// Adapter for using `T::BridgedChain::MAX_BRIDGED_AUTHORITIES` in `BoundedVec`.
pub struct StoredAuthorityListLimit<T>(PhantomData<T>);

impl<T: Config> Get<u32> for StoredAuthorityListLimit<T> {
    fn get() -> u32 {
        T::BridgedChain::MAX_AUTHORITIES_COUNT
    }
}

/// A bounded GRANDPA Authority List and ID.
#[derive(CloneNoBound, Decode, Encode, Eq, TypeInfo, MaxEncodedLen, RuntimeDebugNoBound)]
#[scale_info(skip_type_params(T))]
pub struct StoredAuthoritySet<T: Config> {
    /// List of GRANDPA authorities for the current round.
    pub authorities: StoredAuthorityList<StoredAuthorityListLimit<T>>,
    /// Monotonic identifier of the current GRANDPA authority set.
    pub set_id: SetId,
}

impl<T: Config> StoredAuthoritySet<T> {
    /// Try to create a new bounded GRANDPA Authority Set from unbounded list.
    ///
    /// Returns error if number of authorities in the provided list is too large.
    pub fn try_new(authorities: AuthorityList, set_id: SetId) -> Result<Self, Error<T>> {
        Ok(Self {
            authorities: TryFrom::try_from(authorities)
                .map_err(|_| Error::TooManyAuthoritiesInSet)?,
            set_id,
        })
    }

    /// Returns number of bytes that may be subtracted from the PoV component of
    /// `submit_finality_proof` call, because the actual authorities set is smaller than the maximal
    /// configured.
    ///
    /// Maximal authorities set size is configured by the `MaxBridgedAuthorities` constant from
    /// the pallet configuration. The PoV of the call includes the size of maximal authorities
    /// count. If the actual size is smaller, we may subtract extra bytes from this component.
    pub fn unused_proof_size(&self) -> u64 {
        // we can only safely estimate bytes that are occupied by the authority data itself. We have
        // no means here to compute PoV bytes, occupied by extra trie nodes or extra bytes in the
        // whole set encoding
        let single_authority_max_encoded_len =
            <(AuthorityId, AuthorityWeight)>::max_encoded_len() as u64;
        let extra_authorities =
            T::BridgedChain::MAX_AUTHORITIES_COUNT.saturating_sub(self.authorities.len() as _);
        single_authority_max_encoded_len.saturating_mul(extra_authorities as u64)
    }
}

impl<T: Config> PartialEq for StoredAuthoritySet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.set_id == other.set_id && self.authorities == other.authorities
    }
}

impl<T: Config> Default for StoredAuthoritySet<T> {
    fn default() -> Self {
        StoredAuthoritySet {
            authorities: BoundedVec::default(),
            set_id: 0,
        }
    }
}

impl<T: Config> From<StoredAuthoritySet<T>> for AuthoritySet {
    fn from(t: StoredAuthoritySet<T>) -> Self {
        AuthoritySet {
            authorities: t.authorities.into(),
            set_id: t.set_id,
        }
    }
}

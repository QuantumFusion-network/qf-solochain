use crate::cli::CliChain;
use pallet_bridge_parachains::{RelayBlockHash, RelayBlockHasher, RelayBlockNumber};
use relay_substrate_client::{Chain, ChainWithTransactions, Parachain, RelayChain};
use strum::{EnumString, EnumVariantNames};
use substrate_relay_helper::{
    finality::SubstrateFinalitySyncPipeline, messages::SubstrateMessageLane,
    parachains::SubstrateParachainsPipeline,
};

/// Minimal bridge representation that can be used from the CLI.
/// It connects a source chain to a target chain.
pub trait CliBridgeBase: Sized {
    /// The source chain.
    type Source: Chain + CliChain;
    /// The target chain.
    type Target: ChainWithTransactions + CliChain;
}

/// Bridge representation that can be used from the CLI for relaying headers
/// from a relay chain to a relay chain.
pub trait RelayToRelayHeadersCliBridge: CliBridgeBase {
    /// Finality proofs synchronization pipeline.
    type Finality: SubstrateFinalitySyncPipeline<
        SourceChain = Self::Source,
        TargetChain = Self::Target,
    >;
}
//! Fastchain-to-SlowchainParachain headers sync entrypoint.
use crate::bridge::{CliBridgeBase, RelayToRelayHeadersCliBridge};
use substrate_relay_helper::{
    finality::SubstrateFinalitySyncPipeline,
    finality_base::{SubstrateFinalityPipeline, engine::Grandpa as GrandpaFinalityEngine},
};
substrate_relay_helper::generate_submit_finality_proof_call_builder!(
    EvochainFinalityToOwnershipParachain,
    EvochainFinalityToOwnershipParachainCallBuilder,
    crate::client_slowchain::RuntimeCall::BridgeEvochainGrandpa,
    crate::client_slowchain::BridgeGrandpaCall::submit_finality_proof
);

/// Description of Evochain -> Rococo finalized headers bridge.
#[derive(Clone, Debug)]
pub struct EvochainFinalityToOwnershipParachain;

impl SubstrateFinalityPipeline for EvochainFinalityToOwnershipParachain {
    type SourceChain = crate::client_fastchain::Evochain;
    type TargetChain = crate::client_slowchain::OwnershipParachain;

    type FinalityEngine = GrandpaFinalityEngine<Self::SourceChain>;
}

impl SubstrateFinalitySyncPipeline for EvochainFinalityToOwnershipParachain {
    type SubmitFinalityProofCallBuilder = EvochainFinalityToOwnershipParachainCallBuilder;
}

//// `Evochain` to `OwnershipParachain`  bridge definition.
pub struct EvochainToOwnershipParachainCliBridge {}

impl CliBridgeBase for EvochainToOwnershipParachainCliBridge {
    type Source = crate::client_fastchain::Evochain;
    type Target = crate::client_slowchain::OwnershipParachain;
}

impl RelayToRelayHeadersCliBridge for EvochainToOwnershipParachainCliBridge {
    type Finality = EvochainFinalityToOwnershipParachain;
}

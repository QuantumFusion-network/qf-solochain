// Copyright (C) Quantum Fusion Network, 2025.
// Copyright (C) Parity Technologies (UK) Ltd., until 2025.
// SPDX-License-Identifier: Apache-2.0

//! SPIN relay implementation.
#![forbid(missing_docs, unsafe_code)]
use std::{fmt::Debug, marker::PhantomData, pin::Pin, sync::Arc};

use codec::Codec;
use futures::prelude::*;

use sc_client_api::{BlockOf, backend::AuxStore};
use sc_consensus::{BlockImport, BlockImportParams, ForkChoiceStrategy, StateAction};
use sc_consensus_slots::{
    BackoffAuthoringBlocksStrategy, InherentDataProviderExt, SimpleSlotWorkerToSlotWorker,
    SlotInfo, StorageChanges,
};
use sc_telemetry::TelemetryHandle;
use sp_api::{Core, ProvideRuntimeApi};
use sp_application_crypto::AppPublic;
use sp_blockchain::HeaderBackend;
use sp_consensus::{BlockOrigin, Environment, Error as ConsensusError, Proposer, SelectChain};
use sp_consensus_slots::Slot;
use sp_inherents::CreateInherentDataProviders;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::{Block as BlockT, Header, Member, NumberFor};

pub use qfp_consensus_spin::{
    ConsensusLog, SPIN_ENGINE_ID, SessionLength, SlotDuration, SpinApi, SpinAuxData,
    digests::CompatibleDigestItem,
    inherents::{INHERENT_IDENTIFIER, InherentDataProvider, InherentType as SpinInherent},
};
pub use sc_consensus_slots::SlotProportion;
pub use sp_consensus::SyncOracle;

mod bridge;
mod cli;
mod fastchain_headers_to_slowchain_parachain;
mod client_fastchain;
mod client_slowchain;
mod chain_schema;

use relay_substrate_client::{AccountIdOf, AccountKeyPairOf};
use sp_core::Pair;
use structopt::StructOpt;
use strum::{EnumString, EnumVariantNames, VariantNames};

use fastchain_headers_to_slowchain_parachain::EvochainToOwnershipParachainCliBridge;
use relay_substrate_client::Client;
use relay_utils::metrics::{GlobalMetrics, StandaloneMetric};
use substrate_relay_helper::finality::SubstrateFinalitySyncPipeline;

use bridge::RelayToRelayHeadersCliBridge;
use chain_schema::*;

const LOG_TARGET: &str = "relay";

/// Start headers relayer process.
#[derive(StructOpt)]
pub struct RelayHeaders {
    /// A bridge instance to relay headers for.
    #[structopt(possible_values = RelayHeadersBridge::VARIANTS, case_insensitive = true)]
    bridge: RelayHeadersBridge,
    /// If passed, only mandatory headers (headers that are changing the GRANDPA authorities set)
    /// are relayed.
    #[structopt(long)]
    only_mandatory_headers: bool,
    #[structopt(flatten)]
    source: SourceConnectionParams,
    #[structopt(flatten)]
    target: TargetConnectionParams,
    #[structopt(flatten)]
    target_sign: TargetSigningParams,
    // #[structopt(flatten)]
    // prometheus_params: PrometheusParams,
}

#[derive(Debug, EnumString, EnumVariantNames)]
#[strum(serialize_all = "kebab_case")]
/// Headers relay bridge.
pub enum RelayHeadersBridge {
    EvochainToOwnershipParachain,
    RococoToEvochain,
}

trait HeadersRelayer: RelayToRelayHeadersCliBridge
where
    AccountIdOf<Self::Target>: From<<AccountKeyPairOf<Self::Target> as Pair>::Public>,
{
    /// Relay headers.
    async fn relay_headers(data: RelayHeaders) -> anyhow::Result<()> {
        let source_client = data.source.into_client::<Self::Source>().await?;
        let target_client = data.target.into_client::<Self::Target>().await?;
        let target_transactions_mortality = data.target_sign.target_transactions_mortality;
        let target_sign = data.target_sign.to_keypair::<Self::Target>()?;

        let metrics_params: relay_utils::metrics::MetricsParams =
            data.prometheus_params.into_metrics_params()?;
        GlobalMetrics::new()?.register_and_spawn(&metrics_params.registry)?;

        let target_transactions_params = substrate_relay_helper::TransactionParams {
            signer: target_sign,
            mortality: target_transactions_mortality,
        };
        Self::Finality::start_relay_guards(
            &target_client,
            &target_transactions_params,
            target_client.can_start_version_guard(),
        )
        .await?;

        substrate_relay_helper::finality::run::<Self::Finality>(
            source_client,
            target_client,
            data.only_mandatory_headers,
            target_transactions_params,
            metrics_params,
        )
        .await
    }
}

impl HeadersRelayer for EvochainToOwnershipParachainCliBridge {}
// impl HeadersRelayer for RococoToEvochainCliBridge {}

impl RelayHeaders {
    /// Run the command.
    pub async fn run(self) -> anyhow::Result<()> {
        match self.bridge {
            RelayHeadersBridge::EvochainToOwnershipParachain => {
                EvochainToOwnershipParachainCliBridge::relay_headers(self)
            }
            RelayHeadersBridge::RococoToEvochain => unimplemented!(),
        }
        .await
    }
}

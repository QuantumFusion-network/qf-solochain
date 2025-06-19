#!/bin/bash

# Params
SPEC_PATH="../chain-specs"
NODE="../../polkadot-sdk/target/release/polkadot"

# $NODE build-spec --disable-default-bootnode --chain=rococo-local > $SPEC_PATH/relaychain-spec.json
$NODE build-spec --disable-default-bootnode --chain $SPEC_PATH/relaychain-spec.json --raw > $SPEC_PATH/relaychain-spec-raw.json

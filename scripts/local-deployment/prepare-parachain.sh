#!/bin/bash

# Params
SPEC_PATH="../chain-specs"
NODE="../target/debug/qf-parachain-node"

cargo build -p qf-parachain-node
cargo build -p subkey

$NODE build-spec --disable-default-bootnode > $SPEC_PATH/parachain-spec.json
$NODE build-spec --disable-default-bootnode --chain $SPEC_PATH/parachain-spec.json --raw > $SPEC_PATH/parachain-spec-raw.json

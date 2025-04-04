#!/bin/bash

SPEC_PATH="../chain-specs"
DATA_PATH="../data-relay"
NODE="/Users/max/workspace/Rust/QF/polkadot-1.0.0/target/release/polkadot"

mkdir -p $DATA_PATH

# polkadot \
$NODE \
--port 40340 \
--rpc-port 9950 \
-d $DATA_PATH \
--chain $SPEC_PATH/relaychain-spec-raw.json --alice

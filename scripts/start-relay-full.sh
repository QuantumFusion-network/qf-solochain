#!/bin/bash

SPEC_PATH="../chain-specs"
DATA_PATH="../data-relay"
NODE="../../polkadot-sdk/target/release/polkadot"

mkdir -p $DATA_PATH

$NODE key generate-node-key --chain=rococo-local --base-path $DATA_PATH

# polkadot \
$NODE \
--port 40340 \
--rpc-port 9950 \
--force-authoring --rpc-cors=all --alice --discover-local \
-d $DATA_PATH \
--chain $SPEC_PATH/relaychain-spec-raw.json \
--rpc-methods unsafe

#!/bin/bash

SPEC_PATH="../chain-specs"
DATA_PATH="../data-relay-c1"
NODE="../../polkadot-sdk/target/release/polkadot"

mkdir -p $DATA_PATH

$NODE key generate-node-key --chain=rococo-local --base-path $DATA_PATH

# polkadot \
$NODE \
--validator \
--port 40341 \
--rpc-port 9951 \
--dave \
--insecure-validator-i-know-what-i-do \
-d $DATA_PATH \
--chain $SPEC_PATH/relaychain-spec-raw.json

#!/bin/bash

# Params
SPEC_PATH="../chain-specs"
DATA_PATH="../data-para-c1"
NODE="../target/debug/qf-parachain-node"
SUBKEY="../target/debug/subkey"
CHAIN_NAME="local_testnet"

# Generating the chain key
mkdir -p $DATA_PATH/chains/$CHAIN_NAME/network/
$SUBKEY generate-node-key > $DATA_PATH/chains/$CHAIN_NAME/network/secret_ed25519

# Run the node
$NODE \
--collator \
--port 40338 \
--rpc-port 9948 \
-d $DATA_PATH \
--chain $SPEC_PATH/parachain-spec-raw.json \
-- \
--chain $SPEC_PATH/relaychain-spec-raw.json --port 30335

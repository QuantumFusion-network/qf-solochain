#!/bin/bash

# Params
SPEC_PATH="../chain-specs"
DATA_PATH="../data-fast-v1"
NODE="../target/debug/qf-parachain-node"
# NODE="../target/debug/qf-node"
CHAIN_NAME="local_testnet_fast"
SUBKEY="../target/debug/subkey"

# Generating the chain key
mkdir -p $DATA_PATH/chains/$CHAIN_NAME/network/
$SUBKEY generate-node-key > $DATA_PATH/chains/$CHAIN_NAME/network/secret_ed25519

# Run the node
$NODE \
--fastchain \
--validator \
--port 40335 \
--rpc-port 9945 \
--chain $SPEC_PATH/fastchain-spec-raw.json \
--bob \
-d $DATA_PATH

#!/bin/bash

# Params
SPEC_PATH="../chain-specs"
DATA_PATH="../data-para-full"
NODE="../target/debug/qf-parachain-node"

# Run the node
$NODE \
--port 40337 \
--rpc-port 9947 \
-d $DATA_PATH \
--chain $SPEC_PATH/parachain-spec-raw.json \
-- \
--chain $SPEC_PATH/relaychain-spec-raw.json --port 30337

#!/bin/bash

# Params
SPEC_PATH="../chain-specs"
DATA_PATH="../data-fast-full"
NODE="../target/debug/qf-parachain-node"
# NODE="../target/debug/qf-node"

# Run the node
$NODE \
--fastchain \
--port 40334 \
--rpc-port 9944 \
-d $DATA_PATH \
--chain $SPEC_PATH/fastchain-spec-raw.json

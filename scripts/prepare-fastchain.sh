#!/bin/bash

# Params
SPEC_PATH="../chain-specs"
# NODE_NAME="qf-parachain-node"
NODE_NAME="qf-node"
NODE="../target/debug/$NODE_NAME"

cargo build -p $NODE_NAME

# $NODE build-spec --fastchain --disable-default-bootnode > $SPEC_PATH/fastchain-spec.json
# $NODE build-spec --fastchain --disable-default-bootnode --chain $SPEC_PATH/fastchain-spec.json --raw > $SPEC_PATH/fastchain-spec-raw.json

$NODE build-spec --disable-default-bootnode > $SPEC_PATH/fastchain-spec.json
$NODE build-spec --disable-default-bootnode --chain $SPEC_PATH/fastchain-spec.json --raw > $SPEC_PATH/fastchain-spec-raw.json

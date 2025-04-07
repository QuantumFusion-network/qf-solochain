#!/bin/bash

# Params
SPEC_PATH="../chain-specs"
NODE="../target/debug/qf-node"

cargo build -p qf-node

$NODE build-spec --disable-default-bootnode > $SPEC_PATH/fastchain-spec.json
$NODE build-spec --disable-default-bootnode --chain $SPEC_PATH/fastchain-spec.json --raw > $SPEC_PATH/fastchain-spec-raw.json

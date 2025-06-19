#!/bin/bash

# Params
SPEC_PATH="../chain-specs"
NODE="../target/debug/qf-parachain-node"

cargo build -p qf-parachain-node
cargo build -p subkey

echo "Skip generating base spec? [y/n]"
read skip
if [$skip = "y"] || [$skip = "Y"]
then 
$NODE build-spec --disable-default-bootnode > $SPEC_PATH/parachain-spec.json
echo "Edit your config first and press ENTER for continue.."
read inp
fi
$NODE build-spec --disable-default-bootnode --chain $SPEC_PATH/parachain-spec.json --raw > $SPEC_PATH/parachain-spec-raw.json

$NODE export-genesis-state \
  --chain $SPEC_PATH/parachain-spec-raw.json  > $SPEC_PATH/genesis-state-2000

$NODE export-genesis-wasm \
  --chain $SPEC_PATH/parachain-spec-raw.json  > $SPEC_PATH/genesis-wasm-2000

$NODE export-genesis-wasm \
  -r --chain $SPEC_PATH/parachain-spec-raw.json  > $SPEC_PATH/genesis-wasm-2000-raw

#!/bin/zsh

SPEC_BUILDER="../target/release/chain-spec-builder"
SPEC_PATH="../chain-specs"
BUILD_PATH="../target/release"
NODE="../target/debug/qf-parachain-node"

$SPEC_BUILDER \
--chain-spec-path $SPEC_PATH/parachain-spec-builder-raw.json \
convert-to-raw \
$SPEC_PATH/parachain-spec-builder.json

$NODE export-genesis-state \
  --chain $SPEC_PATH/parachain-spec-builder-raw.json  > $SPEC_PATH/genesis-state-2000

$NODE export-genesis-wasm \
  --chain $SPEC_PATH/parachain-spec-builder-raw.json  > $SPEC_PATH/genesis-wasm-2000

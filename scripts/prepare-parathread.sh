#!/bin/zsh

SPEC_BUILDER="../target/release/chain-spec-builder"
SPEC_PATH="../chain-specs"
BUILD_PATH="../target/release"

$SPEC_BUILDER \
--chain-spec-path $SPEC_PATH/parachain-spec-builder.json \
create \
--relay-chain rococo-local \
--para-id 2000 \
--runtime $BUILD_PATH/wbuild/qf-parachain-runtime/qf_parachain_runtime.compact.compressed.wasm \
named-preset local_testnet

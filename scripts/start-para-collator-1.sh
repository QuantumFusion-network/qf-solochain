#!/bin/bash

# Params
SPEC_PATH="../chain-specs"
DATA_PATH="../data-para-c1"
NODE="../target/debug/qf-parachain-node"
SUBKEY="../target/debug/subkey"
CHAIN_NAME="qfpara"

cargo build -p qf-parachain-node

# Generating the chain key
mkdir -p $DATA_PATH/chains/$CHAIN_NAME/network/
$SUBKEY generate-node-key > $DATA_PATH/chains/$CHAIN_NAME/network/secret_ed25519

# Get relay chain peer ID
RELAY_PEER_ID=$(curl -s -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "system_localPeerId"}' \
  http://localhost:9950 | jq -r '.result')

echo "Relay PeerId is $RELAY_PEER_ID"

# Run the node
$NODE \
--collator \
--port 40338 \
--rpc-port 9948 \
-d $DATA_PATH \
--force-authoring --rpc-cors=all --alice \
--chain $SPEC_PATH/parachain-spec-builder-raw.json \
--discover-local \
--relaychain \
--chain $SPEC_PATH/relaychain-spec-raw.json --port 30335 \
--discover-local \
--bootnodes /ip4/127.0.0.1/tcp/40340/p2p/$RELAY_PEER_ID

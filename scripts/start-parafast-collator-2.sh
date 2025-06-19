#!/bin/bash

# Params
SPEC_PATH="../chain-specs"
DATA_PATH="../data-parafast-c2"
NODE="../target/debug/qf-parachain-node"
SUBKEY="../target/debug/subkey"
CHAIN_NAME="qfpara"
FAST_CHAIN_NAME="local_testnet_fast"

cargo build -p qf-parachain-node

# Generating the chain key
mkdir -p $DATA_PATH/chains/$CHAIN_NAME/network/
$SUBKEY generate-node-key > $DATA_PATH/chains/$CHAIN_NAME/network/secret_ed25519

# Generating the chain key
mkdir -p $DATA_PATH/chains/$FAST_CHAIN_NAME/network/
$SUBKEY generate-node-key > $DATA_PATH/chains/$FAST_CHAIN_NAME/network/secret_ed25519

# Get relay chain peer ID
RELAY_PEER_ID=$(curl -s -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "system_localPeerId"}' \
  http://localhost:9950 | jq -r '.result')

echo "Relay PeerId is $RELAY_PEER_ID"

# Run the node
$NODE \
--fastchain \
--validator \
--port 40336 \
--rpc-port 9946 \
--force-authoring --rpc-cors=all \
--discover-local \
--chain $SPEC_PATH/fastchain-spec-raw.json \
--bob \
-d $DATA_PATH \
\; \
--collator \
--port 40339 \
--rpc-port 9949 \
--bob \
--discover-local \
--force-authoring --rpc-cors=all \
-d $DATA_PATH \
--chain $SPEC_PATH/parachain-spec-builder-raw.json \
--relaychain \
--discover-local \
--chain $SPEC_PATH/relaychain-spec-raw.json --port 30336 \
--bootnodes /ip4/127.0.0.1/tcp/40340/p2p/$RELAY_PEER_ID

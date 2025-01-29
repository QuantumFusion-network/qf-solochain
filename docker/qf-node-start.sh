#!/bin/bash

if [ ! -z "${QF_NODE_NAME}" ]; then
	OPTIONAL_NAME="--name ${QF_NODE_NAME}"
fi

/opt/qf-solochain/qf-node \
	--port ${QF_PORT} \
	--base-path /opt/qf-solochain/node-data/qf-devnet-node/ \
	--chain qf-devnet \
	--node-key-file /opt/qf-solochain/node-data/qf-devnet-node/chains/qf-devnet/keystore/node-key \
	--rpc-methods safe \
	--rpc-port ${QF_RPC_PORT} \
	--rpc-cors all \
	${OPTIONAL_NAME} \
	--telemetry-url ${QF_TELEMETRY_URL}

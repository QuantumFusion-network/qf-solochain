# Executables and runtimes

Project with `cargo build --release` creates the following executables.

```console
$ find target/release -maxdepth 1 -executable -type f
target/release/qf-parachain-node
target/release/qf-node
```

- **qf-parachain-node**: This is the node implementation for running QF Network as a parachain.
 Parachains are specialized blockchains that connect to the Polkadot relay chain.
  This executable allows QF to operate as part of the Polkadot ecosystem, with features
   like XCM (Cross-Consensus Message) support for cross-chain communication, as seen in the XCM configuration files.

- **qf-node**: This is the standalone node implementation for the QF Network blockchain
 (referred to as "fastchain" in the documentation). It can run independently without connecting
  to a relay chain, and can operate in different modes including as a full node or as a validator.

## Build the node binary
For building the fastchain node binary, use
```bash
cargo build -p qf-node --release
```

And for parachain node, use

```bash
cargo build -p qf-parachain-node --release
```

## Make the chainspec
**NOTE:** Chainspec - You can read about chainspecs in [official documentation](https://docs.polkadot.com/develop/parachains/deployment/generate-chain-specs)

1. Define the path to the chainspec:

    ```bash
    export SPEC_PATH=./chainspecs   # or your own path
    ```

2. For building the chainspec, use:

    ```bash
    ./target/debug/qf-node build-spec --disable-default-bootnode --raw > $SPEC_PATH/fastchain-spec-raw.json
    ```

## Prepare the Key for Validator and Collator modes

**NOTE:** In this part, we describe how we manage the keys for this tutorial.
You can view the [official documentation with additional methods](https://docs.polkadot.com/infrastructure/running-a-validator/onboarding-and-offboarding/key-management/#generate-session-keys)

Not all nodes can generate the key as:
```bash
./target/debug/qf-node key generate-node-key
```
You should build the `subkey` for generating keys by command:
```bash
cargo build -p subkey --release
```

1. You must create the folders in `data` for storing the keys with the command:

    ```bash
    mkdir -p $DATA_PATH/chains/$CHAIN_NAME/network
    ```
    where
    - `DATA_PATH` - path to the data directory
    - `CHAIN_NAME` - name of the chain ("local_testnet" for default)

2. After that, you can generate the key as:

    ```bash
    ./target/debug/subkey generate-node-key > $DATA_PATH/chains/$CHAIN_NAME/network/secret_ed25519
    ```
    where
    - `DATA_PATH` - path to the data directory
    - `CHAIN_NAME` - name of the chain ("local_testnet" for default)

or:

```bash
./target/debug/qf-node key generate-node-key > $DATA_PATH/chains/$CHAIN_NAME/network/secret_ed25519
```

## Run the Fastchain Node
- As a full node (no need to generate the key):
```bash
./target/debug/qf-node --chain $SPEC_PATH/fastchain-spec-raw.json -d $DATA_PATH
```
- As validator (user Alice and need to generate the key):
```bash
./target/debug/qf-node --chain $SPEC_PATH/fastchain-spec-raw.json --validator --alice -d $DATA_PATH
```


You can also specify the ports:
- `--port <port>` - port for the node
- `--rpc-port <port>` - port for the P2P connection

## Run the parachain node
Before running the local parachain node, you need to have a running relaychain node.
To run the local relaychain, you can use this bash script:
```bash
#!/bin/bash

SPEC_PATH="../chain-specs"
DATA_PATH="../data-relay"
NODE="<path to your polkadot node>/polkadot"

mkdir -p $DATA_PATH

# polkadot \
$NODE \
--port 40340 \
--rpc-port 9950 \
-d $DATA_PATH \
--chain $SPEC_PATH/relaychain-spec-raw.json --alice
```

First, generate the chainspec, similar to how we did it for fastchain:
```bash
./target/debug/qf-parachain-node build-spec --disable-default-bootnode --raw > $SPEC_PATH/parachain-spec-raw.json
```

- As a full node (no need to generate the key)
```bash
./target/debug/qf-parachain-node --chain $SPEC_PATH/parachain-spec-raw.json -d $DATA_PATH -- --chain $SPEC_PATH/relaychain-spec-raw.json
```
- As a collator (user Alice and need to generate the key)
```bash
./target/debug/qf-parachain-node --chain $SPEC_PATH/parachain-spec-raw.json --collator --alice -d $DATA_PATH -- --chain $SPEC_PATH/relaychain-spec-raw.json
```

## Executables and runtimes

### Main node executable and runtime
The main node executable is `qf-node` and the runtime is `qf-runtime`.

### Build the node binary
For building the fastchain node binary use 
```bash
cargo build -p qf-node --release
```

or for parachain node use

```bash
cargo build -p qf-parachain-node --release
```

### Make the chainspec
1. Define the path to the chainspec
```bash
export SPEC_PATH=./chainspecs   # or your own path
```

2. For building the chainspec use 
```bash
./target/debug/qf-node build-spec --disable-default-bootnode > $SPEC_PATH/fastchain-spec.json
```

Make the raw version of the spec 
```bash
./target/debug/qf-node build-spec --chain $SPEC_PATH/fastchain-spec.json --raw > $SPEC_PATH/fastchain-spec-raw.json
```

### Prepare the key for validator and collator modes
Not all nodes can generate the key as 
```bash
./target/debug/qf-node key generate-node-key
```
and you should build the `subkey` for generating keys by command 
```bash
cargo build -p subkey --release
```

1. You must create the folders in `data` for storing the keys by command 
```bash
mkdir -p $DATA_PATH/chains/$CHAIN_NAME/network
```
where 
- `DATA_PATH` - path to the data directory
- `CHAIN_NAME` - name of the chain ("local_testnet" for default)

2. After that you can generate the key as 

```bash
./target/debug/subkey generate-node-key > $DATA_PATH/chains/$CHAIN_NAME/network/secret_ed25519
```
where 
- `DATA_PATH` - path to the data directory
- `CHAIN_NAME` - name of the chain ("local_testnet" for default)

or

```bash
./target/debug/qf-node key generate-node-key > $DATA_PATH/chains/$CHAIN_NAME/network/secret_ed25519
```

### Run the fastchain node
- As full node (no need to generate the key)
```bash
./target/debug/qf-node --chain $SPEC_PATH/fastchain-spec-raw.json -d $DATA_PATH
```
- As validator (user Alice and need to generate the key)
```bash
./target/debug/qf-node --chain $SPEC_PATH/fastchain-spec-raw.json --validator --alice -d $DATA_PATH
```


Also you can specify the ports:
- `--port <port>` - port for the node
- `--rpc-port <port>` - port for the RPC

### Run the parachain node
Before run the parachain node you need to run the relaychain node.
For run the local relaychain you can customize and use this bash script:
```bash
#!/bin/bash

SPEC_PATH="../chain-specs"
DATA_PATH="../data-relay"
NODE="<pato to yoyr polkadot node>/polkadot"

mkdir -p $DATA_PATH

# polkadot \
$NODE \
--port 40340 \
--rpc-port 9950 \
-d $DATA_PATH \
--chain $SPEC_PATH/relaychain-spec-raw.json --alice
```

First generate the chainspec as for fastchain but by 
```bash
./target/debug/qf-parachain-node build-spec --disable-default-bootnode > $SPEC_PATH/parachain-spec.json
./target/debug/qf-parachain-node build-spec --chain $SPEC_PATH/parachain-spec.json --raw > $SPEC_PATH/parachain-spec-raw.json
```

- As full node (no need to generate the key)
```bash
./target/debug/qf-parachain-node --chain $SPEC_PATH/parachain-spec-raw.json -d $DATA_PATH -- --chain $SPEC_PATH/relaychain-spec-raw.json
```
- As collator (user Alice and need to generate the key)
```bash
./target/debug/qf-parachain-node --chain $SPEC_PATH/parachain-spec-raw.json --collator --alice -d $DATA_PATH -- --chain $SPEC_PATH/relaychain-spec-raw.json
```



# Local testing

For local testing you should have:
1. Fastchain
2. Relaychain and qf-parachain
3. Spin-finality-relayer

## Start local setup

### 1. Start Fastchain

Build fastchain node:

```bash
cargo b -r -p qf-node
```

and start it:

```bash
./target/release/qf-node --dev --state-pruning archive
```

### 2. Start Relaychain and qf-parachain

To start both we will use zombienet tool(https://github.com/paritytech/zombienet).
To download and install it use comands that described in repository.

Firstly need to download (or build) Polkadot binary from Polkadot-sdk repository (https://github.com/paritytech/polkadot-sdk)
Better to use the same version as we import in ./Cargo.toml.

To build(at Polkadot-sdk directory) you should use:
```bash
cargo b -r -p polkadot
```

Also we need build qf-parachain-node(at qf-solochain directory):
```bash
cargo b -r -p qf-parachain-node
```

For default recomended to use zombienet file ./parachain/zombient.toml.
If it needed to use custom chain-spec - you can add it like this:

```toml
[[parachains]]
id = 4775
chain_spec_path = "./node/src/res/qf-local-para.raw.json" # Path to your chain-spec
```

To build custom chain spec you can use following commands:
```bash
./target/release/qf-parachain-node build-spec \
  --chain local_testnet \
  --disable-default-bootnode \
  > parachain-spec-plain.json
```

configure it if you need and make raw spec:

```bash
./target/release/qf-parachain-node build-spec \
  --chain parachain-spec-plain.json \
  --disable-default-bootnode \
  --raw \
  > parachain-spec-raw.json
```

To start zombienet file you should use following command:
```bash
zombienet --provider native spawn parachain/zombienet.toml
```

### 3. Spin-finality-relayer

Before starting you should configure .env file
Or if you don't use it you can configure parameters at begining of index.ts file.

To start use:
```bash
cd relayer/spin-finality-relayer
npm i
npm start
```

## Possible problems and Errors

1. ParaId in chain spec and zombienet config should be the same
2. It may be error with origins. Relayer sign 3 transactions - on parachain
'set_authority_set', 'submit_finality_proof' and on fastchain 'note_anchor_verified'.
Origins on chains and origins in relayer should match


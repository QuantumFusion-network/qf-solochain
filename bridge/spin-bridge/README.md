# Spin Bridge

TypeScript utility that streams GRANDPA finality proofs from the FastChain node into the
`spin-polkadot` pallet on the parachain. The flow is:

1. connect to both chains (default `ws://127.0.0.1:9944` for FastChain and
   `ws://127.0.0.1:9988` for the parachain);
2. mirror the grandpa authority set on the parachain via `sudo` and the
   `setAuthoritySet` extrinsic whenever `currentSetId` or the validator roster changes;
3. subscribe to `grandpa_subscribeJustifications`, forwarding each SCALE-encoded
   justification to `submitFinalityProof` together with the matching set id.

## Usage

```bash
cd bridge/spin-bridge
pnpm install # or npm install / yarn install
BRIDGE_URI=//Alice pnpm start
```

Environment variables:

- `FASTCHAIN_WS` – FastChain WS endpoint (default `ws://127.0.0.1:9944`).
- `PARACHAIN_WS` – parachain WS endpoint (default `ws://127.0.0.1:9988`).
- `BRIDGE_URI` – signing key for both the sudo authority-set update and
  finality submissions (default `//Alice`).
- `LOG_LEVEL` – pino log level (default `info`).

The script queues submissions sequentially and logs each accepted proof once the
parachain extrinsic is finalized.

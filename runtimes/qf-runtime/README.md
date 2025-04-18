## Summary

The QF Runtime is a high-throughput Substrate-based runtime optimized for 100 ms block production, powered by a custom **SPIN** [^1] leadership protocol that grants a single block-authoring right for a fixed number of slots. It integrates standard FRAME pallets—system, timestamp, balances, staking, session, sudo, transaction payment, GRANDPA finality and custom QF modules for **PolkaVM** smart contracts [^2] and faucet funding. Together, these components provide a complete foundation for validators, smart contracts, and end users to interact seamlessly on QF's solo chain.

## Key Features

### High-Speed Block Production  
- **Slot Duration** is set to 100 ms, enabling sub-second block times.  
  Defined in [`SLOT_DURATION`](https://github.com/QuantumFusion-network/qf-solochain/blob/eb15c7f09221b375c46c54508144d46c45ee6e37/runtimes/qf-runtime/src/lib.rs#L89)

### SPIN Leadership Protocol  
- The **SPIN** mechanism hands off single-leader rights for a [configurable number of blocks](https://github.com/QuantumFusion-network/qf-solochain/blob/eb15c7f09221b375c46c54508144d46c45ee6e37/runtimes/qf-runtime/src/configs/mod.rs#L118), reducing election overhead and ensuring swift block production.[^3]

### Finality & Validator Participation  
- Integrates **pallet_grandpa** for adaptive finality, enabling validators to vote on block finalization.  
  Documentation: [GRANDPA consensus module](https://paritytech.github.io/polkadot-sdk/master/pallet_grandpa/index.html)

## Runtime Configuration

### Constants  
- `SLOT_DURATION`: 100 ms
- SPIN window length: configurable via runtime parameters in `pallet_spin`

### Consensus Hooks  
- `pallet_spin` hooks into block initialization to rotate leadership after the SPIN window expires.  
- `pallet_grandpa` handles justification generation and finality rounds.

## Enabled Pallets

| **Pallet**                                                                                                             | **Purpose**                                                                         |
|------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------|
| [`frame_system`](https://paritytech.github.io/polkadot-sdk/master/frame_system/pallet/struct.Pallet.html)              | Core blockchain primitives (accounts, block number, events). (Substrate)            |
| [`pallet_timestamp`](https://paritytech.github.io/polkadot-sdk/master/pallet_timestamp/index.html)                     | Consensus-based on-chain time source. (Substrate)                                   |
| [`pallet_spin`](https://github.com/QuantumFusion-network/qf-solochain/tree/main/pallets/spin)                          | QF's SPIN leadership protocol for fixed-window block authoring. (custom in QF repo) |
| [`pallet_grandpa`](https://paritytech.github.io/polkadot-sdk/master/pallet_grandpa/index.html)                         | Finality gadget, voting and authority set management. (Substrate)                   |
| [`pallet_balances`](https://paritytech.github.io/polkadot-sdk/master/pallet_balances/index.html)                       | Account balances and transfers. (Substrate)                                         |
| [`pallet_transaction_payment`](https://paritytech.github.io/polkadot-sdk/master/pallet_transaction_payment/index.html) | Fee calculation and weight-based transaction pricing. (Substrate)                   |
| [`pallet_sudo`](https://paritytech.github.io/polkadot-sdk/master/pallet_sudo/index.html)                               | Super-user key for on-chain governance. (Substrate)                                 |
| [`pallet_qf_polkavm`](https://github.com/QuantumFusion-network/qf-solochain/tree/main/pallets/qf-polkavm)              | PolkaVM smart contract execution environment. (custom in QF repo)                   |
| [`pallet_qf_polkavm_dev`](https://github.com/QuantumFusion-network/qf-solochain/tree/main/pallets/qf-polkavm-dev)      | Development tools for PolkaVM. (custom in QF repo)                                  |
| [`pallet_faucet`](https://github.com/QuantumFusion-network/qf-solochain/tree/main/pallets/faucet)                      | DevNet token faucet for easy funding of test accounts. (custom in QF repo)          |
| [`pallet_staking`](https://paritytech.github.io/polkadot-sdk/master/pallet_staking/index.html)                         | Delegated staking and validator selection. (Substrate)                              |
| [`pallet_session`](https://paritytech.github.io/polkadot-sdk/master/pallet_session/index.html)                         | Session key and rotation management for staking. (Substrate)                        |

[^1]: <https://github.com/QuantumFusion-network/spec/tree/main/docs/SPIN> "SPIN docs".
[^2]: <https://github.com/QuantumFusion-network/spec/blob/main/docs/PolkaVM/polkavm_pallet.md> "PolkaVM Pallet docs".
[^3]: <https://x.com/quantumfusion_> "QF announcement".

## Summary

The QF Runtime is a high-throughput Substrate-based runtime optimized for 100 ms block production, powered by a custom PoS-based **SPIN** [^1] leadership protocol that grants a single block-authoring right for a fixed number of slots, standard GRANDPA finality protocol and **PolkaVM** smart contracts [^2].

## Key Features

### SPIN Leadership Protocol
- The slot-based **SPIN** mechanism hands off single-leader rights for a configurable number of slots/blocks (configured via [`DefaultSessionLength`](https://github.com/QuantumFusion-network/qf-solochain/blob/eb15c7f09221b375c46c54508144d46c45ee6e37/runtimes/qf-runtime/src/configs/mod.rs#L114)), reducing election overhead and ensuring swift block production. Validator election is handled by the staking pallet, which defines the active validator set used by SPIN for block author selection.
- **Slot Duration** is set to 100 ms, enabling sub-second block times.
  Defined in [`SLOT_DURATION`](https://github.com/QuantumFusion-network/qf-solochain/blob/eb15c7f09221b375c46c54508144d46c45ee6e37/runtimes/qf-runtime/src/lib.rs#L89)

### PolkaVM smart contracts
- PolkaVM pallet provides a fast, secure RISC-V virtual machine, offering lower execution overhead compared to standard WASM smart contracts [^3]. See [benchmarks](https://github.com/paritytech/polkavm/blob/master/BENCHMARKS.md).

[^1]: <https://github.com/QuantumFusion-network/spec/tree/main/docs/SPIN> "SPIN docs".
[^2]: <https://github.com/QuantumFusion-network/spec/blob/main/docs/PolkaVM/polkavm_pallet.md> "PolkaVM Pallet docs".
[^3]: <https://github.com/paritytech/polkavm> "PolkaVM. A fast and secure RISC-V based virtual machine".
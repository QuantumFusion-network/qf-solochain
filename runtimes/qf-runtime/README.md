# qf-runtime

The QF Runtime is a high-throughput Substrate-based runtime optimized for 100 ms block production, powered by a custom
PoS-based **SPIN** [^1] consensus protocol that grants a single block-authoring right for a fixed sequence of slots and
standard GRANDPA finality gadget. It also contains **PolkaVM** execution environment for smart contracts [^2].

## Key Features

### SPIN Consensus Protocol

The slot-based **SPIN** mechanism hands off single-leader rights for a configurable sequence of slots reducing election
overhead and ensuring swift block production. Validator election is handled by staking-related modules, which defines
the active validator set used by SPIN for block author selection.

#### Configuration parameters

<!-- markdownlint-disable-next-line MD013 -->
- [`DefaultSessionLength`](https://github.com/QuantumFusion-network/qf-solochain/blob/eb15c7f09221b375c46c54508144d46c45ee6e37/runtimes/qf-runtime/src/configs/mod.rs#L114) -
 leader tenure duration in a number of slots.

<!-- markdownlint-disable-next-line MD013 -->
- [`SLOT_DURATION`](https://github.com/QuantumFusion-network/qf-solochain/blob/eb15c7f09221b375c46c54508144d46c45ee6e37/runtimes/qf-runtime/src/lib.rs#L89) -
 period of time for a single block, set to 100 ms enabling sub-second block time.

### PolkaVM smart contracts

PolkaVM pallet provides a fast, secure RISC-V virtual machine, offering lower execution overhead compared to standard
WASM smart contracts [^3]. See [benchmarks](https://github.com/paritytech/polkavm/blob/master/BENCHMARKS.md).

[^1]: <https://github.com/QuantumFusion-network/spec/tree/main/docs/SPIN> "SPIN docs".
[^2]: <https://github.com/QuantumFusion-network/spec/blob/main/docs/PolkaVM/polkavm_pallet.md> "PolkaVM Pallet docs".
[^3]: <https://github.com/paritytech/polkavm> "PolkaVM. A fast and secure RISC-V based virtual machine".

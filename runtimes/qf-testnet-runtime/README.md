# qf-testnet-runtime

Copy of the `qf-runtime` at commit `0e4833aab30257730169d4cdbfeaec4227110f2e` created to simplify runtime preparation
for the mainnet and work on testnet migration after the mainnet launch. Consider returning back to one runtime crate for
the testnet and mainnet in the future.

The QF Network Runtime is a high-throughput Substrate-based runtime optimized for 100 ms block production, powered by a custom
PoS-based **SPIN** [^1] consensus protocol that grants a single block-authoring right for a fixed sequence of slots and
standard GRANDPA finality gadget. It also contains `pallet-revive`[^2] with **PolkaVM**[^3] execution environment for smart
contracts.

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

With `pallet-revive` the blockchain supports native Rust smart contracts (see `qf-polkavm-sdk` [^4]), as well as Solidity
(see Revive compiler [^5]), and ink! v6 (see [^6]).

[^1]: <https://github.com/QuantumFusion-network/spec/tree/main/docs/SPIN> "SPIN docs".
[^2]: <https://github.com/paritytech/polkadot-sdk/tree/polkadot-stable2503-6/substrate/frame/revive> "Revive Pallet".
[^3]: <https://github.com/paritytech/polkavm> "PolkaVM. A fast and secure RISC-V based virtual machine".
[^4]: <https://github.com/QuantumFusion-network/qf-polkavm-sdk> "QF Network PolkaVM SDK".
[^5]: <https://github.com/paritytech/revive> "revive".
[^6]: <https://github.com/use-ink/ink> "ink!".

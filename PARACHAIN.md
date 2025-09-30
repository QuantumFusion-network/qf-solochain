# Parachain Template Delta

- `parachain/node/Cargo.toml:2` renames the node crate to `qf-parachain-node` and updates the crate description to "QF Parachain Client Node", diverging from the stock `parachain-template-node` branding.
- `parachain/node/src/command.rs:31` and `parachain/node/src/command.rs:67` change the CLI-facing implementation name to "QF Network Bridging Gadget" instead of the template's "Parachain Collator Template".
- `parachain/node/src/command.rs:53` and `parachain/node/src/command.rs:89` point support requests to `https://github.com/QuantumFusion-network/qf-solochain/issues/new` and set the copyright start year to 2024, replacing the template's Parity URL and 2020 baseline.
- Checked runtime, service, chain-spec, and XCM configuration sources against the Polkadot SDK template commit recorded in `parachain/node/README.md` and found no functional deviations; configuration, consensus, and pallet wiring remain identical to the upstream parachain template.

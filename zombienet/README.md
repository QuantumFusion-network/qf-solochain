# Zombienet

Zombienet is a CLI tool to easily spawn ephemeral Polkadot/Substrate networks and perform tests against them.
See <https://github.com/paritytech/zombienet> for installation and usage guide.

## Run an ephemeral network for SPIN development

This starts an ephemeral network with 5 archive nodes, 3 of which are genesis validators.

```console
zombienet -p native spawn zombienet/spin/spin.toml
```

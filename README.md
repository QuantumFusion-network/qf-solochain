<div align="center">

![Logo](Logo.jpg)

# Quantum Fusion

[![License](https://img.shields.io/github/license/QuantumFusion-network/qf-solochain?color=green)](https://github.com/QuantumFusion-network/qf-solochain/blob/main/LICENSE)
<br>
![GitHub contributors](https://img.shields.io/github/contributors/QuantumFusion-network/qf-solochain)
![GitHub commit activity](https://img.shields.io/github/commit-activity/m/QuantumFusion-network/qf-solochain)
![GitHub last commit](https://img.shields.io/github/last-commit/QuantumFusion-network/qf-solochain)
<br>
[![Twitter URL](https://img.shields.io/twitter/follow/theqfnetwork?style=social)](https://x.com/theqfnetwork)

</div>

For contributing to this project, please read [Contributing](#contributing) section.

## Building From Source

> This section assumes that the developer is running on either macOS or Debian-variant operating system. For Windows,
although there are ways to run it, we recommend using [WSL](https://learn.microsoft.com/en-us/windows/wsl/install)
or a virtual machine.

1. Install Polkadot SDK dependencies following https://docs.polkadot.com/develop/parachains/install-polkadot-sdk/.

2. Clone the repository and build the node binary.

    ```console
    git clone --recursive https://github.com/QuantumFusion-network/qf-solochain.git
    cd qf-solochain
    cargo build --release
    ```

3. Inspect available subcommands.

    ```console
    ./target/release/qf-node --help
    ```

4. Run a local node in dev mode.

    ```bash
    make qf-run
    ```

## Executables and runtimes

This section describes the project's executables and runtimes and provides step-by-step instructions
 for running a local testnet. This guide is suitable for advanced users.
See [docs/executables_and_runtimes.md](docs/executables_and_runtimes.md).

## Run the Full Node

- To build and run a full node in a container please read instruction [docker/README.md](docker/README.md)
- Please notice a full node does register itself at [https://telemetry.qfnetwork.xyz/](https://telemetry.qfnetwork.xyz/).

## Testing with Zombienet

See [zombienet/README.md](zombienet/README.md).

## Makefile commands

- Build the node: `make qf-node`
- Build the release node: `make qf-node-release`
- Build the node and run it: `make qf-run`
- Build the node and run it with wasm file from `output`: `make qf-run-wasm`
- Build the runtime: `make qf-runtime`
- Linting: `make clippy`
- Formatting: `make fmt`
- Run tests: `make qf-test`
- Check all: `make check`
- Make chain spec: `make qf-chainspec`

## Contributing

We welcome contributions of all kinds! Whether you're reporting or fixing a bug, adding a feature, or improving
documentation, your help is greatly appreciated. For a bug or vulnerability report please [open a new issue](https://github.com/QuantumFusion-network/qf-solochain/issues/new).

For code contributions please follow these steps:

1. Fork the repository and create a new branch following the format `your-github-name/descriptive-branch-name` (e.g., `alice/fix-123`).
2. Make smaller commits with clear messages to simplify reviewer's work.
3. Submit a pull request targeting `main` branch and provide a concise description of your changes.

By contributing, you agree to adhere to our [Contributor Covenant Code of Conduct](./CODE_OF_CONDUCT.md), which fosters
a respectful and inclusive environment.

We appreciate your support and look forward to your contributions! 🚀

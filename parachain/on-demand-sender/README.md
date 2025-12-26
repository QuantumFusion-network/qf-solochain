# on-demand-sender

Sends coretime order each relay chain block.

## Usage

1. Set configuration in .env (see .env.example for configuration details).

    ```console
    cp .env.example .env
    nano .env
    ```

1. Start the app.

    ```console
    nvm use
    node index.js
    ```

1. Or in a container.

    1. Build an image.

        ```console
        docker build --build-arg NODE_VERSION=$(cat .nvmrc) -t qfnetwork/on-demand-sender:local .
        ```

    1. Launch the container.

        ```console
        docker run --rm -it --env-file .env.example --network host qfnetwork/on-demand-sender:local
        ```

## Deployment notes

- Stateless and doesn't listen for incoming connections.
- Connects to an RPC endpoint and needs tx sender's account mnemonic (see `.env.example`).

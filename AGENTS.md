## Project Structure & Module Organization
- `node/` is the Substrate entrypoint; `output/` collects staged binaries after make targets run.
- `runtimes/` defines FastChain and AnchorChain logic; Wasm artifacts land under `target/release/wbuild/`.
- `pallets/` hosts pallets such as `spin-anchoring` (SecureUpTo); shared primitives sit in `primitives/`.
- `client/consensus-spin` plus `primitives/consensus-spin` implement SPIN; `docs/`, `docker/`, and `zombienet/` add ops tooling.

## Build, Test, and Development Commands
- `make qf-node` / `make qf-node-release` compile debug or release nodes and copy binaries into `output/`.
- `make qf-runtime` refreshes runtime Wasm artifacts before packaging, anchoring, or chain-spec work.
- `make qf-run` launches a dev FastChain with tmp state; `make qf-run-wasm` loads runtime overrides from `output/`.
- `make clippy`, `make fmt`, and `make qf-test` wrap `cargo` lint, format (+nightly), and test with `SKIP_WASM_BUILD`.
- Run `taplo format` (and `taplo format --check`) whenever you touch `.toml` files.
- Run `cargo +nightly fmt --all` before committing to keep formatting consistent.
- Run `cargo check`, `cargo clippy --all-targets`, and `cargo +nightly fmt -- --check` before review.
- Run `cargo test --workspace --no-fail-fast` before review.
- Run `markdownlint-cli2 "**/*.md" "#target" --config .markdownlint.yaml --fix` when editing Markdown.

## Coding Style & Naming Conventions
- Use Rust defaults: 4-space indentation, snake_case modules/functions, UpperCamelCase types, SCREAMING_SNAKE constants.
- Prefix new crates with `qf-`; place consensus additions under `client/` or `primitives/` to match the layout.
- Run `make fmt` and `make clippy`; avoid `unwrap` in runtime/consensus code—prefer structured errors.

## Testing Guidelines
- Run `make qf-test` for the suite; co-locate integration cases in each crate’s `tests/` folder.
- Model FastChain vs AnchorChain in `zombienet/`; log best/finalized/secure_up_to metrics.
- When touching SecureUpTo or authority transitions, add negative, reorg, and weight-edge cases.

## Commit & Pull Request Guidelines
- Keep commit subjects imperative (e.g., `Add spin-anchoring weight checks`) and wrap bodies near 72 characters.
- Branch names follow `your-handle/summary`; rebase onto `main` before submitting PRs.
- PR descriptions should include motivation, tests (`make qf-test`, zombienet), linked issues/specs, and telemetry/log artefacts.

## Architecture & Security Notes
- SPIN pairs FastChain liveness with AnchorChain finality; keep SecureUpTo monotonic and authenticated.
- Regenerate chain specs with `make qf-chainspec` after runtime updates and commit the JSON alongside refreshed Wasm.
- Treat heights beyond SecureUpTo as probabilistic; never publish runtime keys; follow `SECURITY.md` for disclosure.
